// ============================================================
// flutter_app/lib/src/rust/frb_generated.dart
//
// FRB v2.9.0 minimal stub.
// main.dart'ın ihtiyacı: RustLib.init()
//
// Gerçek codegen: flutter_rust_bridge_codegen generate
// Şu an: stub, APK derlemek için yeterli.
// ============================================================

// ignore_for_file: invalid_use_of_internal_member, unused_import

/// AetherOS Flutter-Rust Bridge giriş noktası.
///
/// Gerçek uygulamada bu sınıf flutter_rust_bridge_codegen
/// tarafından otomatik üretilir ve libaetheros.so'ya bağlanır.
/// Şu an stub implementasyon — API yüzeyi aynı.
class RustLib {
  static bool _initialized = false;

  /// FRB runtime'ını başlatır.
  /// Stub: gerçek implementasyon codegen sonrası .so'ya bağlanır.
  static Future<void> init() async {
    if (_initialized) return;
    _initialized = true;
  }

  static bool get isInitialized => _initialized;
}
