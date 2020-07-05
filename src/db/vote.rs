use super::{alternative::AlternativeId, user::UserId};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct VoteId(pub Uuid);

impl VoteId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InternalVote {
    pub id: VoteId,
    pub alternative_id: AlternativeId,
    pub user_id: UserId,
}
