use super::alternative::AlternativeId;
use crate::db::user::UserId;
use uuid::Uuid;

// Types

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoteId(pub String);

impl VoteId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_hyphenated().to_string())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InternalVote {
    pub id: VoteId,
    pub alternative_id: AlternativeId,
    pub user_id: UserId,
}
