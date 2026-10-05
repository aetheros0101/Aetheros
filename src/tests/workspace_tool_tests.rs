// ============================================================
// src/tests/workspace_tool_tests.rs
//
// Workspace araçları: argüman sözleşmesi, ön-denetim (assess_call),
// capability + governor zinciri ve gerçek dosya işlemleri.
// ============================================================

use std::sync::Arc;

use uuid::Uuid;

use crate::agents::capabilities::AgentCapability;
use crate::agents::workspace_tool::{WorkspaceAgentTool, WorkspaceToolKind};
use crate::security::capability_engine::CapabilityEngine;
use crate::security::governor::{GovernorDecision, SecurityGovernor};
use crate::security::risk_engine::RiskEngine;
use crate::types::agent_tool::{AgentTool, CallVerdict};

fn a(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn ws() -> (tempfile::TempDir, Arc<aetheros_workspace::Workspace>) {
    let dir = tempfile::tempdir().unwrap();
    let w = Arc::new(aetheros_workspace::Workspace::open(dir.path()).unwrap());
    (dir, w)
}

fn tool(w: &Arc<aetheros_workspace::Workspace>, k: WorkspaceToolKind) -> Arc<dyn AgentTool> {
    Arc::new(WorkspaceAgentTool::new(w.clone(), k))
}

// ── katalog ──

#[test]
fn all_kinds_have_unique_names_and_argument_contracts() {
    let (_d, w) = ws();
    let mut names = std::collections::HashSet::new();
    for k in WorkspaceToolKind::ALL {
        let t = WorkspaceAgentTool::new(w.clone(), k);
        assert!(names.insert(t.name()), "yinelenen ad: {}", t.name());
        assert!(t.name().starts_with("workspace_"));
        assert!(
            t.description().contains("Argüman"),
            "{} açıklaması argüman sözleşmesini anlatmalı",
            t.name()
        );
    }
    assert_eq!(names.len(), 10);
}

#[test]
fn capabilities_and_risk_follow_the_blast_radius() {
    use crate::types::agent_tool::RiskLevel as R;
    let (_d, w) = ws();
    let cap_risk = |k| {
        let t = WorkspaceAgentTool::new(w.clone(), k);
        (t.required_capability(), t.risk_level())
    };
    assert_eq!(cap_risk(WorkspaceToolKind::Read), (Some(AgentCapability::WorkspaceRead), R::Low));
    assert_eq!(cap_risk(WorkspaceToolKind::GitStatus), (Some(AgentCapability::WorkspaceRead), R::Low));
    assert_eq!(cap_risk(WorkspaceToolKind::Write), (Some(AgentCapability::WorkspaceWrite), R::Medium));
    assert_eq!(cap_risk(WorkspaceToolKind::Patch), (Some(AgentCapability::WorkspaceWrite), R::Medium));
    assert_eq!(cap_risk(WorkspaceToolKind::Rename), (Some(AgentCapability::WorkspaceMutate), R::High));
    assert_eq!(cap_risk(WorkspaceToolKind::Delete), (Some(AgentCapability::WorkspaceMutate), R::High));
}

// ── assess_call ──

#[test]
fn assess_denies_traversal_absolute_and_nul_paths() {
    let (_d, w) = ws();
    let t = WorkspaceAgentTool::new(w, WorkspaceToolKind::Read);
    for bad in ["../x", "a/../../x", "..", "/etc/passwd", "\\win", "C:\\x", "a\0b"] {
        assert!(
            matches!(t.assess_call(&a(&[bad])), Some(CallVerdict::Deny { .. })),
            "{bad:?} reddedilmeliydi"
        );
    }
    assert_eq!(t.assess_call(&a(&["notlar/a.txt"])), None);
    assert_eq!(t.assess_call(&a(&["."])), None);
    assert_eq!(t.assess_call(&a(&["a..b/c"])), None, "'..' içeren ama bileşen olmayan ad serbest");
}

#[test]
fn assess_checks_both_paths_of_rename_but_not_the_search_query() {
    let (_d, w) = ws();
    let r = WorkspaceAgentTool::new(w.clone(), WorkspaceToolKind::Rename);
    assert!(matches!(r.assess_call(&a(&["ok.txt", "../out.txt"])), Some(CallVerdict::Deny { .. })));
    let s = WorkspaceAgentTool::new(w, WorkspaceToolKind::Search);
    assert_eq!(s.assess_call(&a(&["../x /etc"])), None);
}

#[test]
fn assess_git_dir_read_asks_but_mutation_is_denied() {
    let (_d, w) = ws();
    let read = WorkspaceAgentTool::new(w.clone(), WorkspaceToolKind::Read);
    assert!(matches!(read.assess_call(&a(&[".git/config"])), Some(CallVerdict::Ask { .. })));
    assert!(matches!(read.assess_call(&a(&["sub/.GIT/config"])), Some(CallVerdict::Ask { .. })));
    let write = WorkspaceAgentTool::new(w.clone(), WorkspaceToolKind::Write);
    assert!(matches!(write.assess_call(&a(&[".git/hooks/x", "boom"])), Some(CallVerdict::Deny { .. })));
    let del = WorkspaceAgentTool::new(w, WorkspaceToolKind::Delete);
    assert!(matches!(del.assess_call(&a(&[".git"])), Some(CallVerdict::Deny { .. })));
}

// ── gerçek işlemler ──

#[tokio::test]
async fn write_read_patch_roundtrip() {
    let (dir, w) = ws();
    tool(&w, WorkspaceToolKind::Write)
        .invoke(a(&["notlar.txt", "merhaba dünya"]))
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(dir.path().join("notlar.txt")).unwrap(), "merhaba dünya");

    let out = tool(&w, WorkspaceToolKind::Read).invoke(a(&["notlar.txt"])).await.unwrap();
    assert!(out.contains("merhaba dünya"), "{out}");

    let edits = r#"[{"find":"dünya","replace":"agent","expected_matches":1}]"#;
    tool(&w, WorkspaceToolKind::Patch).invoke(a(&["notlar.txt", edits])).await.unwrap();
    assert_eq!(std::fs::read_to_string(dir.path().join("notlar.txt")).unwrap(), "merhaba agent");
}

