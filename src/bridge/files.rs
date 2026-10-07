// ============================================================
// src/bridge/files.rs
//
// Dosyalar ekranının mantığı: kullanıcının KENDİ yaptığı dosya işlemleri
// (agent'larınkinden ayrı — onay/governor yok, ama aynı path guard var:
// workspace dışına çıkamaz, `.git` değiştirilemez, kök silinemez).
//
// Saf fonksiyonlar `&Workspace` alır; böylece global runtime olmadan
// test edilir. `bridge::api` yalnızca ince FRB sarmalayıcılarıdır.
// Hata metinleri doğrudan kullanıcıya gösterilir → Türkçe.
// ============================================================

use aetheros_workspace::{
    EntryKind, Workspace, WorkspaceError, WorkspaceToolRequest, WorkspaceToolResponse,
};

use crate::bridge::types::{WorkspaceFileResponse, WorkspaceItem};

/// Görüntüleyicinin açacağı en büyük metin dosyası.
pub const MAX_VIEW_BYTES: usize = 512 * 1024;

/// Sürüm çakışması hatalarının ön eki (arayüz bunu tanır).
pub const CONFLICT_PREFIX: &str = "conflict:";

/// Hatayı kullanıcıya gösterilecek Türkçe metne çevirir.
pub fn friendly_error(e: WorkspaceError) -> String {
    match e {
        WorkspaceError::PathOutsideRoot => "Yol çalışma alanının dışında.".into(),
        WorkspaceError::InvalidPath => "Geçersiz yol (kök klasör değiştirilemez).".into(),
        WorkspaceError::IsDirectory => "Bu bir klasör, dosya değil.".into(),
        WorkspaceError::NotDirectory => "Bu bir klasör değil.".into(),
        WorkspaceError::AlreadyExists(p) => format!("'{p}' zaten var."),
        WorkspaceError::NotFound(p) => format!("'{p}' bulunamadı."),
        WorkspaceError::NotText => "Bu dosya metin değil (ikili dosya), görüntülenemez.".into(),
        WorkspaceError::Protected(_) => ".git klasörü korumalı, değiştirilemez.".into(),
        WorkspaceError::TooLarge { limit } => {
            format!("Dosya çok büyük (en fazla {} MB).", (limit / (1024 * 1024)).max(1))
        }
        WorkspaceError::SourceInvalid(m) => m,
        WorkspaceError::VersionConflict { .. } => format!(
            "{CONFLICT_PREFIX} Dosya sen açtıktan sonra değişti (bir agent yazmış olabilir)."
        ),
        WorkspaceError::Io(io) if io.to_string().contains("read limit") => format!(
            "Dosya görüntülemek için çok büyük (en fazla {} KB).",
            MAX_VIEW_BYTES / 1024
        ),
        WorkspaceError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
            "Dosya ya da klasör bulunamadı.".into()
        }
        other => other.to_string(),
    }
}

/// Dosya adı tek bir bileşen olmalı: ayırıcı, `..`, boş ya da NUL yok.
pub fn check_name(name: &str) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() || n == "." || n == ".." {
        return Err("Geçersiz ad.".into());
    }
    if n.contains('/') || n.contains('\\') || n.contains('\0') {
        return Err("Ad '/' veya '\\' içeremez.".into());
    }
    Ok(())
}

/// `dir` + `name` → workspace-göreli yol (`dir` boş ya da "." ise yalnız ad).
pub fn join_rel(dir: &str, name: &str) -> String {
    let d = dir.trim().trim_matches('/');
    if d.is_empty() || d == "." {
        name.to_string()
    } else {
        format!("{d}/{name}")
    }
}

fn to_file_response(doc: aetheros_workspace::FileDocument) -> WorkspaceFileResponse {
    WorkspaceFileResponse {
        path: doc.path,
        content: doc.content,
        size: doc.size as i64,
        version: doc.version.to_string(),
        readonly: doc.readonly,
    }
}

/// Bir klasörün DOĞRUDAN çocukları (klasörler önce).
pub fn list(ws: &Workspace, dir: &str, include_hidden: bool) -> Result<Vec<WorkspaceItem>, String> {
    // tree(depth): 0 = yalnız doğrudan çocuklar.
    let entries = ws.tree(dir, 0, include_hidden).map_err(friendly_error)?;
    let mut items: Vec<WorkspaceItem> = entries
        .into_iter()
        .map(|e| WorkspaceItem {
            is_dir: matches!(e.kind, EntryKind::Directory),
            path: e.path,
            name: e.name,
            size: e.size as i64,
            hidden: e.hidden,
        })
        .collect();
    // Klasörler önce, kendi içinde ada göre (büyük/küçük harf duyarsız).
    items.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(items)
}

