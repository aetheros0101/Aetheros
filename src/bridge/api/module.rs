// ============================================================
// bridge/api/module.rs
//
// WASM modül yönetimi: WAT derleme, yükleme, varlık kontrolü.
// ============================================================

use tracing::info;

use crate::bridge::api::error::require_runtime;
use crate::bridge::types::ModuleUploadResponse;

// ── WASM modül yönetimi ───────────────────────────────────

/// WAT (WebAssembly Text Format) kaynak kodunu WASM binary'ye derle
/// ve ModuleStore'a kaydet.
///
/// Flutter script editor'ün "Derle & Yükle" butonu bu fonksiyonu çağırır.
/// Derleme Rust tarafında `wat::parse_str()` ile yapılır.
///
/// Başarı: ModuleUploadResponse { hash, size } döner.
/// Hata:  WAT sözdizimi hatası string olarak döner.
pub async fn compile_wat_to_wasm(
    name: String,
    wat_source: String,
    entrypoint: String,
    _timeout_ms: u64, // Gelecekte execution timeout için — şimdilik WAT compile'da kullanılmıyor
) -> Result<ModuleUploadResponse, String> {
    // WAT → WASM binary (Rust tarafında, zero-dependency)
    let wasm_binary =
        wat::parse_str(&wat_source).map_err(|e| format!("WAT derleme hatası: {e}"))?;

    info!(
        name       = %name,
        entrypoint = %entrypoint,
        bytes      = wasm_binary.len(),
        "WAT başarıyla WASM'a derlendi"
    );

    // Artık binary olarak upload_wasm_module'ün yaptığını tekrar et
    let rt = require_runtime()?;

    let size = wasm_binary.len() as u64;

    let hash_bytes = rt
        .module_store
        .store(wasm_binary.clone())
        .map_err(|e| format!("Modül kaydedilemedi: {e}"))?;

    let hash = crate::wasm::module_store::ModuleStore::hash_to_hex(&hash_bytes);

    // Diske kaydet
    let file_path = format!("{}/{}.wasm", rt.modules_dir, hash);
    if let Err(e) = std::fs::write(&file_path, &wasm_binary) {
        tracing::warn!(path = %file_path, err = %e, "WASM diske yazılamadı");
    } else {
        info!(hash = %hash, path = %file_path, "WAT→WASM derlendi ve diske kaydedildi");
    }

    Ok(ModuleUploadResponse { hash, size })
}

/// WASM modülü yükle → hash döner.
///
/// Flutter, dosyayı bytes olarak Rust'a verir.
/// Rust, ModuleStore'a kaydeder ve SHA-256 hash döner.
/// Sonraki task'larda bu hash kullanılır.
///
/// Örnek (Dart):
/// ```dart
/// final bytes = await File("my_module.wasm").readAsBytes();
/// final hash = await AetherApi.uploadWasmModule(bytes: bytes);
/// ```
pub async fn upload_wasm_module(bytes: Vec<u8>) -> Result<ModuleUploadResponse, String> {
    let rt = require_runtime()?;

    let size = bytes.len() as u64;

    // ModuleStore'a kaydet → SHA-256 hash al (idempotent)
    let hash_bytes = rt
        .module_store
        .store(bytes.clone())
        .map_err(|e| format!("Modül kaydedilemedi: {e}"))?;

    let hash = crate::wasm::module_store::ModuleStore::hash_to_hex(&hash_bytes);

    // Diske kaydet → restart'ta otomatik restore edilir
    // {docDir}/modules/{hash}.wasm
    let file_path = format!("{}/{}.wasm", rt.modules_dir, hash);
    if let Err(e) = std::fs::write(&file_path, &bytes) {
        // Disk yazma hatası — in-memory yükleme başarılı, sadece persist olmayacak.
        // Uyarı log'la, hata döndürme (modül bu session'da çalışır).
        tracing::warn!(
            path = %file_path,
            err  = %e,
            "WASM modülü diske yazılamadı (session'da çalışır ama restart'ta kaybolur)"
        );
    } else {
        info!(hash = %hash, path = %file_path, "WASM modülü diske kaydedildi");
    }

    info!(hash = %hash, size = size, "WASM modülü yüklendi");

    Ok(ModuleUploadResponse { hash, size })
}

/// Belirtilen hash'e sahip modül runtime'da kayıtlı mı?
///
/// WasmModuleScreen açılışında, eski oturumdan kalan
/// meta-data'yı doğrulamak için her modül için çağrılır.
pub fn check_module_exists(hash_hex: String) -> Result<bool, String> {
    let rt = require_runtime()?;

    let hash = crate::wasm::module_store::ModuleStore::hex_to_hash(&hash_hex)
        .map_err(|e| format!("Geçersiz hash: {e}"))?;

    Ok(rt.module_store.contains(&hash))
}
