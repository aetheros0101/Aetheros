//! FunctionalRequirement → somut task spesifikasyonları (role-template değil).

use crate::compilation::task_graph::{TaskNode, TaskPriority, TaskRole, TaskVerification};
use crate::requirement::FuncArea;
use crate::requirement::{AcceptanceCriterion, FunctionalRequirement, Priority, RequirementSet};

/// Tek bir requirement'tan üretilen planlı görevler (henüz grafa eklenmemiş).
#[derive(Debug, Clone)]
pub struct TaskSpec {
    pub node: TaskNode,
    /// Aynı requirement içindeki sıra (0 = önce).
    pub seq: u32,
}

pub fn expand_requirement_set(req: &RequirementSet) -> Vec<TaskSpec> {
    let mut out = Vec::new();
    let global_acceptance: Vec<String> = req.acceptance.iter().map(|a| a.text.clone()).collect();

    // Analyze her zaman (kök bağlam)
    let mut analyze = TaskNode::new(TaskRole::Analyze, format!("Analyze: {}", req.summary));
    analyze.description = req.summary.clone();
    analyze.domain_tags = req.domain_tags.clone();
    analyze.priority = map_priority(req.priority);
    analyze.capabilities = vec!["workspace_read".into(), "ai_reasoning".into()];
    analyze.verification = TaskVerification {
        method: Some("review".into()),
        expected: Some("Kapsam ve riskler net".into()),
        done: false,
    };
    out.push(TaskSpec {
        node: analyze,
        seq: 0,
    });

    if req.domain_tags.iter().any(|t| t == "docs" || t == "chore") {
        out.extend(expand_docs(req, &global_acceptance));
        return with_verification_tail(out, req);
    }

    if req.domain_tags.iter().any(|t| t == "research") {
        out.extend(expand_research(req, &global_acceptance));
        return with_verification_tail(out, req);
    }

    for fr in &req.functional {
        out.extend(expand_functional(fr, &req.acceptance, &req.domain_tags));
    }

    // NFR → verification / testing hints
    for nfr in &req.non_functional {
        let mut n = TaskNode::new(TaskRole::Custom, format!("NFR: {:?}", nfr.attribute));
        n.description = nfr.description.clone();
        n.source_requirement = Some(nfr.id);
        n.capabilities = vec!["workspace_read".into()];
        n.priority = TaskPriority::Medium;
        n.verification = TaskVerification {
            method: Some("checklist".into()),
            expected: Some(nfr.description.clone()),
            done: false,
        };
        out.push(TaskSpec { node: n, seq: 50 });
    }

    with_verification_tail(out, req)
}

fn expand_docs(req: &RequirementSet, acceptance: &[String]) -> Vec<TaskSpec> {
    let mut specs = Vec::new();
    let mut locate = TaskNode::new(TaskRole::Edit, "Locate target document")
        .with_capabilities(&["workspace_read"]);
    locate.description = req.summary.clone();
    locate.acceptance_criteria = acceptance.to_vec();
    locate.priority = TaskPriority::Low;
    specs.push(TaskSpec {
        node: locate,
        seq: 1,
    });

    let mut edit = TaskNode::new(TaskRole::Edit, format!("Edit: {}", req.summary))
        .with_capabilities(&["workspace_read", "workspace_write"]);
    edit.description = req.summary.clone();
    edit.acceptance_criteria = acceptance.to_vec();
    edit.verification = TaskVerification {
        method: Some("markdown_lint".into()),
        expected: Some("Dosya güncel ve okunabilir".into()),
        done: false,
    };
    if let Some(fr) = req.functional.first() {
        edit.source_requirement = Some(fr.id);
    }
    specs.push(TaskSpec { node: edit, seq: 2 });
    specs
}

