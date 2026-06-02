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
pub struct ExecutionIdentity {
    pub execution_id:
        Uuid,

    pub actor_id: String,

    pub roles: Vec<String>,
}
