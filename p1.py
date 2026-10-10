#!/usr/bin/env python3
"""Adım 2a: Agent'a "cevap ver" yolu (merhaba / soru / bitiş özeti).

Rust:
  - AuditEventKind::AssistantMessage { text }          (logging/audit.rs)
  - describe_audit_event: "assistant_message" etiketi    (bridge/api/helpers.rs)
  - NextStepDecision::Answer(String) + "message" alanı   (agents/planning/planner.rs)
  - otonom döngü: Answer -> denetime yaz, döngüyü bitir  (agents/runtime/loops.rs)
  - 2 test                                               (tests/autonomous_loop_tests.rs)
Dart:
  - assistant_message denetim olayları sohbet mesajı olarak gösterilir
    (Activity listesinde tekrarlanmaz).
Proje kökünde: python3 apply_agent_answer.py   (idempotent)"""
import pathlib, sys

changed = []

def edit(rel, fn):
    p = pathlib.Path(rel)
    if not p.exists():
        sys.exit(f"HATA: {rel} yok (proje kökünde misiniz?)")
    o = p.read_text(); n = fn(o)
    if n != o:
        p.write_text(n); changed.append(rel)

def rep(t, old, new, rel, marker):
    if marker in t:
        return t
    if t.count(old) != 1:
        sys.exit(f"HATA: {rel}: kalıp bulunamadı/benzersiz değil:\n{old[:100]}")
    return t.replace(old, new, 1)

# ── audit.rs ──────────────────────────────────────────────
def audit(t):
    rel = "src/logging/audit.rs"
    t = rep(t, "const MAX_AUDIT_TEXT_CHARS: usize = 2_000;\n",
            "const MAX_AUDIT_TEXT_CHARS: usize = 2_000;\n"
            "/// Agent'ın kullanıcıya yazdığı cevap metni için üst sınır.\n"
            "const MAX_ASSISTANT_TEXT_CHARS: usize = 4_000;\n",
            rel, "MAX_ASSISTANT_TEXT_CHARS")
    t = rep(t,
        "    /// Execution bir hatayla bitti (retryable olmayan adım hatası vb.).\n"
        "    ExecutionFailed { error: String },\n}",
        "    /// Execution bir hatayla bitti (retryable olmayan adım hatası vb.).\n"
        "    ExecutionFailed { error: String },\n"
        "    /// Agent'ın kullanıcıya yazdığı düz metin cevap (selamlama, soru\n"
        "    /// yanıtı, iş bitince kısa özet). Araç çağrısı değildir.\n"
        "    AssistantMessage { text: String },\n}",
        rel, "AssistantMessage { text: String }")
    t = rep(t,
        "            other => other,\n        }\n    }\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct AuditEvent {",
        "            AuditEventKind::AssistantMessage { text } => AuditEventKind::AssistantMessage {\n"
        "                text: clip(&text, MAX_ASSISTANT_TEXT_CHARS),\n"
        "            },\n"
        "            other => other,\n        }\n    }\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct AuditEvent {",
        rel, "MAX_ASSISTANT_TEXT_CHARS)")
    return t

# ── helpers.rs ────────────────────────────────────────────
def helpers(t):
    rel = "src/bridge/api/helpers.rs"
    return rep(t,
        '        K::ExecutionFailed { error } => ("execution_failed", format!("hata: {error}")),\n',
        '        K::ExecutionFailed { error } => ("execution_failed", format!("hata: {error}")),\n'
        '        K::AssistantMessage { text } => ("assistant_message", text.clone()),\n',
        rel, "assistant_message")

# ── types.rs (yalnız doc) ─────────────────────────────────
def types(t):
    rel = "src/bridge/types.rs"
    return rep(t, '    /// "execution_failed"\n    pub kind_label: String,',
               '    /// "execution_failed" | "assistant_message"\n    pub kind_label: String,',
               rel, '"assistant_message"')

