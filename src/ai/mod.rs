//! LLM sağlayıcı katmanı.
//!
//! - `providers/`  Anthropic, Gemini, Ollama, OpenAI istemcileri
//! - `routing/`    aktif sağlayıcı seçimi (`ProviderRouter`)
//! - `inference/`  sağlayıcıdan bağımsız istek/yanıt tipleri
//! - `errors`      ortak `AiError`
//!
//! Planlama, bellek ve araç çağrısı mantığı burada **değil**, `agents/`
//! altında yaşar (`agents::planning`, `agents::memory`, `agents::tools`).
//! `agents` bu katmana bağlanır; bu katman `agents`'a bağlanamaz.

pub mod errors;
pub mod inference;
pub mod providers;
pub mod routing;
