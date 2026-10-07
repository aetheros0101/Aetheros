//! Yaşam döngüsü FSM: yasal geçişler.

use crate::agents::errors::{AgentError, AgentErrorKind};
use crate::agents::state::AgentState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTransition {
    pub from: AgentState,
    pub to: AgentState,
    pub reason: Option<String>,
    pub at_ms: u64,
}

pub struct AgentLifecycle {
    state: AgentState,
    history: Vec<LifecycleTransition>,
}

impl AgentLifecycle {
    pub fn new() -> Self {
        Self {
            state: AgentState::Registered,
            history: Vec::new(),
        }
    }

    pub fn state(&self) -> AgentState {
        self.state
    }

    pub fn history(&self) -> &[LifecycleTransition] {
        &self.history
    }

    pub fn can_transition(from: AgentState, to: AgentState) -> bool {
        use AgentState::*;
        if from == to {
            return true;
        }
        if from.is_terminal() {
            return to == Registered;
        }
        match (from, to) {
            (Registered, Initializing | Cancelled) => true,
            (Initializing, Planning | Failed | Cancelled) => true,
            (Planning, Executing | Failed | Cancelled | Waiting) => true,
            (Executing, Waiting | Retrying | Completed | Failed | Cancelled | Suspended) => true,
            (Waiting, Executing | Cancelled | Failed | Suspended) => true,
            (Retrying, Executing | Failed | Cancelled) => true,
            (Suspended, Executing | Cancelled | Failed) => true,
            _ => false,
        }
    }

    pub fn transition(
        &mut self,
        to: AgentState,
        reason: Option<String>,
        now_ms: u64,
    ) -> Result<LifecycleTransition, AgentError> {
        if !Self::can_transition(self.state, to) {
            return Err(AgentError::new(
                AgentErrorKind::Internal,
                format!(
                    "illegal lifecycle transition {} → {}",
                    self.state.label(),
                    to.label()
                ),
            ));
        }
        let event = LifecycleTransition {
            from: self.state,
            to,
            reason,
            at_ms: now_ms,
        };
        self.state = to;
        self.history.push(event.clone());
        if self.history.len() > 200 {
            self.history.drain(0..self.history.len() - 200);
        }
        Ok(event)
    }

    pub fn force_terminal(&mut self, to: AgentState, reason: Option<String>, now_ms: u64) {
        if !to.is_terminal() {
            return;
        }
        let event = LifecycleTransition {
            from: self.state,
            to,
            reason,
            at_ms: now_ms,
        };
        self.state = to;
        self.history.push(event);
    }
}

impl Default for AgentLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let mut lc = AgentLifecycle::new();
        lc.transition(AgentState::Initializing, None, 1).unwrap();
        lc.transition(AgentState::Planning, None, 2).unwrap();
        lc.transition(AgentState::Executing, None, 3).unwrap();
        lc.transition(AgentState::Completed, None, 4).unwrap();
        assert!(lc.state().is_terminal());
    }

    #[test]
    fn rejects_illegal() {
        let mut lc = AgentLifecycle::new();
        assert!(lc.transition(AgentState::Executing, None, 1).is_err());
    }
}
