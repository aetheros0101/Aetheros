// `#[frb(...)]` makroları (flutter_rust_bridge) derleme sırasında
// `frb_expand` adında bir cfg üretiyor; rustc bunu "beklenmeyen cfg"
// diye 13 kez uyarıyor. Zararsız bir uyumsuzluk (frb_macros'ın eski bir
// check-cfg deklarasyonu) — `cargo update -p flutter_rust_bridge_macros`
// ile düzelebilir ama garanti değil, bu yüzden şimdilik susturuyoruz.
#![allow(unexpected_cfgs)]
// `async_trait`, Result döndüren metotlara ayrıca #[must_use] ekliyor; clippy
// bunu çift must_use sayıyor (makro kaynaklı, kod hatası değil).
#![allow(clippy::double_must_use)]

pub mod errors;
pub mod events;
pub mod metrics;
pub mod persistence;
pub mod runtime;
pub mod task;
pub mod types;
pub mod wasm;
pub mod worker;

pub mod api;
pub mod remote;
pub mod security;

pub mod agents;
pub mod ai;
pub mod logging;
pub mod orchestration;
pub mod scripting;
pub mod workflows;

// ── Flutter-Rust Bridge ───────────────────────────────────
// Mobil FFI katmanı. flutter_rust_bridge_codegen bu modülü
// tarayarak flutter_app/lib/src/rust/ altına Dart dosyaları üretir.
// Sunucu build'inde de derlenir; binary boyutuna etkisi minimumdur.
pub mod bridge;

// flutter_rust_bridge codegen çıktısı; yeniden üretildiğinde üzerine yazılır,
// bu yüzden lint istisnası burada (dosyanın içinde değil) tutulur.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub mod frb_generated;

pub use runtime::api::RuntimeHandle;
pub use runtime::bootstrap::RuntimeBootstrap;
pub use runtime::config::RuntimeConfig;
pub use runtime::runtime::Runtime;

#[cfg(test)]
mod tests;
