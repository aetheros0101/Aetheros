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
pub struct AuthenticationToken {
    pub access_token:
        String,

    pub expires_in:
        usize,
}