fn expand_research(req: &RequirementSet, acceptance: &[String]) -> Vec<TaskSpec> {
    let mut n = TaskNode::new(TaskRole::Research, format!("Research: {}", req.summary))
        .with_capabilities(&["workspace_read", "ai_reasoning"]);
    n.acceptance_criteria = acceptance.to_vec();
    n.verification = TaskVerification {
        method: Some("written_notes".into()),
        expected: Some("Bulgular kaydedildi".into()),
        done: false,
    };
    vec![TaskSpec { node: n, seq: 1 }]
}

fn expand_functional(
    fr: &FunctionalRequirement,
    acceptance: &[AcceptanceCriterion],
    domain_tags: &[String],
) -> Vec<TaskSpec> {
    let acc: Vec<String> = acceptance.iter().map(|a| a.text.clone()).collect();
    let pri = map_priority(fr.priority);

    match fr.area {
        FuncArea::Docs => {
            let mut n = TaskNode::new(TaskRole::Edit, fr.title.clone())
                .with_source(fr.id)
                .with_acceptance(acc)
                .with_capabilities(&["workspace_read", "workspace_write"]);
            n.description = fr.description.clone();
            n.priority = pri;
            vec![TaskSpec { node: n, seq: 1 }]
        }
        FuncArea::Auth => expand_auth(fr, &acc, pri),
        FuncArea::Ui => expand_ui(fr, &acc, pri),
        FuncArea::Api => expand_api(fr, &acc, pri),
        FuncArea::Data => expand_data(fr, &acc, pri),
        FuncArea::Other => {
            // Generic implement chain
            let mut specs = Vec::new();
            if domain_tags.iter().any(|t| t == "architecture") {
                let mut a = TaskNode::new(TaskRole::Architecture, format!("Design: {}", fr.title))
                    .with_source(fr.id)
                    .with_capabilities(&["workspace_read", "ai_reasoning"]);
                a.priority = pri;
                specs.push(TaskSpec { node: a, seq: 1 });
            }
            let mut impl_n = TaskNode::new(TaskRole::Implement, format!("Implement: {}", fr.title))
                .with_source(fr.id)
                .with_acceptance(acc.clone())
                .with_capabilities(&["workspace_read", "workspace_write"]);
            impl_n.description = fr.description.clone();
            impl_n.priority = pri;
            specs.push(TaskSpec {
                node: impl_n,
                seq: 2,
            });
            let mut test = TaskNode::new(TaskRole::Testing, format!("Test: {}", fr.title))
                .with_source(fr.id)
                .with_capabilities(&["workspace_read", "terminal_execution"]);
            test.acceptance_criteria = acc;
            test.priority = pri;
            specs.push(TaskSpec { node: test, seq: 3 });
            specs
        }
    }
}

fn expand_auth(fr: &FunctionalRequirement, acc: &[String], pri: TaskPriority) -> Vec<TaskSpec> {
    let steps: &[(&str, TaskRole, &[&str])] = &[
        (
            "Analyze auth architecture",
            TaskRole::Architecture,
            &["workspace_read", "ai_reasoning"],
        ),
        (
            "Implement user model",
            TaskRole::Database,
            &["workspace_write"],
        ),
        (
            "Implement password hashing",
            TaskRole::Backend,
            &["workspace_write"],
        ),
        (
            "Implement login endpoint",
            TaskRole::Api,
            &["workspace_write"],
        ),
        (
            "Implement session/token handling",
            TaskRole::Backend,
            &["workspace_write"],
        ),
        ("Implement login UI", TaskRole::UiAuth, &["workspace_write"]),
        (
            "Implement logout",
            TaskRole::Implement,
            &["workspace_write"],
        ),
        (
            "Add auth tests",
            TaskRole::Testing,
            &["terminal_execution", "workspace_read"],
        ),
        (
            "Verify auth flow",
            TaskRole::Verification,
            &["workspace_read"],
        ),
    ];
    steps
        .iter()
        .enumerate()
        .map(|(i, (title, role, caps))| {
            let mut n = TaskNode::new(*role, *title)
                .with_source(fr.id)
                .with_acceptance(acc.to_vec())
                .with_capabilities(caps);
            n.description = fr.description.clone();
            n.priority = pri;
            n.verification = TaskVerification {
                method: Some(
                    if matches!(role, TaskRole::Testing | TaskRole::Verification) {
                        "test".into()
                    } else {
                        "code_review".into()
                    },
                ),
                expected: Some(title.to_string()),
                done: false,
            };
            TaskSpec {
                node: n,
                seq: (i as u32) + 1,
            }
        })
        .collect()
}

