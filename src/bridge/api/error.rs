// ============================================================
// bridge/api/error.rs
//
// Merkezi hata yardımcıları.
// Runtime erişimi ve hata dönüşümleri burada toplanır.
// ============================================================

use crate::bridge::state::{MobileRuntime, get_runtime};

/// Runtime'ı alır; yoksa tutarlı "RuntimeNotInitialized" hatası döner.
#[inline]
pub(crate) fn require_runtime() -> Result<&'static MobileRuntime, String> {
    get_runtime().ok_or_else(|| "RuntimeNotInitialized".to_string())
}

/// Agent hatasını kullanıcıya gösterilecek düz metne çevirir
/// (`TaskExecutionFailed { message: "..." }` yerine yalnız mesaj).
pub(crate) fn describe_runtime_error(e: &crate::errors::runtime::RuntimeError) -> String {
    match e {
        crate::errors::runtime::RuntimeError::TaskExecutionFailed { message } => message.clone(),
        other => other.to_string(),
    }
}
