// ============================================================
// flutter_app/lib/api/aetheros_api.dart
//
// Rust bridge üzerindeki tüm Dart çağrılarını tek yerde toplar.
// Ekranlar doğrudan frb_generated'ı değil, bu sınıfı çağırır.
//
// KULLANIM (herhangi bir ekranda):
//   final taskId = await AetherApi.submitTask(...);
//   final status = await AetherApi.getTaskStatus(taskId);
//   final metrics = await AetherApi.getMetrics();
// ============================================================

import 'package:aetheros_app/src/rust/api/aetheros.dart' as rust;

class AetherApi {
  AetherApi._();

  // ── Runtime ───────────────────────────────────────────

  static Future<bool> isReady() async {
    return rust.isRuntimeReady();
  }

  static Future<rust.RuntimeInfo> getRuntimeInfo() async {
    return rust.getRuntimeInfo();
  }

  // ── Task yönetimi ──────────────────────────────────────

  /// Task gönder. Başarılıysa task_id döner.
  static Future<String> submitTask({
    required String entrypoint,
    String wasmModuleHash = '',
    String priority = 'normal',
    int timeoutMs = 30000,
    int maxRetries = 3,
  }) async {
    return rust.submitTask(
      request: rust.TaskRequest(
        wasmModuleHash: wasmModuleHash,
        entrypoint: entrypoint,
        priority: priority,
        timeoutMs: timeoutMs,
        maxRetries: maxRetries,
      ),
    );
  }

  /// Task durumunu sorgula.
  static Future<rust.TaskStatusResponse> getTaskStatus(
    String taskId,
  ) async {
    return rust.getTaskStatus(taskId: taskId);
  }

  /// Son N task'ı listele.
  static Future<List<rust.TaskStatusResponse>> listTasks({
    int limit = 50,
  }) async {
    return rust.listTasks(limit: limit);
  }

  // ── Modül yönetimi ─────────────────────────────────────

  /// WASM binary yükle → SHA-256 hash döner.
  static Future<rust.ModuleUploadResponse> uploadWasmModule({
    required List<int> bytes,
  }) async {
    return rust.uploadWasmModule(bytes: bytes);
  }

  /// Eski isim — geriye dönük uyumluluk için korundu.
  static Future<rust.ModuleUploadResponse> uploadModule(
    List<int> bytes,
  ) async {
    return rust.uploadWasmModule(bytes: bytes);
  }

  // ── Metrikler ──────────────────────────────────────────

  static Future<rust.MetricsSnapshot> getMetrics() async {
    return rust.getMetrics();
  }
}
