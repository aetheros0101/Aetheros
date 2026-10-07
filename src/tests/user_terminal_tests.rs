// ============================================================
// src/tests/user_terminal_tests.rs
//
// Faz 2 — Kullanıcı Terminali: ayrıştırma, politika kararı, onay
// zorlaması ve denetim izi.
// ============================================================

use crate::agents::terminal_tool::{explain_spawn_failure, TerminalAgentTool};
use crate::agents::user_terminal::{
    check, run, split_command_line, MAX_OUTPUT_CHARS, USER_TERMINAL_AGENT_ID,
};
use crate::logging::audit::{AuditEventKind, AuditLog};
use crate::security::command_policy::CommandPolicy;
use crate::types::agent_tool::CallVerdict;

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// ── ayrıştırıcı (saf) ──

#[test]
fn splits_on_whitespace() {
    assert_eq!(split_command_line("touch deneme.txt").unwrap(), v(&["touch", "deneme.txt"]));
    assert_eq!(split_command_line("  ls   -la  ").unwrap(), v(&["ls", "-la"]));
    assert_eq!(split_command_line("").unwrap(), Vec::<String>::new());
    assert_eq!(split_command_line("   ").unwrap(), Vec::<String>::new());
}

#[test]
fn quotes_keep_spaces_and_empty_quotes_make_empty_args() {
    assert_eq!(
        split_command_line(r#"echo "merhaba dünya" 'tek tırnak'"#).unwrap(),
        v(&["echo", "merhaba dünya", "tek tırnak"])
    );
    assert_eq!(split_command_line(r#"echo "" x"#).unwrap(), v(&["echo", "", "x"]));
    assert_eq!(split_command_line(r#"a"b c"d"#).unwrap(), v(&["ab cd"]));
}

#[test]
fn escapes_work_outside_and_inside_double_quotes_but_not_single() {
    assert_eq!(split_command_line(r"echo a\ b").unwrap(), v(&["echo", "a b"]));
    assert_eq!(split_command_line(r#"echo "a\"b""#).unwrap(), v(&["echo", r#"a"b"#]));
    assert_eq!(split_command_line(r#"echo "a\nb""#).unwrap(), v(&["echo", r"a\nb"]));
    assert_eq!(split_command_line(r"echo 'a\b'").unwrap(), v(&["echo", r"a\b"]));
}

#[test]
fn unterminated_quote_is_an_error() {
    assert!(split_command_line(r#"echo "abc"#).is_err());
    assert!(split_command_line("echo 'abc").is_err());
}

#[test]
fn shell_metacharacters_stay_plain_arguments() {
    assert_eq!(
        split_command_line("ls && rm -rf / ; echo $(x) | cat > f").unwrap(),
        v(&["ls", "&&", "rm", "-rf", "/", ";", "echo", "$(x)", "|", "cat", ">", "f"])
    );
}

// ── politika kararı ──

fn tool() -> TerminalAgentTool {
    TerminalAgentTool::with_policy(CommandPolicy::default_policy())
}

#[test]
fn check_returns_the_policy_verdict_for_the_typed_line() {
    let t = tool();
    assert_eq!(check(&t, "ls").unwrap().verdict, CallVerdict::Allow);
    assert!(matches!(check(&t, "touch a.txt").unwrap().verdict, CallVerdict::Ask { .. }));
    assert!(matches!(check(&t, r#"sh -c "echo hi""#).unwrap().verdict, CallVerdict::Deny { .. }));
    assert_eq!(check(&t, "touch a.txt").unwrap().argv, v(&["touch", "a.txt"]));
    assert!(check(&t, "   ").is_err());
    assert!(check(&t, r#"echo "x"#).is_err());
}

// ── çalıştırma + onay zorlaması + denetim ──

#[tokio::test]
async fn deny_never_runs_and_is_audited() {
    let t = tool();
    let audit = AuditLog::new(100);
    let err = run(&t, &audit, r#"sh -c "echo hi""#, true).await.unwrap_err();
    assert!(err.contains("reddedildi"), "{err}");

    let events = audit.list();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].agent_id, USER_TERMINAL_AGENT_ID);
    assert!(matches!(
        &events[0].kind,
        AuditEventKind::GovernorDecision { decision, .. } if decision == "deny"
    ));
}

#[tokio::test]
async fn ask_without_confirmation_does_not_run_and_leaves_no_trace() {
    let t = tool();
    let audit = AuditLog::new(100);
    let err = run(&t, &audit, "touch asla_olusmamali.txt", false).await.unwrap_err();
    assert!(err.contains("onay gerekli"), "{err}");
    assert!(audit.list().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn allowed_command_runs_and_writes_two_audit_events() {
    let t = tool();
    let audit = AuditLog::new(100);
    let res = run(&t, &audit, "echo merhaba", false).await.unwrap();
    assert!(res.success);
    assert_eq!(res.output.trim(), "merhaba");
    assert!(!res.truncated);

    let events = audit.list();
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0].kind,
        AuditEventKind::GovernorDecision { decision, .. } if decision == "allow"
    ));
    assert!(matches!(
        &events[1].kind,
        AuditEventKind::ToolInvoked { success: true, output: Some(_), .. }
    ));
    assert_eq!(events[0].execution_id, events[1].execution_id);
}

#[cfg(unix)]
#[tokio::test]
async fn confirmed_ask_runs_inside_the_workspace() {
    let dir = std::env::temp_dir().join(format!("aeth_uterm_{}", uuid::Uuid::new_v4()));
    let t = TerminalAgentTool::with_workspace(dir.clone());
    // with_workspace varsayılan politikayı yükler; touch → Ask.
    let audit = AuditLog::new(100);

    assert!(run(&t, &audit, "touch onayli.txt", false).await.is_err());
    assert!(!dir.join("onayli.txt").exists(), "onaysız Ask çalışmamalı");

    let res = run(&t, &audit, "touch onayli.txt", true).await.unwrap();
    assert!(res.success);
    assert!(dir.join("onayli.txt").exists(), "onaylı komut workspace'te çalışmalı");

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[tokio::test]
async fn failing_command_reports_failure_not_an_error() {
    let t = tool();
    let audit = AuditLog::new(100);
    // `ls` olmayan bir yolda 0 dışı kodla biter → Ok(RunResult{success:false})
    let res = run(&t, &audit, "ls /bu/yol/yok", false).await.unwrap();
    assert!(!res.success);
    assert!(!res.output.is_empty());
    assert!(matches!(
        &audit.list().last().unwrap().kind,
        AuditEventKind::ToolInvoked { success: false, .. }
    ));
}

#[test]
fn max_output_constant_is_sane() {
    assert!(MAX_OUTPUT_CHARS >= 1_000);
}

// ── spawn hatası açıklaması ──

#[test]
fn spawn_failure_gets_a_plain_explanation() {
    let msg = explain_spawn_failure(
        "terminal",
        "failed to spawn process: Permission denied (os error 13)".to_string(),
    );
    assert!(msg.contains("'terminal' programı başlatılamadı"), "{msg}");
    assert!(msg.contains("doğal dil değil"), "{msg}");
    assert!(msg.contains("os error 13"), "ham ayrıntı korunmalı: {msg}");

    let nf = explain_spawn_failure(
        "xyz",
        "failed to spawn process: No such file or directory (os error 2)".to_string(),
    );
    assert!(nf.contains("'xyz' programı başlatılamadı"), "{nf}");
}

#[test]
fn other_errors_are_left_untouched() {
    let raw = "komut 1 koduyla başarısız oldu: boom".to_string();
    assert_eq!(explain_spawn_failure("ls", raw.clone()), raw);
    let other = "failed to spawn process: Argument list too long".to_string();
    assert_eq!(explain_spawn_failure("ls", other.clone()), other);
}

#[cfg(unix)]
#[tokio::test]
async fn nonexistent_program_reports_the_explained_failure() {
    let t = tool();
    let audit = AuditLog::new(100);
    // Bilinmeyen program → politika "Ask"; onaylı çağrı çalışmayı dener.
    let res = run(&t, &audit, "aetheros_olmayan_program_xyz --x", true)
        .await
        .unwrap();
    assert!(!res.success);
    assert!(res.output.contains("programı başlatılamadı"), "{}", res.output);
}

#[cfg(unix)]
#[test]
fn user_terminal_check_denies_paths_outside_the_workspace() {
    let dir = std::env::temp_dir().join(format!("aeth_uconf_{}", uuid::Uuid::new_v4()));
    let t = TerminalAgentTool::with_workspace(dir.clone());
    for line in ["cat /etc/passwd", "ls ..", "ls /"] {
        let r = check(&t, line).unwrap();
        assert!(matches!(r.verdict, CallVerdict::Deny { .. }), "{line}: {:?}", r.verdict);
    }
    assert_eq!(check(&t, "ls").unwrap().verdict, CallVerdict::Allow);
    let _ = std::fs::remove_dir_all(&dir);
}
