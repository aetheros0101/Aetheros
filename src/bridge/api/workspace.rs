// ============================================================
// bridge/api/workspace.rs
//
// Dosyalar ekranı: kullanıcının kendi workspace işlemleri.
// İnce FRB sarmalayıcıları; mantık bridge::files'ta.
// ============================================================

use crate::bridge::api::error::require_runtime;
use crate::bridge::types::{WorkspaceFileResponse, WorkspaceItem};

fn files_ws() -> Result<std::sync::Arc<aetheros_workspace::Workspace>, String> {
    let rt = require_runtime()?;
    rt.workspace
        .clone()
        .ok_or_else(|| "Çalışma alanı açılamadı.".to_string())
}

/// Çalışma alanının cihazdaki tam yolu (bilgi amaçlı gösterim).
pub fn workspace_files_root() -> Result<String, String> {
    let ws = files_ws()?;
    Ok(ws.info().root)
}

/// `dir` klasörünün doğrudan çocukları ("" ya da "." = kök).
pub fn workspace_list_dir(dir: String, include_hidden: bool) -> Result<Vec<WorkspaceItem>, String> {
    let ws = files_ws()?;
    crate::bridge::files::list(&ws, &dir, include_hidden)
}

/// Metin dosyasını oku (en fazla 512 KB).
pub fn workspace_read_text(path: String) -> Result<WorkspaceFileResponse, String> {
    let ws = files_ws()?;
    crate::bridge::files::read(&ws, &path)
}

/// Metin dosyasını yaz. `expected_version` verilirse dosya o sürümden
/// değiştiyse yazmaz; hata "conflict:" ile başlar.
pub fn workspace_write_text(
    path: String,
    content: String,
    expected_version: Option<String>,
) -> Result<WorkspaceFileResponse, String> {
    let ws = files_ws()?;
    crate::bridge::files::write(&ws, &path, &content, expected_version.as_deref())
}

/// Yeni boş dosya ya da klasör oluştur.
pub fn workspace_create_entry(path: String, is_dir: bool) -> Result<(), String> {
    let ws = files_ws()?;
    crate::bridge::files::create(&ws, &path, is_dir)
}

/// Yeniden adlandır / taşı (hedef varsa hata).
pub fn workspace_rename_entry(from: String, to: String) -> Result<(), String> {
    let ws = files_ws()?;
    crate::bridge::files::rename(&ws, &from, &to)
}

/// Dosya ya da klasörü (içiyle) sil. Geri alınamaz; onay arayüzdedir.
pub fn workspace_delete_entry(path: String) -> Result<(), String> {
    let ws = files_ws()?;
    crate::bridge::files::delete(&ws, &path)
}

/// Dosya seçicinin verdiği dış dosyayı `dest_dir/file_name` olarak kopyala.
/// Dönen değer: workspace-göreli yeni yol.
pub fn workspace_import_file(
    source_path: String,
    dest_dir: String,
    file_name: String,
) -> Result<String, String> {
    let ws = files_ws()?;
    crate::bridge::files::import(&ws, &source_path, &dest_dir, &file_name)
}
