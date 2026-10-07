// ============================================================
// src/agents/user_terminal.rs
//
// Faz 2 — Kullanıcı Terminali: kullanıcının KENDİ yazdığı tek satırlık
// komut, agent'larla AYNI politika motorundan (security::command_policy)
// geçer.
//
//   Deny  → reddedilir, çalışmaz (örn. `sh -c ...`, `rm -rf /`)
//   Allow → doğrudan çalışır (örn. `ls`, `git status`)
//   Ask   → `confirmed == true` verilmedikçe ÇALIŞMAZ. Onay kutusu
//           Flutter tarafındadır; Rust yine de kendi başına zorlar
//           (UI atlansa bile onaysız Ask komutu çalışmaz).
//
// Shell YOK: satır, tırnakları dikkate alan basit bir ayrıştırıcıyla
// argv'ye bölünür; `&&`, `;`, `|`, `>` gibi karakterler düz argüman
// olarak programa gider (enjeksiyon yüzeyi yok).
//
// Her çalıştırma denetim izine yazılır (agent_id = nil UUID →
// "kullanıcı terminali"; execution_id = komut başına yeni UUID).
// ============================================================

use uuid::Uuid;

use crate::agents::terminal_tool::TerminalAgentTool;
use crate::logging::audit::{summarize_output, AuditEventKind, AuditLog};
use crate::types::agent_tool::{AgentTool, CallVerdict};

/// Kullanıcı terminalinin denetim izindeki sabit agent kimliği.
pub const USER_TERMINAL_AGENT_ID: Uuid = Uuid::nil();

/// Arayüze dönen çıktının üst sınırı (karakter). Uzun çıktı kırpılır.
pub const MAX_OUTPUT_CHARS: usize = 20_000;

/// Tek satırı argv'ye böler. Tek/çift tırnak ve `\` kaçışı desteklenir.
/// Tırnak içinde boşluk korunur; boş tırnak (`""`) boş argüman üretir.
/// Kapanmamış tırnak → hata (yarım komutu sessizce çalıştırmak yerine).
pub fn split_command_line(line: &str) -> Result<Vec<String>, String> {
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut started = false; // "" gibi boş argümanı ayırt etmek için
    let mut single = false;
    let mut double = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if single {
            if c == '\'' {
                single = false;
            } else {
                cur.push(c);
            }
            continue;
        }
        if double {
            match c {
                '"' => double = false,
                '\\' => match chars.peek().copied() {
                    Some(n @ ('"' | '\\')) => {
                        cur.push(n);
                        chars.next();
                    }
                    _ => cur.push('\\'),
                },
                _ => cur.push(c),
            }
            continue;
        }
        match c {
            '\'' => {
                single = true;
                started = true;
            }
            '"' => {
                double = true;
                started = true;
            }
            '\\' => match chars.next() {
                Some(n) => {
                    cur.push(n);
                    started = true;
                }
                None => {
                    cur.push('\\');
                    started = true;
                }
            },
            c if c.is_whitespace() => {
                if started {
                    args.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }

    if single || double {
        return Err("kapanmamış tırnak: komutu tamamlayıp tekrar dene".to_string());
    }
    if started {
        args.push(cur);
    }
    Ok(args)
}

/// `check` sonucu: ayrıştırılmış argv + politika kararı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub argv: Vec<String>,
    pub verdict: CallVerdict,
}

/// Komutu ÇALIŞTIRMADAN ayrıştırır ve politikaya sorar.
pub fn check(tool: &TerminalAgentTool, line: &str) -> Result<CheckResult, String> {
    let argv = split_command_line(line)?;
    if argv.is_empty() {
        return Err("komut boş".to_string());
    }
    let verdict = tool
        .assess_call(&argv)
        .unwrap_or_else(|| CallVerdict::Ask {
            reason: "politika kararı yok — insan onayı gerekli".to_string(),
        });
    Ok(CheckResult { argv, verdict })
}

/// Çalıştırma sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    /// Komut 0 koduyla bitti mi?
    pub success: bool,
    /// Başarıda stdout; başarısızlıkta çıkış kodu + stderr.
    pub output: String,
    /// Çıktı `MAX_OUTPUT_CHARS`'a kırpıldı mı?
    pub truncated: bool,
}

fn clip_output(s: String) -> (String, bool) {
    if s.chars().count() <= MAX_OUTPUT_CHARS {
        return (s, false);
    }
    (s.chars().take(MAX_OUTPUT_CHARS).collect(), true)
}

/// Komutu politikaya göre çalıştırır ve denetim izine yazar.
///
/// - Deny → `Err` (çalışmaz, iz bırakır)
/// - Ask ve `confirmed == false` → `Err` (çalışmaz, iz bırakmaz: henüz
///   karar verilmedi; UI onay sorup `confirmed: true` ile tekrar çağırır)
/// - Allow, ya da Ask + `confirmed` → çalışır
pub async fn run(
    tool: &TerminalAgentTool,
    audit: &AuditLog,
    line: &str,
    confirmed: bool,
) -> Result<RunResult, String> {
    let CheckResult { argv, verdict } = check(tool, line)?;
    let execution_id = Uuid::new_v4();

    let (decision, reason) = match &verdict {
        CallVerdict::Allow => ("allow", None),
        CallVerdict::Ask { reason } => ("requires_approval", Some(reason.clone())),
        CallVerdict::Deny { reason } => ("deny", Some(reason.clone())),
    };

    match &verdict {
        CallVerdict::Deny { reason } => {
            audit.record(
                USER_TERMINAL_AGENT_ID,
                execution_id,
                AuditEventKind::GovernorDecision {
                    tool_name: "terminal".to_string(),
                    decision: decision.to_string(),
                    reason: Some(reason.clone()),
                    arguments: argv,
                },
            );
            return Err(format!("reddedildi: {reason}"));
        }
        CallVerdict::Ask { reason } if !confirmed => {
            return Err(format!("onay gerekli: {reason}"));
        }
        _ => {}
    }

    audit.record(
        USER_TERMINAL_AGENT_ID,
        execution_id,
        AuditEventKind::GovernorDecision {
            tool_name: "terminal".to_string(),
            decision: decision.to_string(),
            reason,
            arguments: argv.clone(),
        },
    );

    let result = tool.invoke(argv.clone()).await;

    let (success, text, error, summary) = match result {
        Ok(out) => {
            let summary = summarize_output(&out);
            (true, out, None, Some(summary))
        }
        Err(e) => (false, e.clone(), Some(e), None),
    };

    audit.record(
        USER_TERMINAL_AGENT_ID,
        execution_id,
        AuditEventKind::ToolInvoked {
            tool_name: "terminal".to_string(),
            success,
            error,
            arguments: argv,
            output: summary,
        },
    );

    let (output, truncated) = clip_output(text);
    Ok(RunResult { success, output, truncated })
}
