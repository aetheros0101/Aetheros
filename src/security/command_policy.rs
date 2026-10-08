// ============================================================
// src/security/command_policy.rs
//
// B4: ARGÜMAN-BAZLI komut politikası.
//
// "Shell yok" savunması yalnızca sözdizimini kapatır; `sh -c "..."`,
// `python -c "..."`, `git -c alias.x=!...`, `find -exec`, `env sh`
// hâlâ keyfi kod çalıştırır. Bu modül, terminal aracının çağrısını
// ARGÜMANLARIYLA değerlendirip üç sınıftan birini döndürür:
//
//   Deny  — kesin ret (onayla bile çalışmaz)
//   Allow — açık bir izin kuralı var: onay istemeden çalışır
//   Ask   — varsayılan: insan onayı
//
// Öncelik: Deny > Allow > Ask. Bilinmeyen her şey Ask'tır (fail-safe).
//
// Politika VERİDİR: yerleşik varsayılan aşağıda; isteğe bağlı olarak
// `AETHEROS_COMMAND_POLICY` ortam değişkeniyle bir JSON dosyası
// gösterilebilir (varsayılanı TAMAMEN yerine geçer). Dosya okunamaz /
// geçersizse yerleşik varsayılan kullanılır ve uyarı loglanır.
//
// BİLİNEN SINIRLAR (dürüstçe):
//   - Yol-bazlı kurallar yok (`cat ~/.ssh/id_rsa` Ask'tır ama içerik
//     analizi yapılmaz). Workspace kök hapsi (B9/Faz 1b) ayrı iş.
//   - Allow kuralları yalnızca "çıplak" program adı için geçerlidir
//     (`git`); `./git` veya `/tmp/x/git` Allow olmaz (PATH dışı ikili).
//   - Deny listesi bir kara listedir: tam kapsayıcı değildir. Asıl
//     güvenlik varsayılanın Ask olmasından gelir.
// ============================================================

use std::path::Path;

use serde::Deserialize;
use tracing::warn;

pub use crate::types::agent_tool::CallVerdict;

