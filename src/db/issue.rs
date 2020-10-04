use super::{alternative::InternalAlternative, DbExecutor};
use crate::async_message_handler_with_span;
use crate::span::AsyncSpanHandler;
use crate::websocket::{Alternative, Issue, IssueState};
use actix::prelude::*;
use actix_interop::with_ctx;
use color_eyre::eyre::{Report, WrapErr};
use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, Executor, Postgres};
use tracing::{debug, instrument};

#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct IssueId(pub Uuid);

impl IssueId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for IssueId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, PartialEq, Debug, sqlx::Type)]
#[sqlx(rename = "text")]
#[sqlx(rename_all = "snake_case")]
pub enum InternalIssueState {
    NotStarted,
    InProgress,
    VotingFinished,
    Finished,
}

impl InternalIssueState {
    pub fn to_websocket(self) -> IssueState {
        return match self {
            InternalIssueState::NotStarted => crate::websocket::IssueState::NotStarted,
            InternalIssueState::InProgress => crate::websocket::IssueState::InProgress,
            InternalIssueState::VotingFinished => crate::websocket::IssueState::VotingFinished,
            InternalIssueState::Finished => crate::websocket::IssueState::Finished,
        };
    }

    fn to_db(self) -> &'static str {
        return match self {
            InternalIssueState::NotStarted => "not_started",
            InternalIssueState::InProgress => "in_progress",
            InternalIssueState::VotingFinished => "voting_finished",
            InternalIssueState::Finished => "finished",
        };
    }
}

#[derive(Clone, Debug)]
pub struct InternalIssue {
    pub id: IssueId,
    pub title: String,
    pub description: String,
    pub state: InternalIssueState,
    pub max_voters: i32,
    pub show_distribution: bool,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalIssue>, Report>")]
pub struct IssueById(IssueId);

async_message_handler_with_span!({
    impl AsyncSpanHandler<IssueById> for DbExecutor {
        async fn handle(msg: IssueById) -> Result<Option<InternalIssue>, Report> {
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            let issue_id = msg.0;
            let uuid = issue_id.0;
            debug!("Retrieving issue by id {id}", id = uuid);
            let user = sqlx::query_as!(InternalIssue,
                    r#"
                    SELECT id as "id: _", title, description, state as "state: _", max_voters, show_distribution
                    FROM issues WHERE id = $1
                    "#, uuid
                ).fetch_optional(&pool).await?;

            Ok(user)
        }
    }
});

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<Vec<InternalIssue>>, Report>")]
pub struct ListAllIssues;

async_message_handler_with_span!({
    impl AsyncSpanHandler<ListAllIssues> for DbExecutor {
        async fn handle(_msg: ListAllIssues) -> Result<Option<Vec<InternalIssue>>, Report> {
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            debug!("Retrieving all issues");

            let result = sqlx::query_as!(
                InternalIssue,
                r#"
                SELECT id as "id: _", title, description, state as "state: _", max_voters, show_distribution
                FROM issues
                "#
            )
            .fetch_all(&pool)
            .await?;

            Ok(Some(result))
        }
    }
});

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalIssue>, Report>")]
pub struct ActiveIssue();

async_message_handler_with_span!({
    impl AsyncSpanHandler<ActiveIssue> for DbExecutor {
        async fn handle(_msg: ActiveIssue) -> Result<Option<InternalIssue>, Report> {
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            debug!("Retrieving active issue");
            let user = sqlx::query_as!(
                    InternalIssue,
                    r#"
                    SELECT id as "id: _", title, description, state as "state: _", max_voters, show_distribution
                    FROM issues
                    "#
                )
                .fetch_optional(&pool)
                .await?;
            Ok(user)
        }
    }
});

#[derive(Message, Clone, Debug)]
#[rtype(result = "Result<Option<InternalIssue>, Report>")]
pub struct NewIssue(pub Issue);

async fn insert_issue(
    executor: impl Executor<'_, Database = Postgres>,
    data: &Issue,
) -> Result<InternalIssue, Report> {
    let issue_state = match &data.state {
        Some(state) => state.to_owned().to_internal(),
        None => InternalIssueState::NotStarted,
    };

    // @ToDo: Get available voters
    let max_voters = match data.max_voters {
        Some(num) => num,
        None => 1000,
    };

    sqlx::query_as!(
        InternalIssue,
        r#"
                INSERT INTO issues ( title, description, state, max_voters, show_distribution )
                VALUES ( $1, $2, $3, $4, $5 )
                RETURNING
                    id as "id: _",
                    title as "title: _",
                    description as "description: _",
                    state as "state: _",
                    max_voters as "max_voters: _",
                    show_distribution as "show_distribution: _"
                "#,
        data.title,
        data.description,
        issue_state.to_db(), // why doesnt sqlx fix this :thinking:
        max_voters,
        data.show_distribution
    )
    .fetch_one(executor)
    .await
    .wrap_err("Got error while adding new issue to db")
}

async fn insert_alternative(
    executor: impl Executor<'_, Database = Postgres>,
    data: &Alternative,
    issue: &InternalIssue,
) -> Result<InternalAlternative, Report> {
    sqlx::query_as!(
        InternalAlternative,
        r#"
                INSERT INTO alternatives ( issue_id, title )
                VALUES ( $1, $2 )
                RETURNING
                    id as "id: _",
                    issue_id as "issue_id: _",
                    title as "title: _"
            "#,
        issue.id.0,
        data.title
    )
    .fetch_one(executor)
    .await
    .wrap_err("Got error while adding new alternative to db")
}

#[async_trait::async_trait]
impl AsyncSpanHandler<NewIssue> for DbExecutor {
    #[instrument]
    async fn handle(_msg: NewIssue) -> Result<Option<InternalIssue>, Report> {
        let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
        debug!("Creating new issue in db");

        let mut tx = pool.begin().await?;

        let i = insert_issue(&mut tx, &_msg.0).await?;

        for alt in _msg.0.alternatives.iter() {
            insert_alternative(&mut tx, alt, &i).await?;
        }

        tx.commit().await?;
        Ok(Some(i))
    }
}
crate::span_message_async_impl!(NewIssue, DbExecutor);

#[derive(Message, Clone, Debug)]
#[rtype(result = "Result<InternalIssue, Report>")]
pub struct DbSetIssueState {
    pub issue_id: IssueId,
    pub state: InternalIssueState,
}

#[async_trait::async_trait]
impl AsyncSpanHandler<DbSetIssueState> for DbExecutor {
    #[instrument]
    async fn handle(_msg: DbSetIssueState) -> Result<InternalIssue, Report> {
        let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
        debug!("Setting issue state");

        let issue_id: IssueId = _msg.issue_id;

        let result = sqlx::query_as!(
            InternalIssue,
            r#"
            UPDATE
                issues
            SET
                state = $1
            WHERE
                id = $2
            RETURNING
                id as "id: _",
                title as "title: _",
                description as "description: _",
                state as "state: _",
                max_voters as "max_voters: _",
                show_distribution as "show_distribution: _"
            "#,
            _msg.state.to_db(),
            issue_id.0,
        )
        .fetch_one(&pool)
        .await?;

        Ok(result)
    }
}
crate::span_message_async_impl!(DbSetIssueState, DbExecutor);
