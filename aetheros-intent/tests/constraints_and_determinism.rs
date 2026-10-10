//! Kısıt uygulaması, deterministik sıra ve graf bütünlüğü testleri.

use aetheros_intent::*;

fn chain(n: usize) -> (TaskGraph, Vec<uuid::Uuid>) {
    let mut g = TaskGraph::new();
    let ids: Vec<_> = (0..n)
        .map(|i| g.add_node(TaskNode::new(TaskRole::Implement, format!("T{i:02}"))))
        .collect();
    for w in ids.windows(2) {
        g.add_edge(w[0], w[1], DependencyKind::FinishToStart)
            .unwrap();
    }
    (g, ids)
}

#[test]
fn failed_add_edge_leaves_graph_untouched() {
    let (mut g, ids) = chain(3);
    let edges_before = g.edges.len();
    let deps_before = g.nodes[&ids[0]].dependencies.clone();
    let r = g.add_edge(ids[2], ids[0], DependencyKind::FinishToStart);
    assert!(matches!(r, Err(IntentError::DependencyCycle(_))));
    assert_eq!(g.edges.len(), edges_before, "başarısız kenar eklenmemeli");
    assert_eq!(g.nodes[&ids[0]].dependencies, deps_before);
    assert!(g.topological_order().is_ok(), "graf hâlâ geçerli olmalı");
}

#[test]
fn self_loop_is_rejected() {
    let (mut g, ids) = chain(1);
    let r = g.add_edge(ids[0], ids[0], DependencyKind::Soft);
    assert!(matches!(r, Err(IntentError::DependencyCycle(_))));
}

#[test]
fn topological_order_is_deterministic_and_title_sorted() {
    let mut g = TaskGraph::new();
    // Bağımsız düğümler: sıra başlığa göre olmalı, ekleme sırasına/HashMap'e değil.
    for t in ["C", "A", "B"] {
        g.add_node(TaskNode::new(TaskRole::Custom, t));
    }
    let titles = |g: &TaskGraph| -> Vec<String> {
        g.topological_order()
            .unwrap()
            .iter()
            .map(|id| g.nodes[id].title.clone())
            .collect()
    };
    assert_eq!(titles(&g), vec!["A", "B", "C"]);
    for _ in 0..20 {
        assert_eq!(titles(&g), vec!["A", "B", "C"]);
    }
}

#[test]
fn max_depth_is_enforced() {
    let mut g = TaskGraph::new();
    let root = g.add_node(TaskNode::new(TaskRole::Analyze, "root"));
    let mut parent = root;
    for i in 0..4 {
        let mut n = TaskNode::new(TaskRole::Implement, format!("n{i}"));
        n.parent = Some(parent);
        parent = g.add_node(n);
    }
    assert_eq!(g.hierarchy_depth(), 5);

    let ok = Constraints {
        max_depth: 5,
        max_parallel_branches: 5,
        require_verification: false,
        ..Constraints::default()
    };
    assert!(validate_graph(&g, &ok).is_ok());

    let tight = Constraints {
        max_depth: 4,
        max_parallel_branches: 5,
        require_verification: false,
        ..Constraints::default()
    };
    assert!(matches!(
        validate_graph(&g, &tight),
        Err(IntentError::Constraint(_))
    ));
}

#[test]
fn max_parallel_branches_is_enforced() {
    let mut g = TaskGraph::new();
    let root = g.add_node(TaskNode::new(TaskRole::Analyze, "root"));
    for i in 0..5 {
        let n = g.add_node(TaskNode::new(TaskRole::Implement, format!("p{i}")));
        g.add_edge(root, n, DependencyKind::FinishToStart).unwrap();
    }
    assert_eq!(g.max_parallel_width().unwrap(), 5);
    let c = Constraints {
        max_parallel_branches: 4,
        require_verification: false,
        ..Constraints::default()
    };
    assert!(matches!(
        validate_graph(&g, &c),
        Err(IntentError::Constraint(_))
    ));
    let c = Constraints {
        max_parallel_branches: 5,
        require_verification: false,
        ..Constraints::default()
    };
    assert!(validate_graph(&g, &c).is_ok());
}

#[test]
fn denied_domain_blocks_compilation() {
    let c = Constraints {
        denied_domains: vec!["docs".into()],
        ..Constraints::default()
    };
    let r = IntentPipeline::new(c).process("README dosyasını düzelt");
    assert!(matches!(r, Err(IntentError::Constraint(_))), "got {r:?}");
}

#[test]
fn allowed_domains_whitelist_is_enforced() {
    let c = Constraints {
        allowed_domains: vec!["backend".into()],
        ..Constraints::default()
    };
    let r = IntentPipeline::new(c).process("README dosyasını düzelt");
    assert!(matches!(r, Err(IntentError::Constraint(_))), "got {r:?}");
}

#[test]
fn default_constraints_still_accept_typical_requests() {
    for input in [
        "README dosyasını düzelt",
        "API endpoint ekle",
        "Kullanıcı girişi auth ekle",
        "Facebook benzeri sosyal medya sitesi oluştur auth feed posts ile",
    ] {
        IntentPipeline::default()
            .process(input)
            .unwrap_or_else(|e| panic!("`{input}` varsayılan kısıtlarla geçmeli: {e}"));
    }
}

#[test]
fn multi_word_vague_input_is_flagged() {
    let intent = UserIntent::new("bir şey yap", IntentKind::General, 0.9);
    let r = analyze_ambiguity(&intent);
    assert!(
        r.reasons.iter().any(|x| x == "vague wording"),
        "{:?}",
        r.reasons
    );
}
