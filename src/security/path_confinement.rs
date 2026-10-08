// ============================================================
// src/security/path_confinement.rs
//
// B9: Terminal komutlarının YOL argümanlarını workspace köküne hapseder.
//
// Shell yok → argümanlar programa düz metin olarak gider; yani bir yol
// argümanını (`/etc/passwd`, `../../x`, `link/içeri`) biz görebiliriz.
// Bu modül, yol gibi görünen her argümanı workspace köküne göre çözer ve
// kökün DIŞINA çıkıyorsa reddeder. Çözümleme bileşen bileşen yürünür:
// her sembolik bağ gerçek hedefine çevrilir ve hedef kök içinde kalmalı
// (`link/../x` gibi sözdizimsel kaçışlar da yakalanır).
//
// BİLİNEN SINIRLAR (dürüstçe):
//   - Kapsam yalnızca ARGÜMANLAR. Bir program kendi başına (yapılandırma
//     dosyası, ağ, `~` genişletmesi, stdin) dışarıdaki dosyaları açabilir;
//     bunu sürece sistem düzeyinde hapis (chroot/namespace/Android sandbox)
//     olmadan engelleyemeyiz. Bu katman "yanlışlıkla/kasıtlı dış yol
//     argümanı"na karşı bir savunma katmanıdır, tam bir hapis değildir.
//   - `-I/yol` gibi bayrağa yapışık değerler ve `-o yol` gibi bayraktan
//     SONRA gelen ayrı değerler yol sayılırsa denetlenir; yapışık kısa
//     bayrak değerleri denetlenmez.
//   - Çözümleme ile çalıştırma arasında dosya sistemi değişebilir
//     (TOCTOU); bu yüzden kontrol çalıştırma anında yeniden yapılır.
// ============================================================

use std::path::{Component, Path, PathBuf};

/// Dosya açmayan, argümanı yalnızca yazdıran/okuyan programlar: `echo /etc`
/// zararsızdır, yol denetimi yanlış-pozitif üretmesin.
const NO_FILE_PROGRAMS: &[&str] = &["echo", "date", "uname", "whoami", "pwd"];

/// `git commit -m "fix a/b"` gibi serbest metin alan bayraklar.
const MESSAGE_FLAGS: &[&str] = &["-m", "--message"];

fn outside(arg: &str) -> String {
    format!(
        "'{arg}' çalışma alanının dışına çıkıyor — komutlar yalnızca workspace içindeki \
         yolları kullanabilir (göreli yol kullan, örn. notlar/a.txt)"
    )
}

fn looks_like_path(s: &str) -> bool {
    s == ".." || s == "." || s.starts_with('~') || s.contains('/')
}

/// `path`'i workspace köküne göre çözer; kök dışına çıkıyorsa `Err`.
fn resolve_inside(root: &Path, canonical_root: &Path, path: &str) -> Result<PathBuf, String> {
    if path.starts_with('~') {
        return Err(format!(
            "'{path}' ev dizinine işaret edebilir — workspace içi göreli yol kullan"
        ));
    }
    let p = Path::new(path);
    let relative: PathBuf = if p.is_absolute() {
        p.strip_prefix(root)
            .or_else(|_| p.strip_prefix(canonical_root))
            .map(|r| r.to_path_buf())
            .map_err(|_| outside(path))?
    } else {
        p.to_path_buf()
    };

    let mut cur = canonical_root.to_path_buf();
    for comp in relative.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !cur.pop() || !cur.starts_with(canonical_root) {
                    return Err(outside(path));
                }
            }
            Component::Normal(name) => {
                cur.push(name);
                if let Ok(meta) = std::fs::symlink_metadata(&cur)
                    && meta.file_type().is_symlink()
                {
                    // Kırık/okunamayan bağ: hedefi bilinmez → reddet.
                    cur = std::fs::canonicalize(&cur).map_err(|_| outside(path))?;
                    if !cur.starts_with(canonical_root) {
                        return Err(outside(path));
                    }
                }
            }
            // Göreli yolda olmaması gerekir (mutlak yol yukarıda soyuldu).
            Component::RootDir | Component::Prefix(_) => return Err(outside(path)),
        }
    }
    Ok(cur)
}

