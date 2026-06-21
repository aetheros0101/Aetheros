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

  /// Modülün runtime'da hazır olup olmadığını kontrol et.
  ///
  /// Fix #1 sonrası startup'ta sled'den yüklenir.
  /// Yine de eski oturumdan kalan meta-data'yı doğrulamak için
  /// WasmModuleScreen açılışında her modül için çağrılır.
  static Future<bool> checkModuleExists(String hashHex) async {
    return rust.checkModuleExists(hashHex: hashHex);
  }

  /// WAT (WebAssembly Text Format) kaynak kodunu derle → WASM binary →
  /// ModuleStore'a kaydet. Script editörünün "Derle & Yükle" butonu için.
  ///
  /// Başarı: ModuleUploadResponse { hash, size }
  /// Hata:   WAT sözdizimi hatası mesajı (kullanıcıya gösterilebilir)
  static Future<rust.ModuleUploadResponse> compileWatToWasm({
    required String name,
    required String watSource,
    required String entrypoint,
    int timeoutMs = 30000,
  }) async {
    // NOT: Bu metod FRB codegen'den geçtikten sonra
    // _bridge.compileWatToWasm() çağrısı aktif olacak.
    // Şimdilik adaptör katmanı üzerinden çağrılıyor.
    return rust.compileWatToWasm(
      name:       name,
      watSource:  watSource,
      entrypoint: entrypoint,
      timeoutMs:  timeoutMs,
    );
  }

  /// Mevcut bir task'ı yeni UUID ile yeniden kuyruğa ekle.
  ///
  /// "Yeniden Dene" butonu için — orijinal ayarlar (hash,
  /// entrypoint, priority) korunur, sadece id yenilenir.
  static Future<String> resubmitTask(String taskId) async {
    return rust.resubmitTask(taskId: taskId);
  }

  // ── Log izleme ─────────────────────────────────────────

  /// Son `limit` kadar log entry döndür (yeniden eskiye).
  ///
  /// LogScreen 2 saniyede bir bu fonksiyonu polling ile çeker.
  /// limit: 0 → varsayılan 100.
  static Future<List<rust.LogRecord>> getRecentLogs({int limit = 100}) async {
    return rust.getRecentLogs(limit: limit);
  }

  /// Belirli bir task'a ait log entry'leri döndür.
  ///
  /// Task detay modalındaki "Loglar" sekmesi için.
  static Future<List<rust.LogRecord>> getTaskLogs(
    String taskId, {
    int limit = 50,
  }) async {
    return rust.getTaskLogs(taskId: taskId, limit: limit);
  }

  // ── Metrikler ──────────────────────────────────────────

  static Future<rust.MetricsSnapshot> getMetrics() async {
    return rust.getMetrics();
  }
}
