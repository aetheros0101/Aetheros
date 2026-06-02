pub mod window;
pub mod session;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct AiContext {
    pub session_id:
        String,

    pub correlation_id:
        String,
}
