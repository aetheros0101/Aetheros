//! Execution budget: limitler + anlık tüketim muhasebesi.

use crate::agents::errors::AgentError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionBudget {
    pub max_tokens: usize,
    pub max_steps: usize,
    pub max_runtime_seconds: usize,
}

impl Default for AgentExecutionBudget {
    fn default() -> Self {
        Self {
            max_tokens: 100_000,
            max_steps: 50,
            max_runtime_seconds: 600,
        }
    }
}

/// Çalışma zamanı tüketim sayaçları.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetAccounting {
    pub tokens_used: usize,
    pub steps_used: usize,
    pub started_at_ms: u64,
    pub tool_invocations: usize,
}

impl BudgetAccounting {
    pub fn start(now_ms: u64) -> Self {
        Self {
            started_at_ms: now_ms,
            ..Default::default()
        }
    }

    pub fn record_step(&mut self) {
        self.steps_used = self.steps_used.saturating_add(1);
    }

    pub fn record_tokens(&mut self, n: usize) {
        self.tokens_used = self.tokens_used.saturating_add(n);
    }

    pub fn record_tool(&mut self) {
        self.tool_invocations = self.tool_invocations.saturating_add(1);
    }

    pub fn elapsed_seconds(&self, now_ms: u64) -> usize {
        now_ms.saturating_sub(self.started_at_ms) as usize / 1000
    }

    pub fn check(&self, budget: &AgentExecutionBudget, now_ms: u64) -> Result<(), AgentError> {
        if self.steps_used >= budget.max_steps {
            return Err(AgentError::budget(format!(
                "step limit {} reached",
                budget.max_steps
            )));
        }
        if self.tokens_used >= budget.max_tokens {
            return Err(AgentError::budget(format!(
                "token limit {} reached",
                budget.max_tokens
            )));
        }
        if self.elapsed_seconds(now_ms) >= budget.max_runtime_seconds {
            return Err(AgentError::budget(format!(
                "runtime limit {}s reached",
                budget.max_runtime_seconds
            )));
        }
        Ok(())
    }

    pub fn remaining_steps(&self, budget: &AgentExecutionBudget) -> usize {
        budget.max_steps.saturating_sub(self.steps_used)
    }
}
