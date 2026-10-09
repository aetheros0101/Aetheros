// ============================================================
// bridge/api/script.rs
//
// Agent script kaydı ve capability grant.
// ============================================================

use tracing::info;

use crate::bridge::api::error::require_runtime;
use crate::bridge::api::helpers::parse_capability;

// ── V10 Sprint 1b: Agent Tool Script Registry ──────────────
//
// Akış: önce upload_wasm_module() / compile_wat_to_wasm() ile binary
// yüklenir (hash döner), sonra bu hash burada isimlendirilerek agent
// tool'u olarak kaydedilir. start_agent() her çağrıda script_registry'
// deki TÜM script'leri agent'ın araç kutusuna (tools) ekler — hangi
// script'lerin çalışabileceğini CapabilityEngine (WasmExecution grant'ı)
// belirler, registry'de olmak tek başına yeterli değildir.

/// Daha önce upload edilmiş bir WASM modülünü, agent'ların
/// çağırabileceği isimlendirilmiş bir tool olarak kaydet.
pub fn register_agent_script(
    name: String,
    wasm_module_hash_hex: String,
    entrypoint: String,
    timeout_ms: u64,
) -> Result<(), String> {
    let rt = require_runtime()?;

    let hash = crate::wasm::module_store::ModuleStore::hex_to_hash(&wasm_module_hash_hex)
        .map_err(|e| format!("Geçersiz hash: {e}"))?;

    let binary = rt
        .module_store
        .get(&hash)
        .map_err(|e| format!("Modül bulunamadı: {e}"))?;

    let script = crate::scripting::definition::ScriptDefinition::from_binary(
        name.clone(),
        (*binary).clone(),
        entrypoint,
        timeout_ms,
    );

    rt.script_registry.register(script);

    info!(name = %name, hash = %wasm_module_hash_hex, "Agent script kaydedildi");
    Ok(())
}

/// Şu an kayıtlı, agent'ların potansiyel olarak erişebileceği
/// script isimlerini listele (UI'da göstermek için).
pub fn list_agent_scripts() -> Result<Vec<String>, String> {
    let rt = require_runtime()?;

    Ok(rt
        .script_registry
        .list()
        .into_iter()
        .map(|s| s.name)
        .collect())
}

/// Bir agent'a tek bir capability grant et.
///
/// `capability`: "wasm_execution" | "workflow_execution" | "ai_reasoning"
///               | "remote_execution" | "terminal_execution"
///               | "workspace_read" | "workspace_write" | "workspace_mutate"
///
/// TERCİH EDİLEN YOL: `start_agent(..., capabilities)` — yetkiler agent
/// çalışmaya başlamadan ÖNCE, atomik verilir (yarış yok). Bu fonksiyon,
/// çalışan bir agent'a SONRADAN yetki eklemek içindir ve start_agent'ın
/// döndüğü agent_id ile, execution ilk tool adımına gelmeden önce
/// çağrılmazsa yarışa açıktır.
/// Bu yarış durumu kabul edilebilir (planning genelde tool adımından
/// önce bir miktar zaman alır) ama sağlam bir çözüm değil — gerçek
/// çözüm Approval Engine'in agent'ı "grant bekliyor" durumunda
/// başlatıp ilk adımdan önce durdurmasıdır (gelecek sprint).
pub fn grant_agent_capability(agent_id_hex: String, capability: String) -> Result<(), String> {
    let rt = require_runtime()?;

    let agent_id =
        uuid::Uuid::parse_str(&agent_id_hex).map_err(|e| format!("Geçersiz agent_id: {e}"))?;

    let cap = parse_capability(capability.as_str())?;

    rt.capability_engine.grant_capability(agent_id, cap);

    info!(agent_id = %agent_id, capability = %capability, "Agent capability grant edildi");
    Ok(())
}
