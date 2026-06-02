use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct ConversationSession {
    pub session_id:
        Uuid,

    pub messages:
        Vec<String>,
}
