// ============================================================
// flutter_app/lib/src/rust/api/aetheros.dart
//
// ⚡ STUB DEĞİL — Gerçek FRB bridge adapter'ı
//
// FRB codegen bu dosyaya DOKUNMAZ.
// Bu dosya bridge/api.dart'ı çağırır, BigInt↔int dönüşümlerini
// kapsüller ve tüm ekranların beklediği arayüzü sağlar.
// ============================================================

// ignore_for_file: unused_import, camel_case_types

import 'dart:async';

import '../frb_generated.dart';
import '../bridge/api.dart' as _bridge;
import 'rest/router.dart' as rest_router;
import '../bridge/types.dart' as _bt;
import '../metrics/runtime.dart' as _bm;

// ── Ekranların beklediği tipler (int tabanlı, değişmez) ──────

class TaskRequest {
  final String wasmModuleHash;
  final String entrypoint;
  final String priority;
  final int timeoutMs;
  final int maxRetries;

  const TaskRequest({
    required this.wasmModuleHash,
    required this.entrypoint,
    required this.priority,
    required this.timeoutMs,
    required this.maxRetries,
  });
}

class TaskStatusResponse {
  final String taskId;
  final String state;
  final int createdAt;
  final int updatedAt;
  final int attempts;
  final String? errorMessage;

  const TaskStatusResponse({
    required this.taskId,
    required this.state,
    required this.createdAt,
    required this.updatedAt,
    required this.attempts,
    this.errorMessage,
  });
}

class MetricsSnapshot {
  final int activeWorkers;
  final int queuedTasks;
  final int completedTasks;
  final int failedTasks;
  final int retriedTasks;

  const MetricsSnapshot({
    required this.activeWorkers,
    required this.queuedTasks,
    required this.completedTasks,
    required this.failedTasks,
    required this.retriedTasks,
  });
}

class RuntimeInfo {
  final String version;
  final bool isRunning;
  final String backend;
  final int workerCount;

  const RuntimeInfo({
    required this.version,
    required this.isRunning,
    required this.backend,
    required this.workerCount,
  });
}

class ModuleUploadResponse {
  final String hash;
  final int size;

  const ModuleUploadResponse({required this.hash, required this.size});
}

class LogRecord {
  final int timestampMs;
  final String level;
  final String? taskId;
  final String message;
  final String eventType;

  const LogRecord({
    required this.timestampMs,
    required this.level,
    this.taskId,
    required this.message,
    required this.eventType,
  });
}

// ── Gerçek bridge fonksiyonları ───────────────────────────────

Future<void> initializeRuntime({
  required String dbPath,
  required int workerCount,
}) =>
    _bridge.initializeRuntime(dbPath: dbPath, workerCount: workerCount);

Future<bool> isRuntimeReady() => _bridge.isRuntimeReady();

Future<RuntimeInfo> getRuntimeInfo() async {
  final r = await _bridge.getRuntimeInfo();
  return RuntimeInfo(
    version: r.version,
    isRunning: r.isRunning,
    backend: r.backend,
    workerCount: r.workerCount, // u32 → int: doğrudan uyumlu
  );
}

Future<String> submitTask({required TaskRequest request}) =>
    _bridge.submitTask(
      request: _bt.TaskRequest(
        wasmModuleHash: request.wasmModuleHash,
        entrypoint: request.entrypoint,
        priority: request.priority,
        timeoutMs: BigInt.from(request.timeoutMs), // int → BigInt (u64)
        maxRetries: request.maxRetries,             // u32 → int: doğrudan uyumlu
      ),
    );

Future<TaskStatusResponse> getTaskStatus({required String taskId}) async {
  final r = await _bridge.getTaskStatus(taskId: taskId);
  return TaskStatusResponse(
    taskId: r.taskId,
    state: r.state,
    createdAt: r.createdAt.toInt(), // PlatformInt64 → int
    updatedAt: r.updatedAt.toInt(),
    attempts: r.attempts,
    errorMessage: r.errorMessage,
  );
}

