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
pub struct IsolationBoundary {
    pub namespace:
        String,

    pub memory_limit_mb:
        usize,
}
