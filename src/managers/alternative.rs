use uuid::Uuid;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct AlternativeId(pub String);
impl AlternativeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_hyphenated().to_string())
    }
}