Future<List<TaskStatusResponse>> listTasks({required int limit}) async {
  final list = await _bridge.listTasks(limit: limit);
  return list
      .map((r) => TaskStatusResponse(
            taskId: r.taskId,
            state: r.state,
            createdAt: r.createdAt.toInt(),
            updatedAt: r.updatedAt.toInt(),
            attempts: r.attempts,
            errorMessage: r.errorMessage,
          ))
      .toList();
}

Future<ModuleUploadResponse> uploadWasmModule({
  required List<int> bytes,
}) async {
  final r = await _bridge.uploadWasmModule(bytes: bytes);
  return ModuleUploadResponse(
    hash: r.hash,
    size: r.size.toInt(), // BigInt (u64) → int
  );
}

/// WAT → WASM derle + ModuleStore'a kaydet.
/// FRB bridge'de compile_wat_to_wasm olarak tanımlı.
/// Codegen'den geçince _bridge.compileWatToWasm() direkt çağrılabilir.
Future<ModuleUploadResponse> compileWatToWasm({
  required String name,
  required String watSource,
  required String entrypoint,
  required int timeoutMs,
}) async {
  final r = await _bridge.compileWatToWasm(
    name:       name,
    watSource:  watSource,
    entrypoint: entrypoint,
    timeoutMs:  BigInt.from(timeoutMs), // u64 → BigInt
  );
  return ModuleUploadResponse(
    hash: r.hash,
    size: r.size.toInt(),
  );
}

Future<MetricsSnapshot> getMetrics() async {
  final s = await _bridge.getMetrics();
  return MetricsSnapshot(
    activeWorkers:  s.activeWorkers.toInt(),  // BigInt (u64) → int
    queuedTasks:    s.queuedTasks.toInt(),
    completedTasks: s.completedTasks.toInt(),
    failedTasks:    s.failedTasks.toInt(),
    retriedTasks:   s.retriedTasks.toInt(),
  );
}

// ── WASM modül kontrolü ───────────────────────────────────

Future<bool> checkModuleExists({required String hashHex}) =>
    _bridge.checkModuleExists(hashHex: hashHex);

// ── Task yeniden gönderme ─────────────────────────────────

Future<String> resubmitTask({required String taskId}) =>
    _bridge.resubmitTask(taskId: taskId);

// ── Log izleme ───────────────────────────────────────────

Future<List<LogRecord>> getRecentLogs({required int limit}) async {
  final list = await _bridge.getRecentLogs(limit: limit);
  return list
      .map((e) => LogRecord(
            timestampMs: e.timestampMs.toInt(), // PlatformInt64 → int
            level: e.level,
            taskId: e.taskId,
            message: e.message,
            eventType: e.eventType,
          ))
      .toList();
}

Future<List<LogRecord>> getTaskLogs({
  required String taskId,
  required int limit,
}) async {
  final list = await _bridge.getTaskLogs(taskId: taskId, limit: limit);
  return list
      .map((e) => LogRecord(
            timestampMs: e.timestampMs.toInt(),
            level: e.level,
            taskId: e.taskId,
            message: e.message,
            eventType: e.eventType,
          ))
      .toList();
}

// ── Agent tipleri ─────────────────────────────────────────────

class AgentStartResponse {
  final String executionId;
  final String agentId;
  final String status;

  const AgentStartResponse({
    required this.executionId,
    required this.agentId,
    required this.status,
  });
}

class AgentStatusResponse {
  final String executionId;
  final String agentId;
  final String objective;
  final String status;       // running | completed | failed
  final String? error;
  final int startedAt;       // ms epoch
  final int? finishedAt;

  const AgentStatusResponse({
    required this.executionId,
    required this.agentId,
    required this.objective,
    required this.status,
    this.error,
    required this.startedAt,
    this.finishedAt,
  });
}

