use super::{alternative::AlternativeId, issue::IssueId, user::UserId, DbExecutor};
use crate::span::AsyncSpanHandler;
use actix::prelude::*;
use actix_interop::with_ctx;
use color_eyre::eyre::{eyre, Report, WrapErr};
use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, Executor, Postgres};
use tracing::{debug, instrument};

#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct VoteId(pub Uuid);

impl VoteId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for VoteId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InternalVote {
    pub id: VoteId,
    pub alternative_id: AlternativeId,
    pub issue_id: IssueId,
    pub user_id: UserId,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "Result<InternalVote, Report>")]
pub struct AddVote(pub UserId, pub IssueId, pub AlternativeId);

async fn get_vote_for_user(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: UserId,
    issue_id: IssueId,
) -> Result<Option<InternalVote>, Report> {
    sqlx::query_as!(
        InternalVote,
        r#"
        SELECT
            id as "id: _",
            alternative_id as "alternative_id: _",
            issue_id as "issue_id: _",
            user_id as "user_id: _"
        FROM votes
        WHERE user_id= $1 AND issue_id = $2
        "#,
        user_id.0,
        issue_id.0,
    )
    .fetch_optional(executor)
    .await
    .wrap_err("Got error while retrieving vote for user")
}

async fn get_votes_for_issue(
    executor: impl Executor<'_, Database = Postgres>,
    issue_id: IssueId,
) -> Result<Vec<InternalVote>, Report> {
    sqlx::query_as!(
        InternalVote,
        r#"
        SELECT
            id as "id: _",
            alternative_id as "alternative_id: _",
            issue_id as "issue_id: _",
            user_id as "user_id: _"
        FROM votes
        WHERE issue_id = $1
        "#,
        issue_id.0,
    )
    .fetch_all(executor)
    .await
    .wrap_err("Got error while retrieving votes for issue")
}

async fn insert_vote(
    executor: impl Executor<'_, Database = Postgres>,
    alternative_id: AlternativeId,
    issue_id: IssueId,
    user_id: UserId,
) -> Result<InternalVote, Report> {
    sqlx::query_as!(
        InternalVote,
        r#"
        INSERT INTO votes (alternative_id, issue_id, user_id) VALUES($1, $2, $3)
        RETURNING id as "id: _", alternative_id as "alternative_id: _", issue_id as "issue_id: _", user_id as "user_id: _"
        "#,
        alternative_id.0,
        issue_id.0,
        user_id.0,
    )
    .fetch_one(executor)
    .await
    .wrap_err("Got error while adding vote to DB")
}

#[async_trait::async_trait]
impl AsyncSpanHandler<AddVote> for DbExecutor {
    #[instrument]
    async fn handle(msg: AddVote) -> Result<InternalVote, Report> {
        debug!(vote = ?msg, "Adding vote");
        let AddVote(user_id, issue_id, alternative_id) = msg;

        let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
        let mut tx = pool.begin().await?;

        let user_vote = get_vote_for_user(&mut tx, user_id.clone(), issue_id.clone()).await?;
        if user_vote.is_some() {
            return Err(eyre!("User has already voted"));
        }
        let inserted_vote = insert_vote(&mut tx, alternative_id, issue_id, user_id).await?;

        tx.commit().await?;

        Ok(inserted_vote)
    }
}
crate::span_message_async_impl!(AddVote, DbExecutor);

#[derive(Message, Clone, Debug)]
#[rtype(result = "Result<Vec<InternalVote>, Report>")]
pub struct VotesForIssue(pub IssueId);

#[async_trait::async_trait]
impl AsyncSpanHandler<VotesForIssue> for DbExecutor {
    #[instrument]
    async fn handle(msg: VotesForIssue) -> Result<Vec<InternalVote>, Report> {
        debug!("Retrieving votes for issue");
        let VotesForIssue(issue_id) = msg;

        let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
        get_votes_for_issue(&pool, issue_id).await
    }
}
crate::span_message_async_impl!(VotesForIssue, DbExecutor);