# ── planner.rs ────────────────────────────────────────────
def planner(t):
    rel = "src/agents/planning/planner.rs"
    t = rep(t,
        "pub enum NextStepDecision {\n    Step(AgentPlanStep),\n    Done,\n}",
        "pub enum NextStepDecision {\n    Step(AgentPlanStep),\n    Done,\n"
        "    /// Hedef bitti ve AI kullanıcıya yazılı bir cevap/özet bıraktı\n"
        "    /// (`{\"done\": true, \"message\": \"…\"}`). Araç adımı DEĞİLDİR.\n"
        "    Answer(String),\n}",
        rel, "Answer(String)")
    t = rep(t,
        "    #[serde(default)]\n    done: bool,\n    #[serde(default)]\n    name: Option<String>,",
        "    #[serde(default)]\n    done: bool,\n"
        "    /// `done: true` ile birlikte gelen kullanıcıya cevap / bitiş özeti.\n"
        "    #[serde(default)]\n    message: Option<String>,\n"
        "    #[serde(default)]\n    name: Option<String>,",
        rel, "message: Option<String>")
    t = rep(t,
        "Output ONLY valid JSON, no explanation, in exactly one of these two forms:\n"
        "{{\"done\": true}}\n"
        "{{\"done\": false, \"name\": \"step_name\", \"retryable\": true, \"tool_name\": \"...\", \"arguments\": [\"...\"]}}\n",
        "Output ONLY valid JSON, no explanation, in exactly one of these three forms:\n"
        "{{\"done\": true}}\n"
        "{{\"done\": true, \"message\": \"short reply to the user\"}}\n"
        "{{\"done\": false, \"name\": \"step_name\", \"retryable\": true, \"tool_name\": \"...\", \"arguments\": [\"...\"]}}\n",
        rel, "three forms")
    t = rep(t,
        "- Tool \"arguments\" is a JSON array of strings;",
        "- If the objective is a greeting, a question, or otherwise needs no tool,\n"
        "  do NOT invent a tool step: answer {{\"done\": true, \"message\": \"...\"}} with a\n"
        "  short reply in the user's language. When you finish after tool steps,\n"
        "  put a one or two sentence summary of what was done in \"message\".\n"
        "- Tool \"arguments\" is a JSON array of strings;",
        rel, "needs no tool")
    t = rep(t,
        "        if parsed.done {\n            return Ok(NextStepDecision::Done);\n        }",
        "        if parsed.done {\n"
        "            return Ok(match parsed.message.map(|m| m.trim().to_string()) {\n"
        "                Some(m) if !m.is_empty() => NextStepDecision::Answer(m),\n"
        "                _ => NextStepDecision::Done,\n"
        "            });\n"
        "        }",
        rel, "NextStepDecision::Answer(m)")
    return t

# ── loops.rs ──────────────────────────────────────────────
def loops(t):
    rel = "src/agents/runtime/loops.rs"
    return rep(t,
        "                Some(NextStepDecision::Step(step)) => step,\n",
        "                Some(NextStepDecision::Answer(text)) => {\n"
        "                    info!(\n"
        "                        agent_id = %context.agent_id,\n"
        "                        execution_id = %context.execution_id,\n"
        "                        \"Autonomous loop: AI kullanıcıya cevap bıraktı, tamamlandı\"\n"
        "                    );\n"
        "                    self.audit(\n"
        "                        context.agent_id,\n"
        "                        context.execution_id,\n"
        "                        AuditEventKind::AssistantMessage { text },\n"
        "                    );\n"
        "                    break;\n"
        "                }\n"
        "                Some(NextStepDecision::Step(step)) => step,\n",
        rel, "NextStepDecision::Answer(text)")

