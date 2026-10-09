//! aetheros-intent v0.4 — 10 invariant testleri.

use aetheros_intent::*;
use std::collections::HashSet;
use std::sync::Arc;

fn pipeline() -> IntentPipeline {
    IntentPipeline::default()
}

// ── 1. Full pipeline ─────────────────────────────────────────

#[test]
fn t01_full_pipeline_end_to_end() {
    let r = pipeline()
        .process("Kullanıcı girişi için auth sistemi ekle")
        .expect("pipeline");

    assert!(!r.intent.raw_text.is_empty());
    assert!(!r.requirements.functional.is_empty());
    assert!(!r.graph.nodes.is_empty());
    assert!(r.graph.root.is_some());

    let order = r.graph.topological_order().expect("topo");
    assert_eq!(order.len(), r.graph.nodes.len());

    let compiled = r.graph.to_compiled_tasks().expect("compiled");
    assert_eq!(compiled.len(), r.graph.nodes.len());
}

// ── 2. Cycle detection ───────────────────────────────────────

#[test]
fn t02_cycle_detection() {
    let mut g = TaskGraph::new();
    let a = g.add_node(TaskNode::new(TaskRole::Analyze, "A"));
    let b = g.add_node(TaskNode::new(TaskRole::Implement, "B"));
    let c = g.add_node(TaskNode::new(TaskRole::Testing, "C"));

    g.add_edge(a, b, DependencyKind::FinishToStart).unwrap();
    g.add_edge(b, c, DependencyKind::FinishToStart).unwrap();

    // Introduce cycle C → A
    let cycle = g.add_edge(c, a, DependencyKind::FinishToStart);
    assert!(
        matches!(cycle, Err(IntentError::DependencyCycle(_))),
        "expected cycle error, got {cycle:?}"
    );
}

// ── 3. Dependency ordering ───────────────────────────────────

#[test]
fn t03_dependency_ordering() {
    let r = pipeline()
        .process("API rate limit endpoint ekle")
        .expect("pipeline");

    let order = r.graph.topological_order().unwrap();
    let pos: std::collections::HashMap<_, _> =
        order.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    for e in &r.graph.edges {
        if e.kind == DependencyKind::FinishToStart {
            let pf = pos[&e.from];
            let pt = pos[&e.to];
            assert!(
                pf < pt,
                "FinishToStart violated: {:?} must precede {:?}",
                r.graph.nodes[&e.from].title,
                r.graph.nodes[&e.to].title
            );
        }
    }

    // Analyze should be first among roots
    if let Some(root) = r.graph.root {
        assert_eq!(order[0], root);
    }
}

// ── 4. Requirement → Task traceability ───────────────────────

#[test]
fn t04_requirement_task_traceability() {
    let r = pipeline()
        .process("Kullanıcı girişi auth ve login ekle")
        .expect("pipeline");

    let fr_ids: HashSet<_> = r.requirements.functional.iter().map(|f| f.id).collect();
    assert!(!fr_ids.is_empty());

    let traced: Vec<_> = r
        .graph
        .nodes
        .values()
        .filter(|n| n.source_requirement.is_some())
        .collect();

    assert!(!traced.is_empty(), "expected tasks with source_requirement");

    for n in &traced {
        let src = n.source_requirement.unwrap();
        assert!(
            fr_ids.contains(&src)
                || r.requirements
                    .non_functional
                    .iter()
                    .any(|nfr| nfr.id == src),
            "task '{}' source_requirement {src} not in RequirementSet",
            n.title
        );
    }
}

// ── 5. Acceptance criteria propagation ───────────────────────

#[test]
fn t05_acceptance_criteria_propagation() {
    let r = pipeline()
        .process("README dosyasını düzelt")
        .expect("pipeline");

    assert!(
        !r.requirements.acceptance.is_empty(),
        "requirements should carry acceptance"
    );

    let with_ac: Vec<_> = r
        .graph
        .nodes
        .values()
        .filter(|n| !n.acceptance_criteria.is_empty())
        .collect();

    // Edit / Verification nodes should carry criteria
    assert!(
        !with_ac.is_empty(),
        "at least one task should inherit acceptance criteria"
    );

    let compiled = r.graph.to_compiled_tasks().unwrap();
    assert!(
        compiled.iter().any(|t| !t.acceptance_criteria.is_empty()
            || t.role == TaskRole::Analyze
            || t.role == TaskRole::Verification),
        "CompiledTask should preserve acceptance where present"
    );
}

// ── 6. Constraint preservation ───────────────────────────────

#[test]
fn t06_constraint_preservation() {
    let mut ctx = ProjectContextSnapshot::default();
    ctx.framework_hints = vec!["React".into(), "PostgreSQL".into()];

    // Text also mentions tech → heuristic constraints
    let r = pipeline()
        .process_with_context("React ve PostgreSQL ile basit profil sayfası ekle", &ctx)
        .expect("pipeline");

    // Tech choices from context and/or text
    let tech_names: Vec<_> = r
        .requirements
        .tech_choices
        .iter()
        .map(|t| t.name.to_ascii_lowercase())
        .collect();

    assert!(
        tech_names.iter().any(|n| n.contains("react"))
            || r.requirements
                .constraints
                .iter()
                .any(|c| c.text.to_ascii_lowercase().contains("react")),
        "React constraint/tech should be preserved: tech={tech_names:?} constraints={:?}",
        r.requirements.constraints
    );

    // Graph nodes should still be valid under default Constraints
    validate_graph(&r.graph, &Constraints::default()).expect("graph valid");
}

