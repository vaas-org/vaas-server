use super::DbExecutor;
use crate::message_handler_with_span;
use crate::span::SpanHandler;
use actix::prelude::*;
use actix_interop::{with_ctx, FutureInterop};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use tracing::{debug, Span};

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
#[rtype(result = "Result<Option<InternalIssue>, &'static str>")]
pub struct IssueById(IssueId);

message_handler_with_span! {
    impl SpanHandler<IssueById> for DbExecutor {
        type Result = ResponseActFuture<Self, <IssueById as Message>::Result>;

        fn handle(&mut self, msg: IssueById, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            async {
                let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
                let issue_id = msg.0;
                let uuid = issue_id.0;
                debug!("Retrieving issue by id {id}", id = uuid);
                let user = sqlx::query_as!(InternalIssue,
                    r#"
                    SELECT id as "id: _", title, description, state as "state: _", max_voters, show_distribution FROM issues WHERE id = $1
                    "#, uuid
                ).fetch_optional(&pool).await.unwrap();

                Ok(user)
            }.interop_actor_boxed(self)
        }
    }
}

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalIssue>, &'static str>")]
pub struct ActiveIssue();

message_handler_with_span! {
    impl SpanHandler<ActiveIssue> for DbExecutor {
        type Result = ResponseActFuture<Self, <ActiveIssue as Message>::Result>;

        fn handle(&mut self, _msg: ActiveIssue, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            async {
                let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
                debug!("Retrieving active issue");
                let user = sqlx::query_as!(InternalIssue,
                    r#"
                    SELECT id as "id: _", title, description, state as "state: _", max_voters, show_distribution FROM issues
                    "#
                ).fetch_optional(&pool).await.unwrap();

                Ok(user)
            }.interop_actor_boxed(self)
        }
    }
}