/// Politika dosyasını gösteren ortam değişkeni.
pub const POLICY_ENV: &str = "AETHEROS_COMMAND_POLICY";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DenyRule {
    pub reason: String,
    /// Program adı (basename, küçük harf) bunlardan biriyse eşleşir.
    pub programs: Vec<String>,
    /// Program adı bu önekle başlıyorsa eşleşir (örn. "mkfs.").
    pub programs_prefix: Vec<String>,
    /// Argümanlardan biri BİREBİR bunlardan biriyse eşleşir.
    pub args_any: Vec<String>,
    /// Argümanlardan biri bu öneklerden biriyle başlıyorsa eşleşir.
    pub args_prefix_any: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AllowRule {
    pub programs: Vec<String>,
    /// Boş değilse: `arguments[1]` (program'ın hemen ardındaki argüman)
    /// bunlardan biri OLMALI. Global seçeneklere (`git -C x status`)
    /// bilerek izin verilmez — alt-komut yerine değer yakalanabilir.
    pub subcommands: Vec<String>,
    /// Bu öneklerden biriyle başlayan bir argüman varsa kural eşleşmez
    /// (karar Ask'a düşer). Örn. `git diff --output=dosya`.
    pub unless_args_prefix: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CommandPolicy {
    pub deny: Vec<DenyRule>,
    pub allow: Vec<AllowRule>,
}

fn basename(p: &str) -> &str {
    // Hem '/' hem '\\' ayırıcı sayılır (Android/Linux + olası Windows yolu).
    p.rsplit(['/', '\\']).next().unwrap_or(p)
}

/// "python3.11" → "python", "Bash.EXE" → "bash". Sürüm sonekleri ve
/// uzantı, kara liste kaçırmasın diye atılır.
fn normalized(program: &str) -> String {
    let lower = basename(program).to_ascii_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    stem.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')
        .to_string()
}

fn has_path_separator(program: &str) -> bool {
    program.contains('/') || program.contains('\\')
}

impl DenyRule {
    fn matches(&self, base_lower: &str, norm: &str, args: &[String]) -> bool {
        let program_cond = !self.programs.is_empty() || !self.programs_prefix.is_empty();
        let args_cond = !self.args_any.is_empty() || !self.args_prefix_any.is_empty();
        // Koşulsuz kural "her şeyi reddet" anlamına gelirdi — yazım hatası
        // sonucu olabilir; yok sayılır.
        if !program_cond && !args_cond {
            return false;
        }

        let prog_hit = !program_cond
            || self
                .programs
                .iter()
                .any(|p| p.eq_ignore_ascii_case(base_lower) || p.eq_ignore_ascii_case(norm))
            || self
                .programs_prefix
                .iter()
                .any(|p| base_lower.starts_with(&p.to_ascii_lowercase()));

        let args_hit = !args_cond
            || args.iter().any(|a| {
                self.args_any.iter().any(|x| x == a)
                    || self
                        .args_prefix_any
                        .iter()
                        .any(|x| a.starts_with(x.as_str()))
            });

        prog_hit && args_hit
    }
}

impl AllowRule {
    fn matches(&self, program: &str, args: &[String]) -> bool {
        // PATH dışı (yol içeren) ikili asla otomatik izin almaz.
        if has_path_separator(program) {
            return false;
        }
        if !self.programs.iter().any(|p| p == program) {
            return false;
        }
        if args.iter().any(|a| {
            self.unless_args_prefix
                .iter()
                .any(|x| a.starts_with(x.as_str()))
        }) {
            return false;
        }
        if self.subcommands.is_empty() {
            return true;
        }
        match args.first() {
            Some(sub) => self.subcommands.iter().any(|s| s == sub),
            None => false,
        }
    }
}

impl CommandPolicy {
    /// `arguments[0]` = program, geri kalanı onun argümanları
    /// (terminal_tool sözleşmesi).
    pub fn assess(&self, arguments: &[String]) -> CallVerdict {
        let Some(program) = arguments.first() else {
            return CallVerdict::Deny {
                reason: "program belirtilmedi".into(),
            };
        };
        if program.trim().is_empty() {
            return CallVerdict::Deny {
                reason: "program adı boş".into(),
            };
        }
        // Boşluk içeren program adı, neredeyse kesinlikle yanlış biçimli bir
        // komut satırıdır (["touch deneme.txt"]). Onaya düşürmek yerine
        // net bir gerekçeyle reddedilir — ajan bu hatayı görüp düzeltir,
        // kullanıcı anlamsız bir "komutu" onaylamak zorunda kalmaz.
        if program.chars().any(char::is_whitespace) {
            return CallVerdict::Deny {
                reason: "program adı boşluk içeremez: komutu ayrı argümanlarla ver, \
                         örn. [\"touch\", \"deneme.txt\"] (tek bir komut satırı string'i değil)"
                    .into(),
            };
        }
        let args = &arguments[1..];
        let base_lower = basename(program).to_ascii_lowercase();
        let norm = normalized(program);

        // 1) Deny her zaman kazanır.
        for rule in &self.deny {
            if rule.matches(&base_lower, &norm, args) {
                let reason = if rule.reason.is_empty() {
                    "politika tarafından yasaklandı".to_string()
                } else {
                    rule.reason.clone()
                };
                return CallVerdict::Deny {
                    reason: format!("{reason} ('{}')", basename(program)),
                };
            }
        }

        // 2) Açık allow kuralı → otomatik izin.
        let prog_exact = program.as_str();
        if self.allow.iter().any(|r| r.matches(prog_exact, args)) {
            return CallVerdict::Allow;
        }

        // 3) Hiçbiri → insan onayı.
        CallVerdict::Ask {
            reason: format!(
                "'{}' için açık bir izin kuralı yok — insan onayı gerekli",
                basename(program)
            ),
        }
    }

    pub fn from_json_str(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("komut politikası JSON'u geçersiz: {e}"))
    }

    /// `AETHEROS_COMMAND_POLICY` varsa o dosyayı, yoksa/bozuksa yerleşik
    /// varsayılanı döndürür. Hata ASLA politikayı gevşetmez: bozuk dosya
    /// → varsayılan (sıkı) politika.
    pub fn load() -> Self {
        let Ok(path) = std::env::var(POLICY_ENV) else {
            return Self::default_policy();
        };
        if path.trim().is_empty() {
            return Self::default_policy();
        }
        match std::fs::read_to_string(Path::new(&path))
            .map_err(|e| e.to_string())
            .and_then(|s| Self::from_json_str(&s))
        {
            Ok(p) => p,
            Err(e) => {
                warn!(path = %path, error = %e, "komut politikası yüklenemedi; yerleşik varsayılan kullanılıyor");
                Self::default_policy()
            }
        }
    }

    /// Yerleşik, muhafazakâr varsayılan.
    pub fn default_policy() -> Self {
        fn v(items: &[&str]) -> Vec<String> {
            items.iter().map(|s| s.to_string()).collect()
        }

        let deny = vec![
            DenyRule {
                reason: "kabuk/yorumlayıcı keyfi kod çalıştırır".into(),
                programs: v(&[
                    "sh",
                    "bash",
                    "zsh",
                    "dash",
                    "ash",
                    "ksh",
                    "csh",
                    "tcsh",
                    "fish",
                    "busybox",
                    "python",
                    "perl",
                    "ruby",
                    "node",
                    "nodejs",
                    "deno",
                    "bun",
                    "php",
                    "lua",
                    "tclsh",
                    "powershell",
                    "pwsh",
                    "cmd",
                    "osascript",
                ]),
                ..Default::default()
            },
            DenyRule {
                reason: "başka bir programı çağırır (politikayı atlatır)".into(),
                programs: v(&[
                    "env", "xargs", "nohup", "nice", "ionice", "timeout", "stdbuf", "setsid",
                    "sudo", "su", "doas", "chroot", "nsenter", "unshare", "strace", "ltrace",
                    "watch", "time", "command", "exec", "eval", "awk", "gawk", "mawk", "nawk",
                ]),
                ..Default::default()
            },
            DenyRule {
                reason: "find ile komut çalıştırma/silme".into(),
                programs: v(&["find"]),
                args_any: v(&["-exec", "-execdir", "-ok", "-okdir", "-delete"]),
                ..Default::default()
            },
            DenyRule {
                reason: "git ile keyfi komut/yapılandırma enjeksiyonu".into(),
                programs: v(&["git"]),
                args_any: v(&["-c"]),
                args_prefix_any: v(&[
                    "--config-env",
                    "--exec-path",
                    "--upload-pack",
                    "--receive-pack",
                    "--exec=",
                ]),
                ..Default::default()
            },
            DenyRule {
                reason: "tar ile komut çalıştırma".into(),
                programs: v(&["tar"]),
                args_any: v(&["-I"]),
                args_prefix_any: v(&[
                    "--to-command",
                    "--checkpoint-action",
                    "--use-compress-program",
                ]),
                ..Default::default()
            },
            DenyRule {
                reason: "kök/ev dizinini hedefleyen silme".into(),
                programs: v(&["rm"]),
                args_any: v(&["/", "/*", "~", "~/", "$HOME"]),
                ..Default::default()
            },
            DenyRule {
                reason: "yıkıcı sistem komutu".into(),
                programs: v(&["dd", "mkfs", "shutdown", "reboot", "halt", "poweroff"]),
                programs_prefix: v(&["mkfs."]),
                ..Default::default()
            },
        ];

        let allow = vec![
            AllowRule {
                programs: v(&["git"]),
                subcommands: v(&["status", "diff", "log", "show", "rev-parse", "ls-files"]),
                // Dosyaya yazan / harici program çalıştıran okuma bayrakları
                unless_args_prefix: v(&[
                    "--output",
                    "--ext-diff",
                    "--textconv",
                    "--open-files-in-pager",
                ]),
            },
            AllowRule {
                programs: v(&["ls", "pwd", "echo", "date", "uname", "whoami"]),
                ..Default::default()
            },
        ];

        Self { deny, allow }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }
    fn p() -> CommandPolicy {
        CommandPolicy::default_policy()
    }
    fn is_deny(v: CallVerdict) -> bool {
        matches!(v, CallVerdict::Deny { .. })
    }
    fn is_ask(v: CallVerdict) -> bool {
        matches!(v, CallVerdict::Ask { .. })
    }

    #[test]
    fn exit_criteria_sh_c_denied_status_auto_commit_asks() {
        assert!(is_deny(p().assess(&a(&["sh", "-c", "id"]))));
        assert_eq!(p().assess(&a(&["git", "status"])), CallVerdict::Allow);
        assert!(is_ask(p().assess(&a(&["git", "commit", "-m", "x"]))));
    }

    #[test]
    fn shells_and_interpreters_denied_even_via_path_or_version() {
        for cmd in [
            vec!["/bin/sh", "-c", "id"],
            vec!["/usr/bin/bash"],
            vec!["python3", "-c", "print(1)"],
            vec!["python3.11", "x.py"],
            vec!["/usr/bin/perl", "-e", "1"],
            vec!["node", "-e", "1"],
            vec!["BASH.EXE"],
            vec!["..\\..\\windows\\cmd.exe"],
        ] {
            assert!(is_deny(p().assess(&a(&cmd))), "{cmd:?} reddedilmeliydi");
        }
    }

    #[test]
    fn wrappers_that_launch_other_programs_are_denied() {
        for cmd in [
            vec!["env", "sh"],
            vec!["xargs", "rm"],
            vec!["timeout", "5", "sh"],
            vec!["sudo", "ls"],
            vec!["awk", "BEGIN{system(\"id\")}"],
        ] {
            assert!(is_deny(p().assess(&a(&cmd))), "{cmd:?}");
        }
    }

    #[test]
    fn find_exec_denied_plain_find_asks() {
        assert!(is_deny(
            p().assess(&a(&["find", ".", "-exec", "sh", "{}", ";"]))
        ));
        assert!(is_deny(p().assess(&a(&["find", ".", "-delete"]))));
        assert!(is_ask(p().assess(&a(&["find", ".", "-name", "*.rs"]))));
    }

    #[test]
    fn git_config_injection_denied() {
        assert!(is_deny(p().assess(&a(&["git", "-c", "alias.x=!sh", "x"]))));
        assert!(is_deny(p().assess(&a(&[
            "git",
            "status",
            "--exec-path=/tmp"
        ]))));
        assert!(is_deny(p().assess(&a(&[
            "git",
            "fetch",
            "--upload-pack=evil"
        ]))));
    }

    #[test]
    fn git_readonly_subcommands_auto_allowed() {
        for sub in ["status", "diff", "log", "show", "rev-parse", "ls-files"] {
            assert_eq!(p().assess(&a(&["git", sub])), CallVerdict::Allow, "{sub}");
        }
        assert_eq!(
            p().assess(&a(&["git", "diff", "--stat", "HEAD~1"])),
            CallVerdict::Allow
        );
    }

    #[test]
    fn git_mutating_or_unknown_subcommands_ask() {
        for sub in [
            "commit", "push", "reset", "checkout", "clean", "branch", "config",
        ] {
            assert!(is_ask(p().assess(&a(&["git", sub]))), "{sub}");
        }
    }

    #[test]
    fn git_global_option_before_subcommand_cannot_smuggle_an_allowed_name() {
        // `git -C status commit`: git "status" dizinine gidip `commit` çalıştırır.
        // Alt-komut arguments[1] olmak zorunda → Allow OLMAZ.
        assert!(is_ask(p().assess(&a(&["git", "-C", "status", "commit"]))));
        assert!(is_ask(p().assess(&a(&["git", "--no-pager", "status"]))));
    }

    #[test]
    fn git_output_flags_fall_back_to_ask() {
        assert!(is_ask(p().assess(&a(&["git", "diff", "--output=/tmp/x"]))));
        assert!(is_ask(p().assess(&a(&["git", "log", "--ext-diff"]))));
    }

    #[test]
    fn path_qualified_binary_never_auto_allowed() {
        assert!(is_ask(p().assess(&a(&["./git", "status"]))));
        assert!(is_ask(p().assess(&a(&["/tmp/evil/ls"]))));
    }

    #[test]
    fn simple_readonly_programs_allowed() {
        for cmd in [
            vec!["ls", "-la"],
            vec!["pwd"],
            vec!["echo", "merhaba"],
            vec!["date"],
        ] {
            assert_eq!(p().assess(&a(&cmd)), CallVerdict::Allow, "{cmd:?}");
        }
    }

    #[test]
    fn unknown_program_asks_by_default() {
        assert!(is_ask(p().assess(&a(&["curl", "https://example.com"]))));
        assert!(is_ask(p().assess(&a(&["cat", "/etc/passwd"]))));
    }

    #[test]
    fn destructive_commands_denied() {
        assert!(is_deny(p().assess(&a(&["rm", "-rf", "/"]))));
        assert!(is_deny(p().assess(&a(&["rm", "-rf", "~"]))));
        assert!(is_deny(p().assess(&a(&[
            "dd",
            "if=/dev/zero",
            "of=/dev/sda"
        ]))));
        assert!(is_deny(p().assess(&a(&["mkfs.ext4", "/dev/sda1"]))));
        // Hedefsiz rm → insan karar versin
        assert!(is_ask(p().assess(&a(&["rm", "dosya.txt"]))));
    }

    #[test]
    fn whole_command_line_in_one_argument_is_denied_with_guidance() {
        match p().assess(&a(&["touch deneme.txt"])) {
            CallVerdict::Deny { reason } => {
                assert!(reason.contains("ayrı argümanlarla"), "{reason}")
            }
            other => panic!("{other:?}"),
        }
        assert!(is_deny(p().assess(&a(&["git status"]))));
        assert!(is_deny(p().assess(&a(&["ls -la", "/tmp"]))));
        // Doğru biçim bozulmadı
        assert!(is_ask(p().assess(&a(&["touch", "deneme.txt"]))));
    }

    #[test]
    fn empty_or_blank_program_denied() {
        assert!(is_deny(p().assess(&[])));
        assert!(is_deny(p().assess(&a(&[""]))));
        assert!(is_deny(p().assess(&a(&["   "]))));
    }

    #[test]
    fn deny_beats_allow() {
        let policy = CommandPolicy {
            deny: vec![DenyRule {
                reason: "x".into(),
                programs: vec!["git".into()],
                args_any: vec!["push".into()],
                ..Default::default()
            }],
            allow: vec![AllowRule {
                programs: vec!["git".into()],
                ..Default::default()
            }],
        };
        assert_eq!(policy.assess(&a(&["git", "status"])), CallVerdict::Allow);
        assert!(is_deny(policy.assess(&a(&["git", "push"]))));
    }

    #[test]
    fn conditionless_deny_rule_is_ignored_not_deny_all() {
        let policy = CommandPolicy {
            deny: vec![DenyRule {
                reason: "hatalı".into(),
                ..Default::default()
            }],
            allow: vec![],
        };
        assert!(is_ask(policy.assess(&a(&["ls"]))));
    }

    #[test]
    fn json_policy_parses_and_partial_fields_default() {
        let json = r#"{
            "deny":  [ { "reason": "no curl", "programs": ["curl"] } ],
            "allow": [ { "programs": ["cargo"], "subcommands": ["check", "test"] } ]
        }"#;
        let policy = CommandPolicy::from_json_str(json).unwrap();
        assert!(is_deny(policy.assess(&a(&["curl", "x"]))));
        assert_eq!(policy.assess(&a(&["cargo", "test"])), CallVerdict::Allow);
        assert!(is_ask(policy.assess(&a(&["cargo", "publish"]))));
        // Özel politika varsayılanı TAMAMEN yerine geçer: sh artık Ask.
        assert!(is_ask(policy.assess(&a(&["sh"]))));
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(CommandPolicy::from_json_str("{ bozuk").is_err());
    }

    #[test]
    fn deny_reason_names_the_program() {
        match p().assess(&a(&["/bin/sh", "-c", "id"])) {
            CallVerdict::Deny { reason } => assert!(reason.contains("'sh'"), "{reason}"),
            other => panic!("{other:?}"),
        }
    }
}
