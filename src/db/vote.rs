use super::{alternative::AlternativeId, user::UserId, DbExecutor};
use crate::async_message_handler_with_span;
use crate::span::AsyncSpanHandler;
use actix::prelude::*;
use actix_interop::with_ctx;
use color_eyre::eyre::{Report, WrapErr};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
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
    pub user_id: UserId,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "Result<InternalVote, Report>")]
pub struct AddVote(pub UserId, pub AlternativeId);

async_message_handler_with_span!({
    impl AsyncSpanHandler<AddVote> for DbExecutor {
        #[instrument]
        async fn handle(msg: AddVote) -> Result<InternalVote, Report> {
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            debug!(vote = ?msg, "Adding vote");
            let AddVote(user_id, alternative_id) = msg;
            let vote = sqlx::query_as!(
                    InternalVote,
                    r#"
                    INSERT INTO votes (alternative_id, user_id) VALUES($1, $2)
                    RETURNING id as "id: _", alternative_id as "alternative_id: _", user_id as "user_id: _"
                    "#,
                    alternative_id.0,
                    user_id.0,
                )
                .fetch_one(&pool)
                .await
                .wrap_err("Got error while adding vote to DB")?;
            Ok(vote)
        }
    }
});