/// Terminal çağrısının (`arguments[0]` = program) yol argümanlarını denetler.
pub fn check_arguments(root: &Path, arguments: &[String]) -> Result<(), String> {
    let Some(program) = arguments.first() else {
        return Ok(());
    };
    let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    // Yol içeren program (./betik, /bin/ls) da kök içinde olmalı.
    if looks_like_path(program) {
        resolve_inside(root, &canonical_root, program)?;
    }
    if NO_FILE_PROGRAMS.contains(&program.as_str()) {
        return Ok(());
    }

    let is_git = program == "git";
    let mut skip_next = false;
    for arg in &arguments[1..] {
        if skip_next {
            skip_next = false;
            continue;
        }
        if is_git && MESSAGE_FLAGS.contains(&arg.as_str()) {
            skip_next = true; // sonraki argüman commit mesajı (serbest metin)
            continue;
        }
        let candidate: &str = if arg.starts_with('-') {
            // `--output=/yol` → '=' sonrası değer denetlenir; çıplak bayrak değil.
            match arg.find('=') {
                Some(i) => &arg[i + 1..],
                None => continue,
            }
        } else {
            arg.as_str()
        };
        // Çıplak isimler de çözülür: workspace içindeki `kacis` adlı bir symlink
        // dışarı işaret ediyorsa `touch kacis` dışarıda dosya yaratırdı.
        if !candidate.is_empty() {
            resolve_inside(root, &canonical_root, candidate)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }

    fn temp_ws() -> PathBuf {
        let d = std::env::temp_dir().join(format!("aeth_conf_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(d.join("alt")).unwrap();
        d
    }

    #[test]
    fn plain_and_relative_paths_inside_are_ok() {
        let ws = temp_ws();
        for cmd in [
            a(&["ls"]),
            a(&["ls", "-la"]),
            a(&["touch", "a.txt"]),
            a(&["cat", "alt/b.txt"]),
            a(&["cat", "./alt/../a.txt"]),
            a(&["ls", "."]),
            a(&["git", "log", "--oneline"]),
        ] {
            assert!(check_arguments(&ws, &cmd).is_ok(), "{cmd:?}");
        }
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn absolute_paths_outside_and_parent_escapes_are_rejected() {
        let ws = temp_ws();
        for cmd in [
            a(&["cat", "/etc/passwd"]),
            a(&["ls", "/"]),
            a(&["cat", "../x"]),
            a(&["cat", "alt/../../x"]),
            a(&["ls", ".."]),
            a(&["cat", "~/.ssh/id_rsa"]),
            a(&["cat", "~"]),
            a(&["/bin/ls"]),
        ] {
            assert!(
                check_arguments(&ws, &cmd).is_err(),
                "{cmd:?} reddedilmeliydi"
            );
        }
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn absolute_path_inside_workspace_is_ok() {
        let ws = temp_ws();
        let inside = ws.join("alt").join("c.txt").display().to_string();
        assert!(check_arguments(&ws, &a(&["cat", &inside])).is_ok());
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn flag_values_after_equals_are_checked_but_bare_flags_are_not() {
        let ws = temp_ws();
        assert!(check_arguments(&ws, &a(&["git", "diff", "--output=/tmp/x"])).is_err());
        assert!(check_arguments(&ws, &a(&["git", "diff", "--output=alt/x"])).is_ok());
        assert!(check_arguments(&ws, &a(&["ls", "-la", "--color=auto"])).is_ok());
        assert!(check_arguments(&ws, &a(&["git", "status", "--short"])).is_ok());
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn echo_and_git_message_are_free_text() {
        let ws = temp_ws();
        assert!(check_arguments(&ws, &a(&["echo", "/etc/passwd"])).is_ok());
        assert!(
            check_arguments(&ws, &a(&["git", "commit", "-m", "/etc ve ../x düzeltildi"])).is_ok()
        );
        assert!(check_arguments(&ws, &a(&["git", "commit", "--message", "a/b"])).is_ok());
        // ama -m'den sonraki SONRAKİ argüman yine denetlenir
        assert!(check_arguments(&ws, &a(&["git", "commit", "-m", "x", "/etc/passwd"])).is_err());
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escaping_the_workspace_is_rejected() {
        let ws = temp_ws();
        let outside_dir = std::env::temp_dir().join(format!("aeth_out_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::os::unix::fs::symlink(&outside_dir, ws.join("kacis")).unwrap();
        std::os::unix::fs::symlink(&outside_dir, ws.join("alt").join("kacis2")).unwrap();
        // kırık bağ (yazma ile dışarıda dosya yaratabilirdi)
        std::os::unix::fs::symlink(outside_dir.join("yok"), ws.join("kirik")).unwrap();

        for cmd in [
            a(&["cat", "kacis/x"]),
            a(&["cat", "alt/kacis2/x"]),
            a(&["touch", "kacis"]),
            a(&["cat", "kacis"]),
            a(&["touch", "kirik"]),
            // link/.. sözdizimsel olarak içeride görünür ama gerçekte dışarı çıkar
            a(&["cat", "kacis/../a.txt"]),
        ] {
            assert!(
                check_arguments(&ws, &cmd).is_err(),
                "{cmd:?} reddedilmeliydi"
            );
        }
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&outside_dir);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_pointing_inside_the_workspace_is_ok() {
        let ws = temp_ws();
        std::os::unix::fs::symlink(ws.join("alt"), ws.join("kisayol")).unwrap();
        assert!(check_arguments(&ws, &a(&["ls", "kisayol/x"])).is_ok());
        let _ = std::fs::remove_dir_all(&ws);
    }
}