// ── Workflow tipleri ──────────────────────────────────────────

class WorkflowStepRequest {
  final String id;
  final String name;
  final String kind;
  final String? entrypoint;
  final List<String> dependsOn;
  final bool retryable;

  const WorkflowStepRequest({
    required this.id,
    required this.name,
    required this.kind,
    this.entrypoint,
    this.dependsOn  = const [],
    this.retryable  = false,
  });
}

class WorkflowStartResponse {
  final String workflowId;
  final String name;
  final String status;

  const WorkflowStartResponse({
    required this.workflowId,
    required this.name,
    required this.status,
  });
}

class WorkflowStatusResponse {
  final String workflowId;
  final String name;
  final String status;       // running | completed | failed
  final String? error;
  final int startedAt;
  final int? finishedAt;

  const WorkflowStatusResponse({
    required this.workflowId,
    required this.name,
    required this.status,
    this.error,
    required this.startedAt,
    this.finishedAt,
  });
}

// ── Cluster tipleri ───────────────────────────────────────────

class ClusterNode {
  final String nodeId;
  final String address;
  final bool   healthy;
  final List<String> capabilities;
  final double cpuPercent;
  final int    memoryMb;
  final int    activeExecutions;

  const ClusterNode({
    required this.nodeId,
    required this.address,
    required this.healthy,
    required this.capabilities,
    required this.cpuPercent,
    required this.memoryMb,
    required this.activeExecutions,
  });
}

class ClusterStatusResponse {
  final String health;       // Healthy | Degraded | Critical
  final int    total;
  final int    healthy;
  final bool   hasQuorum;
  final String? leader;
  final List<ClusterNode> nodes;

  const ClusterStatusResponse({
    required this.health,
    required this.total,
    required this.healthy,
    required this.hasQuorum,
    this.leader,
    required this.nodes,
  });
}

class NodeRegistrationResponse {
  final String nodeId;
  final String address;
  final String status;

  const NodeRegistrationResponse({
    required this.nodeId,
    required this.address,
    required this.status,
  });
}

// ── Agent fonksiyonları ───────────────────────────────────────

Future<AgentStartResponse> startAgent({
  required String objective,
  required int maxSteps,
  required int maxTokens,
}) async {
  final r = await _bridge.startAgent(
    objective: objective,
    maxSteps:  BigInt.from(maxSteps),    // usize → BigInt
    maxTokens: BigInt.from(maxTokens),   // usize → BigInt
  );
  return AgentStartResponse(
    executionId: r.executionId,
    agentId:     r.agentId,
    status:      r.status,
  );
}

Future<AgentStatusResponse> getAgentStatus({
  required String executionId,
}) async {
  final r = await _bridge.getAgentStatus(executionId: executionId);
  return AgentStatusResponse(
    executionId: r.executionId,
    agentId:     r.agentId,
    objective:   r.objective,
    status:      r.status,
    error:       r.error,
    startedAt:   r.startedAt.toInt(),
    finishedAt:  r.finishedAt?.toInt(),
  );
}

Future<List<AgentStatusResponse>> listAgents({required int limit}) async {
  final list = await _bridge.listAgents(limit: BigInt.from(limit));
  return list.map((r) => AgentStatusResponse(
    executionId: r.executionId,
    agentId:     r.agentId,
    objective:   r.objective,
    status:      r.status,
    error:       r.error,
    startedAt:   r.startedAt.toInt(),
    finishedAt:  r.finishedAt?.toInt(),
  )).toList();
}

// ── Workflow fonksiyonları ─────────────────────────────────────

