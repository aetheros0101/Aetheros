// ============================================================
// flutter_app/lib/core/approval_format.dart
//
// B5: onay ekranının SAF (Flutter'dan bağımsız) biçimlendirme
// yardımcıları — `flutter test` ile kolayca test edilebilsin diye
// ayrı dosyada.
//
// ÖNEMLİ: Burada üretilen komut satırı yalnızca GÖSTERİM içindir.
// Runtime komutu shell'e VERMEZ; argüman listesi olarak çalıştırır
// (bkz. Rust tarafı terminal_tool.rs). Tırnaklama, kullanıcının
// argüman sınırlarını (boşluklu argüman vb.) net görmesi içindir.
// ============================================================

/// Rust tarafındaki varsayılan onay süresi (DEFAULT_APPROVAL_TTL_MINUTES).
/// Doğruluk kaynağı Rust'tır: süresi dolan onay sonraki yoklamada listeden
/// kaybolur. Buradaki değer yalnızca "yaklaşık kalan süre" göstermek için.
const Duration kApprovalTtl = Duration(minutes: 60);

final RegExp _safeArg = RegExp(r'^[A-Za-z0-9_./:=@%+,\-~]+$');

/// Tek bir argümanı gösterim için tırnaklar: güvenli karakterlerse olduğu
/// gibi, aksi halde tek tırnak içinde (içindeki `'` → `'\''`).
String quoteArg(String arg) {
  if (arg.isEmpty) return "''";
  if (_safeArg.hasMatch(arg)) return arg;
  return "'${arg.replaceAll("'", r"'\''")}'";
}

/// `terminal` aracı için `["git","commit","-m","a b"]` → `git commit -m 'a b'`.
/// Diğer araçlar için `araç_adı arg1 arg2`.
String commandLine(String toolName, List<String> arguments) {
  final parts = arguments.map(quoteArg).join(' ');
  if (toolName == 'terminal') {
    return parts.isEmpty ? '(boş komut)' : parts;
  }
  return parts.isEmpty ? toolName : '$toolName $parts';
}

/// "az önce", "5 dk önce", "2 sa önce", "3 gün önce".
String ageLabel(int createdAtMs, int nowMs) {
  final diff = Duration(milliseconds: (nowMs - createdAtMs).clamp(0, 1 << 52).toInt());
  if (diff.inSeconds < 45) return 'az önce';
  if (diff.inMinutes < 60) return '${diff.inMinutes} dk önce';
  if (diff.inHours < 24) return '${diff.inHours} sa önce';
  return '${diff.inDays} gün önce';
}

/// Yaklaşık kalan süre etiketi; süre dolduysa `null` (liste zaten
/// bir sonraki yoklamada onu kaldıracaktır).
String? remainingLabel(int createdAtMs, int nowMs, {Duration ttl = kApprovalTtl}) {
  final left = ttl - Duration(milliseconds: (nowMs - createdAtMs).clamp(0, 1 << 52).toInt());
  if (left <= Duration.zero) return null;
  if (left.inMinutes < 1) return '<1 dk kaldı';
  return '~${left.inMinutes} dk kaldı';
}
