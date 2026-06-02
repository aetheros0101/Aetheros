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
pub struct RoutingPolicy {
    pub prefer_local:
        bool,

    pub allow_remote:
        bool,

    pub max_cost:
        f32,
}