Future<WorkflowStartResponse> startWorkflow({
  required String name,
  required List<WorkflowStepRequest> steps,
}) async {
  // FRB codegen WorkflowStepRequest'i rest::router'dan aldığı için
  // bridge'in beklediği tip zaten aynı — direkt map edilebilir
  // rest_router.WorkflowStepRequest = FRB'nin codegen'den ürettiği tip
  //
  // NOT: Rust tarafında alan adı `kind` (src/api/rest/router.rs).
  // `#[serde(rename = "type")]` SADECE JSON (REST) serileştirmesini
  // etkiler; Rust struct alanının kendi adını değiştirmez. FRB codegen
  // de Dart tarafına struct alanını birebir `kind` olarak aktarır — yani
  // burada 'type' Dart'ta reserved keyword olduğu için 'type_' üretildiği
  // varsayımı yanlıştı. Üretilen sınıfta 'type_' diye bir parametre yok.
  final bridgeSteps = steps.map((s) => rest_router.WorkflowStepRequest(
    id:         s.id,
    name:       s.name,
    kind:       s.kind,
    entrypoint: s.entrypoint,
    dependsOn:  s.dependsOn,
    retryable:  s.retryable,
  )).toList();

  final r = await _bridge.startWorkflow(name: name, steps: bridgeSteps);
  return WorkflowStartResponse(
    workflowId: r.workflowId,
    name:       r.name,
    status:     r.status,
  );
}

Future<WorkflowStatusResponse> getWorkflowStatus({
  required String workflowId,
}) async {
  final r = await _bridge.getWorkflowStatus(workflowId: workflowId);
  return WorkflowStatusResponse(
    workflowId: r.workflowId,
    name:       r.name,
    status:     r.status,
    error:      r.error,
    startedAt:  r.startedAt.toInt(),
    finishedAt: r.finishedAt?.toInt(),
  );
}

Future<List<WorkflowStatusResponse>> listWorkflows({required int limit}) async {
  final list = await _bridge.listWorkflows(limit: BigInt.from(limit));
  return list.map((r) => WorkflowStatusResponse(
    workflowId: r.workflowId,
    name:       r.name,
    status:     r.status,
    error:      r.error,
    startedAt:  r.startedAt.toInt(),
    finishedAt: r.finishedAt?.toInt(),
  )).toList();
}

// ── Cluster fonksiyonları ─────────────────────────────────────

Future<ClusterStatusResponse> getClusterStatus() async {
  final r = await _bridge.getClusterStatus();
  return ClusterStatusResponse(
    health:    r.health,
    total:     r.total.toInt(),
    healthy:   r.healthy.toInt(),
    hasQuorum: r.hasQuorum,
    leader:    r.leader,
    nodes:     r.nodes.map((n) => ClusterNode(
      nodeId:           n.nodeId,
      address:          n.address,
      healthy:          n.healthy,
      capabilities:     List<String>.from(n.capabilities),
      cpuPercent:       n.cpuPercent,
      memoryMb:         n.memoryMb.toInt(),
      activeExecutions: n.activeExecutions.toInt(),
    )).toList(),
  );
}

Future<NodeRegistrationResponse> registerNode({
  required String address,
  required List<String> capabilities,
}) async {
  final r = await _bridge.registerNode(
    address:      address,
    capabilities: capabilities,
  );
  return NodeRegistrationResponse(
    nodeId:  r.nodeId,
    address: r.address,
    status:  r.status,
  );
}

// ── AI Provider fonksiyonları ──────────────────────────────────
//
// Kullanıcı Ayarlar ekranında bir provider için API key (Anthropic/
// OpenAI/Gemini) ya da host (Ollama) girip kaydettiğinde çağrılır.
// Birden fazla provider yapılandırılmışsa hangisinin kullanılacağına
// setActiveAiProvider ile kullanıcı karar verir — otomatik fallback yok.

/// Bir AI provider'ı yapılandır ve aktif provider listesine ekle.
///
/// providerId: 'anthropic' | 'openai' | 'gemini' | 'ollama'
/// apiKey:     Anthropic/OpenAI/Gemini için zorunlu, Ollama'da kullanılmaz
/// baseUrl:    sadece Ollama için (örn. 'http://127.0.0.1:11434')
/// model:      opsiyonel — verilmezse provider'ın varsayılan modeli kullanılır
Future<void> configureAiProvider({
  required String providerId,
  String? apiKey,
  String? baseUrl,
  String? model,
}) async {
  await _bridge.configureAiProvider(
    providerId: providerId,
    apiKey:     apiKey,
    baseUrl:    baseUrl,
    model:      model,
  );
}