pub fn read(ws: &Workspace, path: &str) -> Result<WorkspaceFileResponse, String> {
    ws.read_file(path, MAX_VIEW_BYTES)
        .map(to_file_response)
        .map_err(friendly_error)
}

/// Yazar. `expected_version` verilirse, dosya o sürümden değiştiyse yazmaz
/// ve `conflict:` ile başlayan hata döner (agent'ın yazdığını ezmemek için).
pub fn write(
    ws: &Workspace,
    path: &str,
    content: &str,
    expected_version: Option<&str>,
) -> Result<WorkspaceFileResponse, String> {
    let expected = match expected_version {
        Some(v) if !v.trim().is_empty() => {
            Some(v.trim().parse::<u64>().map_err(|_| "Geçersiz sürüm bilgisi.".to_string())?)
        }
        _ => None,
    };
    let resp = ws
        .execute_tool(WorkspaceToolRequest::WriteFile {
            path: path.to_string(),
            content: content.to_string(),
            expected_version: expected,
        })
        .map_err(friendly_error)?;
    match resp {
        WorkspaceToolResponse::Written(doc) => Ok(to_file_response(doc)),
        _ => Err("Beklenmeyen yanıt (yazma).".into()),
    }
}

pub fn create(ws: &Workspace, path: &str, is_dir: bool) -> Result<(), String> {
    if let Some(name) = path.rsplit('/').next() {
        check_name(name)?;
    }
    let r = if is_dir { ws.create_dir(path) } else { ws.create_file(path) };
    r.map(|_| ()).map_err(friendly_error)
}

pub fn rename(ws: &Workspace, from: &str, to: &str) -> Result<(), String> {
    if let Some(name) = to.rsplit('/').next() {
        check_name(name)?;
    }
    ws.rename(from, to).map_err(friendly_error)
}

pub fn delete(ws: &Workspace, path: &str) -> Result<(), String> {
    ws.delete(path).map_err(friendly_error)
}

