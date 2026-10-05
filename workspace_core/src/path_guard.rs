//! Yol koruması — yalnızca `std`'ye bağlı, tek başına test edilebilir.
//!
//! Üç garanti:
//!   1. Çağıranın verdiği yol göreli ve yalnızca düz bileşenlerden oluşur
//!      (`..`, kök, sürücü öneki, NUL reddedilir; `.` atlanır).
//!   2. Yolun EN YAKIN MEVCUT atası (symlink'ler çözülerek) kök içinde kalır.
//!      Eski sürüm yalnız doğrudan ebeveyni kontrol ediyordu: ebeveyn henüz
//!      yoksa kontrol atlanıyor ve `linkdir/yeni/f.txt` ile dışarı yazılıyordu.
//!   3. Son bileşen mevcut bir symlink ise hedefi kök içinde olmalıdır
//!      (`follow_final`); aksi halde dışarıdaki dosya okunabiliyordu.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardError {
    /// Yol kökün dışına çıkıyor (ya da symlink ile çıkabiliyor).
    Outside,
    /// Geçersiz yol (NUL vb.).
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Köke eklenmiş tam yol (symlink'ler çözülmemiş hâli).
    pub full: PathBuf,
    /// Normalize edilmiş göreli yol. Boş = kökün kendisi.
    pub rel: PathBuf,
}

impl Resolved {
    pub fn is_root(&self) -> bool {
        self.rel.as_os_str().is_empty()
    }
}

/// Göreli yolu düz bileşenlere indirger.
pub fn normalize(relative: &str) -> Result<PathBuf, GuardError> {
    if relative.contains('\0') {
        return Err(GuardError::Invalid);
    }
    let raw = Path::new(relative);
    if raw.is_absolute() {
        return Err(GuardError::Outside);
    }
    let mut out = PathBuf::new();
    for c in raw.components() {
        match c {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            _ => return Err(GuardError::Outside),
        }
    }
    Ok(out)
}

/// `.git` bileşeni içeren yol mu? (Büyük/küçük harf duyarsız: bazı Android
/// dosya sistemleri büyük/küçük harfi ayırmaz.)
pub fn is_protected(rel: &Path) -> bool {
    rel.components().any(|c| match c {
        Component::Normal(p) => p
            .to_str()
            .map(|s| s.eq_ignore_ascii_case(".git"))
            .unwrap_or(false),
        _ => false,
    })
}

/// `root` önceden `canonicalize` edilmiş olmalıdır.
pub fn resolve(root: &Path, relative: &str, follow_final: bool) -> Result<Resolved, GuardError> {
    let rel = normalize(relative)?;
    let full = root.join(&rel);
    if rel.as_os_str().is_empty() {
        return Ok(Resolved { full, rel });
    }

    // 2) en yakın mevcut ata
    let mut ancestor = full.parent();
    while let Some(a) = ancestor {
        if fs::symlink_metadata(a).is_ok() {
            // Dangling symlink ara klasör olarak kullanılamaz → Outside.
            let real = fs::canonicalize(a).map_err(|_| GuardError::Outside)?;
            if !real.starts_with(root) {
                return Err(GuardError::Outside);
            }
            break;
        }
        ancestor = a.parent();
    }

    // 3) son bileşen symlink ise hedefi kökte kalmalı
    if follow_final {
        if let Ok(meta) = fs::symlink_metadata(&full) {
            if meta.file_type().is_symlink() {
                let real = fs::canonicalize(&full).map_err(|_| GuardError::Outside)?;
                if !real.starts_with(root) {
                    return Err(GuardError::Outside);
                }
            }
        }
    }

    Ok(Resolved { full, rel })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aeth_pg_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        fs::canonicalize(&d).unwrap()
    }

    #[test]
    fn normalize_accepts_plain_and_rejects_escapes() {
        assert_eq!(normalize("a/b.txt").unwrap(), PathBuf::from("a/b.txt"));
        assert_eq!(normalize("./a//b").unwrap(), PathBuf::from("a/b"));
        assert_eq!(normalize("").unwrap(), PathBuf::new());
        assert_eq!(normalize(".").unwrap(), PathBuf::new());
        assert_eq!(normalize("../x"), Err(GuardError::Outside));
        assert_eq!(normalize("a/../../x"), Err(GuardError::Outside));
        assert_eq!(normalize("/etc/passwd"), Err(GuardError::Outside));
        assert_eq!(normalize("a\0b"), Err(GuardError::Invalid));
    }

    #[test]
    fn root_resolves_to_itself() {
        let root = tmp("root");
        for p in ["", ".", "./"] {
            let r = resolve(&root, p, true).unwrap();
            assert!(r.is_root(), "{p:?}");
            assert_eq!(r.full, root);
        }
    }

    #[test]
    fn protected_git_component_is_detected() {
        assert!(is_protected(Path::new(".git/config")));
        assert!(is_protected(Path::new("sub/.GIT/hooks/pre-commit")));
        assert!(is_protected(Path::new(".git")));
        assert!(!is_protected(Path::new(".gitignore")));
        assert!(!is_protected(Path::new("a/git/config")));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_file_pointing_outside_is_rejected_but_inside_is_ok() {
        use std::os::unix::fs::symlink;
        let root = tmp("symfile");
        let out = tmp("symfile_out");
        fs::write(out.join("secret.txt"), "GIZLI").unwrap();
        fs::write(root.join("real.txt"), "ok").unwrap();
        symlink(out.join("secret.txt"), root.join("linkfile")).unwrap();
        symlink(root.join("real.txt"), root.join("inlink")).unwrap();

        assert_eq!(resolve(&root, "linkfile", true), Err(GuardError::Outside));
        assert!(resolve(&root, "inlink", true).is_ok());
        // silme için son bileşen izlenmez: dış link kaldırılabilir
        assert!(resolve(&root, "linkfile", false).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn missing_intermediate_dirs_under_a_symlinked_dir_are_rejected() {
        use std::os::unix::fs::symlink;
        let root = tmp("symdir");
        let out = tmp("symdir_out");
        symlink(&out, root.join("linkdir")).unwrap();

        // ara klasör YOK → eski sürüm burada izin veriyordu
        assert_eq!(resolve(&root, "linkdir/yeni/f.txt", true), Err(GuardError::Outside));
        // ara klasör VAR
        fs::write(out.join("s.txt"), "x").unwrap();
        assert_eq!(resolve(&root, "linkdir/s.txt", true), Err(GuardError::Outside));
        // normal iç yol serbest
        assert!(resolve(&root, "yeni/f.txt", true).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn dangling_symlink_as_directory_is_rejected() {
        use std::os::unix::fs::symlink;
        let root = tmp("dangling");
        symlink(root.join("yok_hedef"), root.join("kirik")).unwrap();
        assert_eq!(resolve(&root, "kirik/x", true), Err(GuardError::Outside));
    }
}
