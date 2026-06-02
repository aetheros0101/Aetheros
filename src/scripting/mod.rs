// ============================================================
// src/scripting/mod.rs  (YENİ MODÜL)
//
// Sprint 3: Script Engine
//
// Kullanıcı bir script tanımlar (WAT/WASM binary),
// ScriptEngine çalıştırır, sonuç String olarak döner.
//
// Katman yapısı:
//   ScriptDefinition  → script metadata + binary
//   ScriptEngine      → WasmEngine üzerinde çalıştırır
//   ScriptResult      → çıktı + execution süresi
//   ScriptTool        → AgentTool impl (agent içinde kullanım)
//   ScriptRegistry    → isimle kayıt + lookup
//
// Kullanım örneği:
//   let engine = ScriptEngine::new(wasm_engine);
//   let result = engine.run(&script_def).await?;
//   println!("{}", result.output);
// ============================================================

pub mod definition;
pub mod engine;
pub mod registry;
pub mod result;
pub mod tool;

pub use definition::ScriptDefinition;
pub use engine::ScriptEngine;
pub use registry::ScriptRegistry;
pub use result::ScriptResult;
pub use tool::ScriptTool;
