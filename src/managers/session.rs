use crate::db::user::UserId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SessionId(pub String);
impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_hyphenated().to_string())
    }
}

#[derive(Clone, Debug)]
pub struct InternalSession {
    pub id: SessionId,
    pub user_id: UserId,
}

pub struct SessionManager {
    sessions: HashMap<SessionId, InternalSession>,
}

impl SessionManager {
    pub fn find_by_id(&self, session_id: SessionId) -> Option<InternalSession> {
        self.sessions.get(&session_id).cloned()
    }

    pub fn insert(&mut self, session: InternalSession) {
        self.sessions.insert(session.id.clone(), session);
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }
}
