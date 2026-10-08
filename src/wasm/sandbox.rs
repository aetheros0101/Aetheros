use std::time::Duration;

use wasmtime::{Config, Engine};

pub fn create_engine() -> Result<Engine, wasmtime::Error> {
    let mut config = Config::new();

    config.async_support(true);

    config.consume_fuel(true);

    config.epoch_interruption(true);

    config.cranelift_opt_level(wasmtime::OptLevel::Speed);

    config.wasm_multi_memory(true);

    config.wasm_reference_types(true);

    Engine::new(&config)
}

pub struct SandboxLimits {
    pub memory_limit_bytes: usize,
    pub execution_timeout: Duration,
    pub fuel_limit: u64,
}