/// Kullanıcının seçtiği dış dosyayı `dest_dir` altına `file_name` adıyla kopyalar.
pub fn import(
    ws: &Workspace,
    source_path: &str,
    dest_dir: &str,
    file_name: &str,
) -> Result<String, String> {
    check_name(file_name)?;
    let dest = join_rel(dest_dir, file_name.trim());
    ws.import_external(std::path::Path::new(source_path), &dest)
        .map(|r| r.path)
        .map_err(friendly_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ws() -> (tempfile::TempDir, Workspace) {
        let d = tempfile::tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        (d, w)
    }

    #[test]
    fn name_and_join_rules() {
        assert!(check_name("a.txt").is_ok());
        for bad in ["", " ", ".", "..", "a/b", "a\\b", "a\0b"] {
            assert!(check_name(bad).is_err(), "{bad:?}");
        }
        assert_eq!(join_rel("", "a.txt"), "a.txt");
        assert_eq!(join_rel(".", "a.txt"), "a.txt");
        assert_eq!(join_rel("d", "a.txt"), "d/a.txt");
        assert_eq!(join_rel("/d/e/", "a.txt"), "d/e/a.txt");
    }

    #[test]
    fn list_puts_folders_first_and_hides_dotfiles_by_default() {
        let (d, w) = ws();
        std::fs::write(d.path().join("b.txt"), "x").unwrap();
        std::fs::write(d.path().join(".gizli"), "x").unwrap();
        std::fs::create_dir(d.path().join("klasor")).unwrap();
        std::fs::write(d.path().join("klasor/ic.txt"), "x").unwrap();

        let items = list(&w, "", false).unwrap();
        let names: Vec<_> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["klasor", "b.txt"], "klasör önce, gizli yok, yalnız doğrudan çocuklar");
        assert!(items[0].is_dir && !items[1].is_dir);

        let all = list(&w, ".", true).unwrap();
        assert!(all.iter().any(|i| i.name == ".gizli" && i.hidden));
        let inner = list(&w, "klasor", false).unwrap();
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].path, "klasor/ic.txt");
    }

    #[test]
    fn write_read_roundtrip_and_conflict_detection() {
        let (d, w) = ws();
        create(&w, "n.txt", false).unwrap();
        let first = write(&w, "n.txt", "bir", None).unwrap();
        assert_eq!(read(&w, "n.txt").unwrap().content, "bir");

        // doğru sürümle yazılır
        let second = write(&w, "n.txt", "iki", Some(&first.version)).unwrap();
        assert_eq!(second.content, "iki");

        // dosya dışarıdan (agent gibi) değişir → eski sürümle yazma reddedilir
        std::thread::sleep(Duration::from_millis(30));
        std::fs::write(d.path().join("n.txt"), "agent yazdi").unwrap();
        let err = write(&w, "n.txt", "ben", Some(&second.version)).unwrap_err();
        assert!(err.starts_with(CONFLICT_PREFIX), "{err}");
        assert_eq!(read(&w, "n.txt").unwrap().content, "agent yazdi", "ezilmemeli");

        // sürümsüz (bilinçli üzerine yaz) çalışır
        write(&w, "n.txt", "ben", None).unwrap();
        assert_eq!(read(&w, "n.txt").unwrap().content, "ben");
        assert!(write(&w, "n.txt", "x", Some("sayi-degil")).is_err());
    }

    #[test]
    fn create_rename_delete_flow_with_turkish_errors() {
        let (d, w) = ws();
        create(&w, "klasor", true).unwrap();
        create(&w, "klasor/a.txt", false).unwrap();
        assert!(create(&w, "klasor/a.txt", false).unwrap_err().contains("zaten var"));
        assert!(create(&w, "klasor/..", false).is_err());

        rename(&w, "klasor/a.txt", "klasor/b.txt").unwrap();
        assert!(d.path().join("klasor/b.txt").exists());
        assert!(rename(&w, "klasor/yok.txt", "klasor/c.txt").unwrap_err().contains("bulunamadı"));
        assert!(rename(&w, "klasor/b.txt", "klasor/ba/sd").is_ok(), "alt yola taşıma serbest");

        delete(&w, "klasor").unwrap();
        assert!(!d.path().join("klasor").exists());
    }

    #[test]
    fn root_git_and_outside_are_refused_with_clear_messages() {
        let (_d, w) = ws();
        assert!(delete(&w, ".").unwrap_err().contains("kök"));
        assert!(delete(&w, "").is_err());
        assert!(create(&w, ".git/hooks", true).unwrap_err().contains(".git"));
        assert!(write(&w, "../kacis.txt", "x", None).unwrap_err().contains("dışında"));
        assert!(read(&w, "../x").unwrap_err().contains("dışında"));
    }

    #[test]
    fn binary_and_oversized_files_give_friendly_errors() {
        let (d, w) = ws();
        std::fs::write(d.path().join("ikili.bin"), [0xff, 0xfe, 0x00, 0x80]).unwrap();
        assert!(read(&w, "ikili.bin").unwrap_err().contains("metin değil"));

        std::fs::write(d.path().join("buyuk.txt"), "x".repeat(MAX_VIEW_BYTES + 10)).unwrap();
        let e = read(&w, "buyuk.txt").unwrap_err();
        assert!(e.contains("çok büyük"), "{e}");
    }

    #[test]
    fn import_copies_with_validated_name_and_never_overwrites() {
        let (d, w) = ws();
        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("kaynak.txt");
        std::fs::write(&src, "dış").unwrap();
        let src_s = src.to_string_lossy().into_owned();

        create(&w, "belge", true).unwrap();
        let p = import(&w, &src_s, "belge", "yeni.txt").unwrap();
        assert_eq!(p, "belge/yeni.txt");
        assert_eq!(std::fs::read_to_string(d.path().join("belge/yeni.txt")).unwrap(), "dış");

        assert!(import(&w, &src_s, "belge", "yeni.txt").unwrap_err().contains("zaten var"));
        assert!(import(&w, &src_s, "belge", "../x.txt").is_err(), "ad tek bileşen olmalı");
        assert!(import(&w, "/yok/yok.txt", "belge", "y.txt").unwrap_err().contains("bulunamadı"));
        assert!(import(&w, &src_s, ".git", "x.txt").unwrap_err().contains(".git"));
    }
}
