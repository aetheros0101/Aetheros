//src/ai/tools/registry.rs

use std::sync::Arc;

use dashmap::DashMap;

use crate::types::agent_tool::AgentTool;

pub struct ToolRegistry {
    tools:
        DashMap<
            String,
            Arc<dyn AgentTool>,
        >,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools:
                DashMap::new(),
        }
    }

    pub fn register(
        &self,
        tool:
            Arc<dyn AgentTool>,
    ) {
        self.tools.insert(
            tool.name().into(),
            tool,
        );
    }

    pub fn total_tools(
        &self,
    ) -> usize {
        self.tools.len()
    }
}
