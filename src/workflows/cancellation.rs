use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
)]
pub enum CancellationStrategy {
    Ignore,
    Cascade,
    Immediate,
}