#[tokio::test]
async fn list_root_with_dot_works() {
    let (_d, w) = ws();
    tool(&w, WorkspaceToolKind::CreateFile).invoke(a(&["x.txt"])).await.unwrap();
    let out = tool(&w, WorkspaceToolKind::List).invoke(a(&["."])).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).expect("çıktı geçerli JSON olmalı");
    assert_eq!(v["kind"], "entries");
    assert_eq!(v["entries"][0]["name"], "x.txt");
}

#[tokio::test]
async fn search_returns_matches_as_json() {
    let (_d, w) = ws();
    tool(&w, WorkspaceToolKind::Write)
        .invoke(a(&["a.txt", "bulunacak kelime burada"]))
        .await
        .unwrap();
    let out = tool(&w, WorkspaceToolKind::Search).invoke(a(&["kelime"])).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).expect("çıktı geçerli JSON olmalı");
    assert_eq!(v["kind"], "search");
    assert_eq!(v["matches"][0]["path"], "a.txt");
}

#[tokio::test]
async fn missing_and_invalid_arguments_are_clean_errors() {
    let (_d, w) = ws();
    assert!(tool(&w, WorkspaceToolKind::Read).invoke(vec![]).await.is_err());
    assert!(tool(&w, WorkspaceToolKind::Write).invoke(a(&["f.txt"])).await.is_err());
    assert!(tool(&w, WorkspaceToolKind::Patch).invoke(a(&["f.txt", "json-degil"])).await.is_err());
    assert!(tool(&w, WorkspaceToolKind::Read).invoke(a(&["yok.txt"])).await.is_err());
}

