//! Git sahneleme (stage / unstage) ve commit.
//!
//! Tüm komutlar `super::git` ile arındırılmış ortamda çalışır (HOME yok,
//! hook yok, pager yok). Yol argümanları her zaman `--` ile ayrılır; böylece
//! `-` ile başlayan bir dosya adı seçenek gibi yorumlanamaz.

use super::{git, run};
use crate::error::{Result, WorkspaceError};
use std::path::Path;

/// Commit mesajı için üst sınır (bayt).
const MAX_MESSAGE_BYTES: usize = 8 * 1024;

fn fail(output: &std::process::Output) -> WorkspaceError {
    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let err = if err.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        err
    };
    WorkspaceError::Git(err)
}

/// `path` (workspace-göreli) için değişiklikleri sahneye al.
pub fn stage(root: &Path, path: &str) -> Result<()> {
    let mut c = git(root);
    // `--literal-pathspecs`: `*`, `:(top)` gibi desenler yorumlanmaz; yol
    // yalnızca tam bir dosya/klasör adı olarak ele alınır.
    // `-A`: silinmiş dosyaları da sahneye alır.
    c.args(["--literal-pathspecs", "add", "-A", "--", path]);
    let out = run(c)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(fail(&out))
    }
}

fn has_head(root: &Path) -> bool {
    let mut c = git(root);
    c.args(["rev-parse", "--verify", "--quiet", "HEAD"]);
    run(c).map(|o| o.status.success()).unwrap_or(false)
}

/// `path` için sahnelenmiş değişikliği geri al (worktree'ye dokunmaz).
pub fn unstage(root: &Path, path: &str) -> Result<()> {
    let mut c = git(root);
    c.arg("--literal-pathspecs");
    if has_head(root) {
        c.args(["reset", "-q", "--", path]);
    } else {
        // Henüz commit yok: HEAD'e göre reset mümkün değil.
        c.args(["rm", "--cached", "-r", "-q", "--ignore-unmatch", "--", path]);
    }
    let out = run(c)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(fail(&out))
    }
}

fn staged_anything(root: &Path) -> Result<bool> {
    let mut c = git(root);
    c.args(["diff", "--cached", "--quiet"]);
    let out = run(c)?;
    // 0: fark yok, 1: fark var, diğerleri: hata.
    match out.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(fail(&out)),
    }
}

fn identity_configured(root: &Path) -> bool {
    let has = |key: &str| {
        let mut c = git(root);
        c.args(["config", "--get", key]);
        run(c)
            .map(|o| o.status.success() && !o.stdout.iter().all(|b| b.is_ascii_whitespace()))
            .unwrap_or(false)
    };
    has("user.name") && has("user.email")
}

/// Sahnelenmiş değişikliklerle commit oluştur.
pub fn commit(root: &Path, message: &str) -> Result<()> {
    let message = message.trim();
    if message.is_empty() {
        return Err(WorkspaceError::Git("Commit mesajı boş olamaz.".into()));
    }
    if message.len() > MAX_MESSAGE_BYTES {
        return Err(WorkspaceError::Git("Commit mesajı çok uzun.".into()));
    }
    if !staged_anything(root)? {
        return Err(WorkspaceError::Git(
            "Commit edilecek hazırlanmış (staged) değişiklik yok.".into(),
        ));
    }

    let mut c = git(root);
    // Ortam temizlendiği için global kimlik okunamaz; depoda kimlik yoksa
    // yalnızca bu komut için varsayılan kimlik kullanılır (yapılandırma
    // dosyasına yazılmaz).
    if !identity_configured(root) {
        c.args([
            "-c",
            "user.name=Aetheros",
            "-c",
            "user.email=aetheros@localhost",
        ]);
    }
    c.args([
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--no-verify",
        "-m",
        message,
    ]);
    let out = run(c)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(fail(&out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    fn init_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let ok = |args: &[&str]| {
            let s = Command::new("git")
                .current_dir(dir.path())
                .args(args)
                .status()
                .expect("git");
            assert!(s.success(), "git {args:?}");
        };
        ok(&["init", "-q"]);
        dir
    }

    #[test]
    fn stage_unstage_and_commit_roundtrip() {
        if !git_available() {
            return;
        }
        let dir = init_repo();
        let root = dir.path();
        fs::write(root.join("a.txt"), "merhaba\n").unwrap();

        stage(root, "a.txt").unwrap();
        assert!(staged_anything(root).unwrap());

        // İlk commit öncesi unstage (HEAD yok yolu).
        unstage(root, "a.txt").unwrap();
        assert!(!staged_anything(root).unwrap());

        stage(root, "a.txt").unwrap();
        commit(root, "ilk commit").unwrap();
        assert!(!staged_anything(root).unwrap());

        // Commit sonrası değişiklik → stage → unstage (HEAD var yolu).
        fs::write(root.join("a.txt"), "değişti\n").unwrap();
        stage(root, "a.txt").unwrap();
        assert!(staged_anything(root).unwrap());
        unstage(root, "a.txt").unwrap();
        assert!(!staged_anything(root).unwrap());
    }

    #[test]
    fn stage_handles_deleted_file() {
        if !git_available() {
            return;
        }
        let dir = init_repo();
        let root = dir.path();
        fs::write(root.join("b.txt"), "x\n").unwrap();
        stage(root, "b.txt").unwrap();
        commit(root, "ekle").unwrap();

        fs::remove_file(root.join("b.txt")).unwrap();
        stage(root, "b.txt").unwrap();
        assert!(staged_anything(root).unwrap());
    }

    #[test]
    fn commit_rejects_empty_message_and_empty_index() {
        if !git_available() {
            return;
        }
        let dir = init_repo();
        let root = dir.path();
        assert!(commit(root, "   ").is_err());
        // Hiçbir şey sahnelenmedi.
        let err = commit(root, "bos").unwrap_err().to_string();
        assert!(err.contains("staged"), "{err}");
    }

    #[test]
    fn glob_patterns_are_not_expanded() {
        if !git_available() {
            return;
        }
        let dir = init_repo();
        let root = dir.path();
        fs::write(root.join("a.txt"), "a\n").unwrap();
        fs::write(root.join("b.txt"), "b\n").unwrap();
        // Desen olarak yorumlansaydı iki dosya da sahnelenirdi.
        assert!(stage(root, "*.txt").is_err());
        assert!(!staged_anything(root).unwrap());
    }

    #[test]
    fn dash_prefixed_filename_is_not_an_option() {
        if !git_available() {
            return;
        }
        let dir = init_repo();
        let root = dir.path();
        fs::write(root.join("-rf"), "x\n").unwrap();
        stage(root, "-rf").unwrap();
        assert!(staged_anything(root).unwrap());
    }
}
