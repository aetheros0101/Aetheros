// ============================================================
// flutter_app/lib/src/rust/api/aetheros.dart
//
// ⚠️  BU DOSYA OTOMATİK ÜRETİLİR — EL İLE DÜZENLEMEYİN
//
// Gerçek dosya şu komutla üretilir:
//   cd flutter_app && dart run flutter_rust_bridge_codegen generate
//
// Bu stub; IDE'nin kırmızı çizgi göstermeden çalışması ve
// uygulama yapısını test edebilmek için var.
// ============================================================

// ignore_for_file: unused_import, camel_case_types

// ── Tipler ───────────────────────────────────────────────

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

// ── Fonksiyonlar (codegen sonrası gerçek implementasyona bağlanır) ──

Future<void> initializeRuntime({
  required String dbPath,
  required int workerCount,
}) async {}

bool isRuntimeReady() => false;

RuntimeInfo getRuntimeInfo() => const RuntimeInfo(
      version: '0.1.0',
      isRunning: false,
      backend: 'wasmi',
      workerCount: 0,
    );

Future<String> submitTask({required TaskRequest request}) async =>
    throw UnimplementedError('codegen bekleniyor');

Future<TaskStatusResponse> getTaskStatus({required String taskId}) async =>
    throw UnimplementedError('codegen bekleniyor');

Future<List<TaskStatusResponse>> listTasks({required int limit}) async => [];

Future<ModuleUploadResponse> uploadWasmModule({
  required List<int> bytes,
}) async =>
    throw UnimplementedError('codegen bekleniyor');

Future<MetricsSnapshot> getMetrics() async => const MetricsSnapshot(
      activeWorkers: 0,
      queuedTasks: 0,
      completedTasks: 0,
      failedTasks: 0,
      retriedTasks: 0,
    );