/// Kayıtlı bir provider'ı kaldır (kullanıcı key'i sildiğinde).
Future<void> removeAiProvider({required String providerId}) async {
  await _bridge.removeAiProvider(providerId: providerId);
}

/// Birden fazla provider yapılandırılmışsa kullanılacak olanı seç.
Future<void> setActiveAiProvider({required String providerId}) async {
  await _bridge.setActiveAiProvider(providerId: providerId);
}

/// Şu an aktif olan provider id'si (hiçbiri aktif değilse null).
Future<String?> getActiveAiProvider() async {
  return _bridge.getActiveAiProvider();
}

/// Yapılandırılmış (kayıtlı) tüm provider id'leri.
Future<List<String>> listAiProviders() async {
  final list = await _bridge.listAiProviders();
  return List<String>.from(list);
}

/// Verilen Ollama sunucusunda yüklü (pull edilmiş) modelleri listele.
/// Ayarlar ekranındaki model dropdown'ını doldurmak için kullanılır.
Future<List<String>> listOllamaModels({required String baseUrl}) async {
  final list = await _bridge.listOllamaModels(baseUrl: baseUrl);
  return List<String>.from(list);
}

/// Kayıtlı bir provider'a küçük bir test isteği gönder, kısa bir
/// çıktı döner. Hata fırlatırsa key/host geçersiz demektir.
Future<String> testAiProvider({required String providerId}) async {
  return _bridge.testAiProvider(providerId: providerId);
}

/// Genel amaçlı, tek seferlik AI sohbet isteği — AKTİF provider üzerinden
/// çalışır. Hangi provider'ın yanıt vereceği kullanıcının Ayarlar'da
/// seçtiği aktif modele bağlıdır.
Future<String> aiChat({
  required String prompt,
  String? systemPrompt,
  int maxTokens = 1024,
}) async {
  return _bridge.aiChat(
    prompt: prompt,
    systemPrompt: systemPrompt,
    maxTokens: BigInt.from(maxTokens), // usize → BigInt
  );
}

// ── Bekleyen onaylar (B5) ────────────────────────────────────
//
// Rust: bridge::api::list_pending_approvals / respond_to_approval.
// SecurityGovernor "RequiresApproval" dediğinde agent duraklar ve burada
// görünür; kullanıcı onaylarsa kalan adımlarla devam eder, reddederse
// execution kalıcı olarak reddedilir. Onaylar uygulama yeniden
// başlatılsa da korunur ve ~60 dk sonra (TTL) otomatik reddedilir.

class PendingApproval {
  final String id;
  final String executionId;
  final String agentId;
  final String objective;
  final String toolName;
  final List<String> arguments;
  final String reason;
  final int createdAt; // ms epoch

  const PendingApproval({
    required this.id,
    required this.executionId,
    required this.agentId,
    required this.objective,
    required this.toolName,
    required this.arguments,
    required this.reason,
    required this.createdAt,
  });
}

Future<List<PendingApproval>> listPendingApprovals() async {
  final list = await _bridge.listPendingApprovals();
  return list
      .map((r) => PendingApproval(
            id:          r.id,
            executionId: r.executionId,
            agentId:     r.agentId,
            objective:   r.objective,
            toolName:    r.toolName,
            arguments:   List<String>.from(r.arguments),
            reason:      r.reason,
            createdAt:   r.createdAt.toInt(),
          ))
      .toList();
}

Future<void> respondToApproval({
  required String approvalId,
  required bool approved,
}) =>
    _bridge.respondToApproval(approvalId: approvalId, approved: approved);
