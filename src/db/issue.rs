use super::DbExecutor;
use crate::async_message_handler_with_span;
use crate::span::AsyncSpanHandler;
use actix::prelude::*;
use actix_interop::with_ctx;
use color_eyre::eyre::Report;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use tracing::debug;

#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct IssueId(pub Uuid);

impl IssueId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
#[derive(Clone, PartialEq, Debug, sqlx::Type)]
#[sqlx(rename = "text")]
#[sqlx(rename_all = "snake_case")]
pub enum InternalIssueState {
    NotStarted,
    InProgress,
    Finished,
}

#[derive(Clone)]
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
                    SELECT id as "id: _", title, description, state as "state: _", max_voters, show_distribution FROM issues WHERE id = $1
                    "#, uuid
                ).fetch_optional(&pool).await?;

            Ok(user)
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
            let user = sqlx::query_as!(InternalIssue,
                    r#"
                    SELECT id as "id: _", title, description, state as "state: _", max_voters, show_distribution FROM issues
                    "#
                ).fetch_optional(&pool).await?;

            Ok(user)
        }
    }
});