#[tokio::test]
async fn read_is_capped_by_agent_limit() {
    let (dir, w) = ws();
    std::fs::write(dir.path().join("big.txt"), "x".repeat(200 * 1024)).unwrap();
    // 128 KiB tavanı: 200 KiB'lık dosya, istenen limit ne olursa olsun okunmaz.
    assert!(tool(&w, WorkspaceToolKind::Read)
        .invoke(a(&["big.txt", "999999999"]))
        .await
        .is_err());
}

#[tokio::test]
async fn invoke_cannot_escape_even_if_precheck_is_bypassed() {
    // assess_call atlansa bile crate path guard dışarı çıkarmaz.
    let (_d, w) = ws();
    assert!(tool(&w, WorkspaceToolKind::Read).invoke(a(&["../x"])).await.is_err());
    assert!(tool(&w, WorkspaceToolKind::Write).invoke(a(&["../x", "z"])).await.is_err());
    assert!(tool(&w, WorkspaceToolKind::Delete).invoke(a(&["."])).await.is_err());
}

// ── governor zinciri ──

fn governor(agent: Uuid, caps: &[AgentCapability]) -> SecurityGovernor {
    let ce = Arc::new(CapabilityEngine::new());
    for c in caps {
        ce.grant_capability(agent, *c);
    }
    SecurityGovernor::new(Some(ce), Some(Arc::new(RiskEngine::new())))
}

#[test]
fn governor_read_allowed_with_workspace_read() {
    let (_d, w) = ws();
    let agent = Uuid::new_v4();
    let g = governor(agent, &[AgentCapability::WorkspaceRead]);
    let t = tool(&w, WorkspaceToolKind::Read);
    assert_eq!(g.evaluate_call(agent, &t, &a(&["a.txt"])), GovernorDecision::Allow);
}

#[test]
fn governor_denies_write_without_workspace_write() {
    let (_d, w) = ws();
    let agent = Uuid::new_v4();
    let g = governor(agent, &[AgentCapability::WorkspaceRead]);
    let t = tool(&w, WorkspaceToolKind::Write);
    assert!(matches!(
        g.evaluate_call(agent, &t, &a(&["a.txt", "x"])),
        GovernorDecision::Deny { .. }
    ));
}

#[test]
fn governor_delete_needs_approval_even_with_mutate_and_traversal_is_denied() {
    let (_d, w) = ws();
    let agent = Uuid::new_v4();
    let g = governor(agent, &[AgentCapability::WorkspaceMutate]);
    let t = tool(&w, WorkspaceToolKind::Delete);
    assert!(matches!(
        g.evaluate_call(agent, &t, &a(&["a.txt"])),
        GovernorDecision::RequiresApproval { .. }
    ));
    assert!(matches!(
        g.evaluate_call(agent, &t, &a(&["../a.txt"])),
        GovernorDecision::Deny { .. }
    ));
}

#[test]
fn governor_git_read_asks_for_approval() {
    let (_d, w) = ws();
    let agent = Uuid::new_v4();
    let g = governor(agent, &[AgentCapability::WorkspaceRead]);
    let t = tool(&w, WorkspaceToolKind::Read);
    assert!(matches!(
        g.evaluate_call(agent, &t, &a(&[".git/config"])),
        GovernorDecision::RequiresApproval { .. }
    ));
}

#[test]
fn capability_grants_are_independent() {
    let agent = Uuid::new_v4();
    let ce = CapabilityEngine::new();
    ce.grant_capability(agent, AgentCapability::WorkspaceWrite);
    assert!(ce.check(&agent, Some(AgentCapability::WorkspaceWrite)).is_allowed());
    assert!(!ce.check(&agent, Some(AgentCapability::WorkspaceRead)).is_allowed());
    assert!(!ce.check(&agent, Some(AgentCapability::WorkspaceMutate)).is_allowed());
    assert!(!ce.check(&agent, Some(AgentCapability::TerminalExecution)).is_allowed());
}