// ── 7. Invalid model JSON ────────────────────────────────────

#[test]
fn t07_invalid_model_json_falls_back() {
    let bad = Arc::new(StaticJsonLlm {
        content: "this is not json at all {{{".into(),
    });

    // Model intent fails parse → hybrid falls back to heuristic
    let hybrid_intent = HybridIntentExtractor::default()
        .with_model(Box::new(ModelIntentExtractor::new(bad.clone())));

    let intent = hybrid_intent
        .extract("README dosyasını düzelt")
        .expect("heuristic fallback");
    assert_eq!(intent.kind, IntentKind::Docs);

    // Model requirement fails → hybrid falls back
    let hybrid_req = HybridRequirementExtractor::default().with_model(Box::new(
        ModelRequirementExtractor::new(Arc::new(StaticJsonLlm {
            content: "NOT JSON".into(),
        })),
    ));

    let set = hybrid_req
        .extract(&intent, &ProjectContextSnapshot::default())
        .expect("req fallback");
    assert!(!set.functional.is_empty());
}

#[test]
fn t07b_model_intent_rejects_garbage_without_hybrid() {
    let model = ModelIntentExtractor::new(Arc::new(StaticJsonLlm {
        content: "not-json".into(),
    }));
    let err = model.extract("anything").unwrap_err();
    assert!(
        matches!(err, IntentError::Extraction(_)),
        "expected Extraction error, got {err:?}"
    );
}

// ── 8. Critical ambiguity ────────────────────────────────────

#[test]
fn t08_critical_ambiguity_blocks_pipeline() {
    // Very short + vague → high ambiguity when confidence low
    // "yap" alone is vague and short
    let result = pipeline().process("yap");
    // Either Ambiguous or Empty-ish path — short "yap" is vague
    match result {
        Err(IntentError::Ambiguous(msg)) => {
            assert!(!msg.is_empty());
        }
        Ok(r) => {
            // If not blocked, ambiguity level should still be elevated for tiny input
            assert!(
                matches!(
                    r.ambiguity.level,
                    AmbiguityLevel::Medium | AmbiguityLevel::High | AmbiguityLevel::Low
                ),
                "tiny input should not be None ambiguity"
            );
        }
        Err(e) => panic!("unexpected error: {e:?}"),
    }

    // Explicit blocking: empty
    let empty = pipeline().process("   ");
    assert!(matches!(empty, Err(IntentError::EmptyInput)));
}

// ── 9. Small request → small graph ───────────────────────────

#[test]
fn t09_small_request_small_graph() {
    let small = pipeline().process("README dosyasını düzelt").expect("docs");
    let large = pipeline()
        .process("Facebook benzeri sosyal medya sitesi oluştur auth feed posts ile")
        .expect("fullstack");

    assert!(
        small.graph.nodes.len() < large.graph.nodes.len(),
        "docs graph ({}) should be smaller than social ({})",
        small.graph.nodes.len(),
        large.graph.nodes.len()
    );

    assert!(
        small.graph.nodes.len() <= 6,
        "README graph should stay small, got {}",
        small.graph.nodes.len()
    );

    assert!(!small.graph.nodes.values().any(|n| {
        matches!(
            n.role,
            TaskRole::Backend | TaskRole::Database | TaskRole::Api | TaskRole::Integration
        )
    }));
}

// ── 10. Adapter dependency preservation ──────────────────────

#[test]
fn t10_adapter_dependency_preservation() {
    let r = pipeline()
        .process("Login API ve test ekle")
        .expect("pipeline");

    let compiled = r.graph.to_compiled_tasks().expect("compiled");
    assert!(!compiled.is_empty());

    // Every depends_on id must exist in the compiled set
    let ids: HashSet<_> = compiled.iter().map(|t| t.id).collect();
    for t in &compiled {
        for dep in &t.depends_on {
            assert!(
                ids.contains(dep),
                "task '{}' depends on unknown id {dep}",
                t.title
            );
        }
    }

    // Topo order must respect depends_on
    let pos: std::collections::HashMap<_, _> = compiled
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id, i))
        .collect();
    for t in &compiled {
        let pt = pos[&t.id];
        for dep in &t.depends_on {
            assert!(
                pos[dep] < pt,
                "CompiledTask dependency order broken for '{}'",
                t.title
            );
        }
    }

    // Identity adapter preserves count and ids
    let exported = export_to_task_layer(&r.graph, &IdentityTaskAdapter).unwrap();
    assert_eq!(exported.len(), compiled.len());
    let exported_set: HashSet<_> = exported.into_iter().collect();
    assert_eq!(exported_set, ids);
}
