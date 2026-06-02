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
pub struct TransportSecurity {
    pub tls_enabled:
        bool,

    pub mutual_tls:
        bool,

    pub token_auth:
        bool,
}