# ── tests ─────────────────────────────────────────────────
TESTS = '''

// ── Cevap yolu: araç gerektirmeyen hedefe düz metin cevap ──

#[tokio::test]
async fn plan_next_parses_a_text_answer() {
    let (router, _p) = scripted_router(&[r#"{"done": true, "message": "  Merhaba!  "}"#]);
    match AgentPlanner::plan_next_detailed("merhaba", &[], &router, &[]).await {
        Ok(NextStepDecision::Answer(text)) => assert_eq!(text, "Merhaba!"),
        other => panic!("cevap bekleniyordu: {other:?}"),
    }
}

#[tokio::test]
async fn blank_message_is_plain_done() {
    let (router, _p) = scripted_router(&[r#"{"done": true, "message": "   "}"#]);
    assert!(matches!(
        AgentPlanner::plan_next_detailed("hedef", &[], &router, &[]).await,
        Ok(NextStepDecision::Done)
    ));
}

#[tokio::test]
async fn answer_is_recorded_for_the_user_and_the_run_completes() {
    let (router, _p) = scripted_router(&[r#"{"done": true, "message": "Merhaba!"}"#]);
    let audit = Arc::new(AuditLog::new(100));
    let context = ctx();
    let exec_id = context.execution_id;

    let result = AgentExecutor::execute(
        context,
        "merhaba".to_string(),
        test_budget(),
        vec![],
        Some(router),
        None,
        None,
        None,
        Some(audit.clone()),
    )
    .await;

    assert_eq!(result.unwrap(), AgentOutcome::Completed);
    let events = audit.list_for_execution(exec_id);
    assert!(events.iter().any(|e| matches!(
        &e.kind,
        AuditEventKind::AssistantMessage { text } if text == "Merhaba!"
    )));
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, AuditEventKind::ExecutionCompleted))
    );
}
'''

def tests(t):
    if "plan_next_parses_a_text_answer" in t:
        return t
    return t.rstrip("\n") + "\n\n" + TESTS.lstrip("\n")

# ── Dart ──────────────────────────────────────────────────
def dart(t):
    rel = "flutter_app/lib/application/services/frb_agent_service.dart"
    if "assistant_message" in t:
        return t
    t = rep(t, "      List<AgentToolActivity> activity = const [];\n",
            "      List<AgentToolActivity> activity = const [];\n"
            "      final answers = <AgentMessageState>[];\n", rel, "final answers")
    t = rep(t,
        "          for (final e in audit)\n            AgentToolActivity(",
        "          for (final e in audit)\n            if (e.kindLabel != 'assistant_message')\n            AgentToolActivity(",
        rel, "kindLabel != 'assistant_message'")
    t = rep(t,
        "      } catch (e) {\n        // Audit is optional enrichment;",
        "        for (final e in audit) {\n"
        "          if (e.kindLabel == 'assistant_message' && e.summary.isNotEmpty) {\n"
        "            answers.add(AgentMessageState(\n"
        "              id: 'ai-${e.id}',\n"
        "              role: 'assistant',\n"
        "              content: e.summary,\n"
        "            ));\n"
        "          }\n"
        "        }\n"
        "      } catch (e) {\n        // Audit is optional enrichment;",
        rel, "role: 'assistant'")
    t = rep(t,
        "      _patchSession(executionId, (s) {\n        return s.copyWith(\n          status: status.status,",
        "      _patchSession(executionId, (s) {\n"
        "        final known = s.messages.map((m) => m.id).toSet();\n"
        "        final fresh = answers.where((a) => !known.contains(a.id)).toList();\n"
        "        return s.copyWith(\n          status: status.status,",
        rel, "final fresh")
    t = rep(t, "          messages: status.error != null &&",
            "          messages: [...(status.error != null &&", rel, "[...(status.error")
    t = rep(t, "              : s.messages,\n        );",
            "              : s.messages), ...fresh],\n        );", rel, "...fresh]")
    return t

edit("src/logging/audit.rs", audit)
edit("src/bridge/api/helpers.rs", helpers)
edit("src/bridge/types.rs", types)
edit("src/agents/planning/planner.rs", planner)
edit("src/agents/runtime/loops.rs", loops)
edit("src/tests/autonomous_loop_tests.rs", tests)
edit("flutter_app/lib/application/services/frb_agent_service.dart", dart)
print("Değişenler:" if changed else "Zaten uygulanmış.")
for c in changed: print("  -", c)
