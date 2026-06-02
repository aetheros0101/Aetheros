pub mod errors;
pub mod events;
pub mod metrics;
pub mod persistence;
pub mod registry;
pub mod runtime;
pub mod task;
pub mod types;
pub mod wasm;
pub mod worker;

pub mod config;
pub mod api;
pub mod plugins;
pub mod remote;
pub mod sdk;
pub mod security;


pub mod agents;
pub mod ai;
pub mod orchestration;
pub mod workflows;
pub mod logging;
pub mod scripting;

pub use runtime::api::RuntimeHandle;
pub use runtime::bootstrap::RuntimeBootstrap;
pub use runtime::config::RuntimeConfig;
pub use runtime::runtime::Runtime;

#[cfg(test)]
mod tests;
