use super::{user::UserId, DbExecutor};
use crate::message_handler_with_span;
use crate::span::SpanHandler;
use actix::prelude::*;
use actix_interop::{with_ctx, FutureInterop};
use color_eyre::eyre::Report;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use tracing::{debug, Span};

#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_string(&self) -> String {
        self.0.to_hyphenated().to_string()
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct InternalSession {
    pub id: SessionId,
    pub user_id: UserId,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalSession>, Report>")]
pub struct SessionById(pub SessionId);

message_handler_with_span! {
    impl SpanHandler<SessionById> for DbExecutor {
        type Result = ResponseActFuture<Self, <SessionById as Message>::Result>;

        fn handle(&mut self, msg: SessionById, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            async {
                let SessionById(session_id) = msg;
                debug!(id = session_id.as_string().as_str(), "Get session by id");
                let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
                let user = sqlx::query_as!(InternalSession,
                    r#"
                    SELECT id as "id: _", user_id as "user_id: _" FROM sessions WHERE id = $1
                    "#, session_id.0
                ).fetch_optional(&pool).await?;

                Ok(user)
            }.interop_actor_boxed(self)
        }
    }
}

#[derive(Message, Clone)]
#[rtype(result = "Result<InternalSession, Report>")]
pub struct SaveSession(pub UserId);

message_handler_with_span! {
impl SpanHandler<SaveSession> for DbExecutor {
    type Result = ResponseActFuture<Self, <SaveSession as Message>::Result>;

    fn handle(&mut self, msg: SaveSession, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
        async {
            let SaveSession(user_id) = msg;
            debug!(
                user_id = user_id.as_string().as_str(),
                "Save new session for user"
            );
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            let user = sqlx::query_as!(
                InternalSession,
                r#"
                    INSERT INTO sessions (user_id) VALUES($1)
                    RETURNING id as "id: _", user_id as "user_id: _"
                    "#,
                user_id.0
            )
            .fetch_one(&pool)
            .await?;

            Ok(user)
        }
        .interop_actor_boxed(self)
    }
}
}
