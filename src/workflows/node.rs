// ============================================================
// src/workflows/node.rs
//
// Faz 5 Düzeltmesi: WorkflowNodeKind Duplikasyonu Kaldırıldı
//
// ÖNCE:
//   WorkflowNodeKind: Wasm, Agent, Script, RemoteTask, InternalTask
//   orchestration::graph::ExecutionNodeKind: Workflow, Agent,
//     Task, Wasm, AiInference, Plugin, RemoteTask
//   → İki ayrı enum, aynı kavram, farklı variant'lar.
//
// SONRA:
//   WorkflowNodeKind → ExecutionNodeKind alias.
//   WorkflowNode artık orchestration::graph::ExecutionNode
//   üzerinde çalışıyor.
//   Mevcut kod WorkflowNodeKind kullanmaya devam edebilir.
// ============================================================

// orchestration::graph zaten daha zengin — 7 variant
// (AiInference ve Plugin workflow'larda da lazım olacak)
pub use crate::orchestration::graph::ExecutionNodeKind as WorkflowNodeKind;
pub use crate::orchestration::graph::ExecutionNode as WorkflowNode;
