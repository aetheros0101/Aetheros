// ============================================================
// flutter_app/lib/core/load_error_banner.dart
//
// Liste yükleyen ekranlar hata alınca eskiden sessizce boş görünüyordu
// (`catch (_)`). Bu yardımcı, hata sürdüğü sürece ekranın üstünde bir
// MaterialBanner gösterir; yükleme tekrar başarılı olunca kendiliğinden
// kalkar. Aynı mesaj tekrar tekrar yeniden çizilmez (3 sn'lik polling
// banner'ı titretmesin).
// ============================================================

import 'package:flutter/material.dart';

import 'app_error.dart';

final class LoadErrorBanner {
  ScaffoldMessengerState? _messenger;
  String? _shown;

  /// `didChangeDependencies` içinde çağır.
  void attach(BuildContext context) {
    _messenger = ScaffoldMessenger.maybeOf(context);
  }

  /// Yükleme başarısız oldu.
  void report(Object error, {VoidCallback? onRetry}) {
    final m = _messenger;
    if (m == null) return;
    final msg = userFacingError(error);
    if (msg == _shown) return; // aynı hata zaten gösteriliyor
    _shown = msg;
    m
      ..hideCurrentMaterialBanner()
      ..showMaterialBanner(MaterialBanner(
        content: Text('Güncellenemedi: $msg'),
        leading: const Icon(Icons.cloud_off, color: Colors.amber),
        backgroundColor: const Color(0xFF2A2415),
        actions: [
          if (onRetry != null)
            TextButton(
              onPressed: () {
                clear();
                onRetry();
              },
              child: const Text('Tekrar dene'),
            ),
          TextButton(onPressed: clear, child: const Text('Kapat')),
        ],
      ));
  }

  /// Yükleme başarılı oldu (ya da kullanıcı kapattı).
  void clear() {
    if (_shown == null) return;
    _shown = null;
    _messenger?.hideCurrentMaterialBanner();
  }

  /// `dispose` içinde çağır. Ağaç kilitliyken setState tetiklememek için
  /// banner bir sonraki karede kaldırılır.
  void dispose() {
    final m = _messenger;
    final hadBanner = _shown != null;
    _messenger = null;
    _shown = null;
    if (m != null && hadBanner) {
      WidgetsBinding.instance
          .addPostFrameCallback((_) => m.hideCurrentMaterialBanner());
    }
  }
}
