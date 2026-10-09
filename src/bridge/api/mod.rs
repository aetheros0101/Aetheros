// ============================================================
// bridge/api/mod.rs
//
// Flutter'ın doğrudan çağırdığı Rust fonksiyonları.
//
// flutter_rust_bridge_codegen bu modülü tarar ve
// flutter_app/lib/src/rust/api/aetheros.dart dosyasını üretir.
//
// KURALLAR:
//   - pub async fn → Dart'ta Future<T> olur
//   - pub fn       → Dart'ta T olur (sync)
//   - Result<T, String> → Dart'ta throws String
//   - anyhow::Result<T> → Dart'ta throws AetherError
//
// ÖMÜR:
//   initialize_runtime() → [kullan] → (shutdown)
//   initialize_runtime() çağrılmadan diğerleri RuntimeNotInitialized döner.
//
// MODÜL YAPISI (sorumluluk ayrımı):
//   runtime   — FRB init + runtime yaşam döngüsü
//   task      — task gönderme / durum / liste / resubmit
//   module    — WASM derleme / yükleme / varlık
//   script    — agent script kaydı + capability grant
//   log       — log izleme
//   metrics   — metrik snapshot
//   agent_api — agent başlat / durum / liste
//   approval  — onay motoru
//   audit     — denetim kayıtları
//   workflow  — workflow başlat / durum / liste
//   cluster   — cluster durumu / node kaydı
//   ai        — AI provider yapılandırma / sohbet
//   terminal  — kullanıcı terminali
//   workspace — dosyalar ekranı (ince sarmalayıcı)
//   error     — merkezi require_runtime + hata dönüşümü
//   helpers   — parse / format yardımcıları
// ============================================================

mod error;
mod helpers;

mod runtime;
mod task;
mod module;
mod script;
mod log;
mod metrics;
mod agent_api;
mod approval;
mod audit;
mod workflow;
mod cluster;
mod ai;
mod terminal;
mod workspace;

// Re-export all public FRB API surface so paths remain:
//   crate::bridge::api::{submit_task, start_agent, ...}
pub use runtime::*;
pub use task::*;
pub use module::*;
pub use script::*;
pub use log::*;
pub use metrics::*;
pub use agent_api::*;
pub use approval::*;
pub use audit::*;
pub use workflow::*;
pub use cluster::*;
pub use ai::*;
pub use terminal::*;
pub use workspace::*;

// Internal helpers that may be used by sibling modules or tests
#[allow(unused_imports)]
pub(crate) use error::{describe_runtime_error, require_runtime};
#[allow(unused_imports)]
pub(crate) use helpers::{
    describe_audit_event, fmt_call_args, parse_capability, parse_hash, parse_priority,
};
