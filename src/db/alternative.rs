use super::{issue::IssueId, DbExecutor};
use crate::{async_message_handler_with_span, span::AsyncSpanHandler};
use actix::prelude::*;
use actix_interop::with_ctx;
use color_eyre::eyre::Report;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use tracing::{debug, instrument};

#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct AlternativeId(pub Uuid);

impl AlternativeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Clone, Debug)]
pub struct InternalAlternative {
    pub id: AlternativeId,
    pub title: String,
    pub issue_id: IssueId,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "Result<Vec<InternalAlternative>, Report>")]
pub struct AlternativesForIssueId(pub IssueId);

async_message_handler_with_span! {
    impl AsyncSpanHandler<AlternativesForIssueId> for DbExecutor {
        #[instrument]
        async fn handle(msg: AlternativesForIssueId) -> Result<Vec<InternalAlternative>, Report> {
            let pool = with_ctx(|a: &mut DbExecutor, _| a.pool());
            let alternative_id = msg.0;
            let uuid = alternative_id.0;
            debug!("Retrieving alternative by id {id}", id = uuid);
            let user = sqlx::query_as!(InternalAlternative,
                                r#"
                                SELECT id as "id: _", title, issue_id as "issue_id: _" FROM alternatives WHERE issue_id = $1
                                "#, uuid
                            ).fetch_all(&pool).await?;
            Ok(user)
        }
    }
}
