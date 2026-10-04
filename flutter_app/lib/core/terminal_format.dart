// ============================================================
// flutter_app/lib/core/terminal_format.dart
//
// Kullanıcı terminali için saf Dart yardımcılar (test edilebilir).
// ============================================================

/// Rust politikasının kararı.
enum TerminalVerdict { allow, ask, deny }

/// Rust'tan gelen karar metnini çözer. Bilinmeyen değer GÜVENLİ tarafa
/// düşer: `deny` (çalıştırma). Asıl zorlama zaten Rust'tadır; bu yalnızca
/// arayüzün yanlış bir "çalıştı" izlenimi vermemesi içindir.
TerminalVerdict parseVerdict(String raw) {
  switch (raw) {
    case 'allow':
      return TerminalVerdict.allow;
    case 'ask':
      return TerminalVerdict.ask;
    default:
      return TerminalVerdict.deny;
  }
}

/// Hızlı komut çipleri (hepsi varsayılan politikada onaysız çalışır).
const List<String> kQuickCommands = ['ls', 'pwd', 'date', 'whoami'];

/// Çıktıyı ekranda ilk N satırla sınırlar (çok uzun çıktı UI'ı kilitlemesin).
/// Kırpıldıysa `(… N satır daha)` notu eklenir.
String limitLines(String text, {int maxLines = 200}) {
  final lines = text.split('\n');
  if (lines.length <= maxLines) return text;
  final shown = lines.take(maxLines).join('\n');
  return '$shown\n(… ${lines.length - maxLines} satır daha)';
}
