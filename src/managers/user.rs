use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, span, Level};
use uuid::Uuid;

#[derive(Clone, Hash, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct UserId(pub String);
impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_hyphenated().to_string())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct InternalUser {
    pub id: UserId,
    pub username: String,
}

pub struct UserManager {
    users: HashMap<UserId, InternalUser>,
}

impl UserManager {
    pub fn find_by_id(&self, user_id: UserId) -> Option<InternalUser> {
        self.users.get(&user_id).cloned()
    }

    pub fn find_by_username(&self, username: String) -> Option<InternalUser> {
        let span = span!(
            Level::TRACE,
            "find by username",
            username = username.as_str()
        );
        let _enter = span.enter();
        for (_user_id, user) in self.users.iter() {
            if user.username == username {
                debug!("Found user");
                return Some(user.clone());
            }
        }
        None
    }
}

impl Default for UserManager {
    fn default() -> Self {
        let user = InternalUser {
            id: UserId::new(),
            username: "user".to_owned(),
        };
        let admin = InternalUser {
            id: UserId::new(),
            username: "admin".to_owned(),
        };
        let users = [(user.id.clone(), user), (admin.id.clone(), admin)]
            .iter()
            .cloned()
            .collect();
        Self { users }
    }
}
