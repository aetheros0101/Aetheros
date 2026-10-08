pub mod audit;
pub mod buffer;
pub mod metrics;

use tracing_subscriber::{EnvFilter, fmt};

pub struct TelemetrySystem;

impl TelemetrySystem {
    pub fn init() {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let result = fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .try_init();

        if result.is_ok() {
            ::tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                "AetherOS telemetry initialized"
            );
        }
    }

    pub fn init_json() {
        // JSON feature gerektirir — şimdilik init() ile aynı
        Self::init();
    }
}
