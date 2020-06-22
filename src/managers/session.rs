use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SessionId(pub String);
impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_hyphenated().to_string())
    }
}