fn expand_ui(fr: &FunctionalRequirement, acc: &[String], pri: TaskPriority) -> Vec<TaskSpec> {
    let titles = [
        ("Design UI for", TaskRole::Frontend),
        ("Implement UI for", TaskRole::Frontend),
        ("Verify UI for", TaskRole::Verification),
    ];
    titles
        .iter()
        .enumerate()
        .map(|(i, (prefix, role))| {
            let mut n = TaskNode::new(*role, format!("{prefix} {}", fr.title))
                .with_source(fr.id)
                .with_acceptance(acc.to_vec())
                .with_capabilities(if matches!(role, TaskRole::Frontend) {
                    &["workspace_read", "workspace_write"]
                } else {
                    &["workspace_read"]
                });
            n.priority = pri;
            TaskSpec {
                node: n,
                seq: (i as u32) + 1,
            }
        })
        .collect()
}

fn expand_api(fr: &FunctionalRequirement, acc: &[String], pri: TaskPriority) -> Vec<TaskSpec> {
    [
        ("Design API for", TaskRole::Architecture),
        ("Implement endpoint for", TaskRole::Api),
        ("Test API for", TaskRole::Testing),
    ]
    .iter()
    .enumerate()
    .map(|(i, (prefix, role))| {
        let mut n = TaskNode::new(*role, format!("{prefix} {}", fr.title))
            .with_source(fr.id)
            .with_acceptance(acc.to_vec())
            .with_capabilities(&["workspace_read", "workspace_write"]);
        n.priority = pri;
        TaskSpec {
            node: n,
            seq: (i as u32) + 1,
        }
    })
    .collect()
}

fn expand_data(fr: &FunctionalRequirement, acc: &[String], pri: TaskPriority) -> Vec<TaskSpec> {
    [
        ("Model data for", TaskRole::Database),
        ("Implement persistence for", TaskRole::Backend),
        ("Test data layer for", TaskRole::Testing),
    ]
    .iter()
    .enumerate()
    .map(|(i, (prefix, role))| {
        let mut n = TaskNode::new(*role, format!("{prefix} {}", fr.title))
            .with_source(fr.id)
            .with_acceptance(acc.to_vec())
            .with_capabilities(&["workspace_read", "workspace_write"]);
        n.priority = pri;
        TaskSpec {
            node: n,
            seq: (i as u32) + 1,
        }
    })
    .collect()
}

fn with_verification_tail(mut specs: Vec<TaskSpec>, req: &RequirementSet) -> Vec<TaskSpec> {
    if !specs.iter().any(|s| s.node.role == TaskRole::Verification) {
        let mut v = TaskNode::new(TaskRole::Verification, "Final verification")
            .with_capabilities(&["workspace_read"]);
        v.acceptance_criteria = req.acceptance.iter().map(|a| a.text.clone()).collect();
        v.verification = TaskVerification {
            method: Some("acceptance".into()),
            expected: Some("All acceptance criteria met".into()),
            done: false,
        };
        specs.push(TaskSpec { node: v, seq: 1000 });
    }
    specs
}

fn map_priority(p: Priority) -> TaskPriority {
    match p {
        Priority::Low => TaskPriority::Low,
        Priority::Medium => TaskPriority::Medium,
        Priority::High => TaskPriority::High,
        Priority::Critical => TaskPriority::Critical,
    }
}
