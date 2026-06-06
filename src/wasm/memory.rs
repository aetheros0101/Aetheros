#![cfg(feature = "backend-wasmtime")]

use wasmtime::{
    ResourceLimiter,
    StoreLimits,
    StoreLimitsBuilder,
};

pub struct MemoryLimiter {
    limits: StoreLimits,
}

impl MemoryLimiter {
    pub fn new(
        max_memory_size: usize,
    ) -> Self {
        let limits =
            StoreLimitsBuilder::new()
                .memory_size(
                    max_memory_size,
                )
                .build();

        Self {
            limits,
        }
    }

    pub fn limits(
        self,
    ) -> StoreLimits {
        self.limits
    }
}

impl ResourceLimiter for MemoryLimiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        self.limits.memory_growing(
            current,
            desired,
            maximum,
        )
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        self.limits.table_growing(
            current,
            desired,
            maximum,
        )
    }
}
