use super::DbExecutor;
use crate::{span::AsyncSpanHandler, span_message_async_impl};
use actix::prelude::*;
use actix_interop::with_ctx;
use color_eyre::eyre::Report;
use color_eyre::eyre::WrapErr;
use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, Executor, Postgres};
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

#[async_trait::async_trait]
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
span_message_async_impl!(UserByUsername, DbExecutor);

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalUser>, Report>")]
pub struct UserById(pub UserId);

#[async_trait::async_trait]
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
span_message_async_impl!(UserById, DbExecutor);

#[derive(Clone, Debug)]
pub struct NewInternalUser {
    pub username: String,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<InternalUser, Report>")]
pub struct NewUser(pub NewInternalUser);

async fn insert_new_user(
    executor: impl Executor<'_, Database = Postgres>,
    data: NewInternalUser,
) -> Result<InternalUser, Report> {
    sqlx::query_as!(
        InternalUser,
        r#"
        INSERT INTO users ( username )
        VALUES ( $1 )
        RETURNING
            id as "id: _",
            username as "username: _"
        "#,
        data.username,
    )
    .fetch_one(executor)
    .await
    .wrap_err("Got error while adding new issue to db")
}

#[async_trait::async_trait]
impl AsyncSpanHandler<NewUser> for DbExecutor {
    async fn handle(msg: NewUser) -> Result<InternalUser, Report> {
        debug!("Handling connect");
        let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());

        insert_new_user(&pool, msg.0).await
    }
}
span_message_async_impl!(NewUser, DbExecutor);
