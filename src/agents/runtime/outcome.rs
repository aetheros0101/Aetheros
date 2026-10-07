//! Execution and step outcomes.

use uuid::Uuid;

/// Bir agent execution'ının (ya da resume'unun) sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentOutcome {
    /// Plan sonuna kadar (ya da budget bitene kadar) hatasız tamamlandı.
    Completed,
    /// Bir adım SecurityGovernor'dan RequiresApproval aldı — tool hiç
    /// invoke edilmeden execution duraklatıldı.
    PendingApproval { approval_id: Uuid },
}

/// Tek bir adımın sonucu (`execute_guarded_step`).
#[derive(Debug)]
pub(crate) enum StepOutcome {
    /// Adım gerçekten çalıştı (başarılı ya da retryable bir hatayla).
    Ran { output: String, success: bool },
    /// Governor RequiresApproval dedi — invoke edilmeden duraklatıldı.
    Paused { approval_id: Uuid },
}
