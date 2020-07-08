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
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_string(&self) -> String {
        self.0.to_hyphenated().to_string()
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct InternalUser {
    pub id: UserId,
    pub username: String,
}

// Find user

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalUser>, Report>")]
pub struct UserByUsername(pub String);

async_message_handler_with_span!({
    impl AsyncSpanHandler<UserByUsername> for DbExecutor {
        async fn handle(msg: UserByUsername) -> Result<Option<InternalUser>, Report> {
            debug!("Handling connect");
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            let username = msg.0;
            let user = sqlx::query_as!(
                InternalUser,
                r#"SELECT id as "id: _", username FROM users WHERE username = $1"#,
                username
            )
            .fetch_optional(&pool)
            .await?;

            Ok(user)
        }
    }
});

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalUser>, Report>")]
pub struct UserById(pub UserId);

async_message_handler_with_span!({
    impl AsyncSpanHandler<UserById> for DbExecutor {
        async fn handle(msg: UserById) -> Result<Option<InternalUser>, Report> {
            debug!("Handling connect");
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            let user_id = msg.0;
            let uuid = user_id.0;
            let user = sqlx::query_as!(
                InternalUser,
                r#"SELECT id as "id: _", username FROM users WHERE id = $1"#,
                uuid
            )
            .fetch_optional(&pool)
            .await?;

            Ok(user)
        }
    }
});
