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
