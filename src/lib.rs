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

// ── Flutter-Rust Bridge ───────────────────────────────────
// Mobil FFI katmanı. flutter_rust_bridge_codegen bu modülü
// tarayarak flutter_app/lib/src/rust/ altına Dart dosyaları üretir.
// Sunucu build'inde de derlenir; binary boyutuna etkisi minimumdur.
pub mod bridge;

pub mod frb_generated; 

pub use runtime::api::RuntimeHandle;
pub use runtime::bootstrap::RuntimeBootstrap;
pub use runtime::config::RuntimeConfig;
pub use runtime::runtime::Runtime;

#[cfg(test)]
mod tests;
