// ============================================================
// src/workflows/execution_graph.rs
//
// Faz 5 Düzeltmesi: Tip Duplikasyonu Kaldırıldı
//
// ÖNCE:
//   ExecutionNode, ExecutionEdge, WorkflowExecutionGraph
//   burada tanımlıydı — orchestration::graph'takilerle
//   neredeyse aynı. İki ayrı tip, aynı kavram.
//
// SONRA:
//   orchestration::graph'taki tipler re-export edildi.
//   WorkflowExecutionGraph → ExecutionGraph alias'ı.
//
//   Fayda:
//   - Tek kaynak (single source of truth)
//   - orchestration/coordination.rs'deki döngüsel
//     bağımlılık kırıldı (artık workflows'a bağlı değil)
//   - Mevcut kullanım yerlerinde import değişmez
// ============================================================

// orchestration::graph zaten daha zengin tanım:
//   ExecutionNode: retryable, correlation_id, metadata
//   ExecutionEdge: from, to
//   ExecutionGraph: id, version, nodes, edges
pub use crate::orchestration::graph::{
    ExecutionEdge,
    ExecutionGraph,
    ExecutionNode,
    ExecutionNodeKind,
};

/// Workflow katmanı için alias.
///
/// WorkflowExecutionGraph ve ExecutionGraph artık aynı tip.
/// Mevcut kod `WorkflowExecutionGraph` kullanmaya devam edebilir.
pub type WorkflowExecutionGraph = ExecutionGraph;
