use super::DbExecutor;
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
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_string(&self) -> String {
        self.0.to_hyphenated().to_string()
    }
}

#[derive(Clone, Debug)]
pub struct InternalUser {
    pub id: i32,
    pub uuid: UserId,
    pub username: String,
}

// Find user

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalUser>, Report>")]
pub struct UserByUsername(pub String);

message_handler_with_span! {
    impl SpanHandler<UserByUsername> for DbExecutor {
        type Result = ResponseActFuture<Self, <UserByUsername as Message>::Result>;

        fn handle(&mut self, msg: UserByUsername, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            debug!("Handling connect");
            async {
                let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
                let username = msg.0;
                let user = sqlx::query_as!(InternalUser,
                    r#"
                    SELECT id, uuid as "uuid: _", username FROM users WHERE username = $1
                    "#, username
                ).fetch_optional(&pool).await?;


                Ok(user)
            }.interop_actor_boxed(self)
        }
    }
}

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalUser>, Report>")]
pub struct UserById(pub UserId);

message_handler_with_span! {
    impl SpanHandler<UserById> for DbExecutor {
        type Result = ResponseActFuture<Self, <UserById as Message>::Result>;

        fn handle(&mut self, msg: UserById, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            debug!("Handling connect");
            async {
                let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
                let user_id = msg.0;
                let uuid = user_id.0;
                let user = sqlx::query_as!(InternalUser,
                    r#"
                    SELECT id, uuid as "uuid: _", username FROM users WHERE uuid = $1
                    "#, uuid
                ).fetch_optional(&pool).await?;

                Ok(user)
            }.interop_actor_boxed(self)
        }
    }
}
