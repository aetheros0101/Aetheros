// ============================================================
// flutter_app/lib/core/audit_format.dart
//
// Denetim kaydı (audit) gösterim yardımcıları — saf Dart, test edilebilir.
// Rust tarafı `details_json`'u serde'nin varsayılan (dıştan etiketli)
// biçiminde verir:  {"ToolInvoked":{...}}  veya  "ExecutionCompleted".
// ============================================================

import 'dart:convert';

/// Bir tool çıktısının özeti (Rust `OutputSummary`).
class OutputPreview {
  final int bytes;
  final String preview;
  const OutputPreview({required this.bytes, required this.preview});
}

/// `details_json` içinden `ToolInvoked.output` özetini çıkarır.
/// Başka tür olay, çıktısız çağrı ya da bozuk JSON → null.
OutputPreview? parseOutputPreview(String detailsJson) {
  try {
    final root = jsonDecode(detailsJson);
    if (root is! Map) return null;
    final inv = root['ToolInvoked'];
    if (inv is! Map) return null;
    final out = inv['output'];
    if (out is! Map) return null;
    final bytes = out['bytes'];
    final preview = out['preview'];
    return OutputPreview(
      bytes: bytes is num ? bytes.toInt() : 0,
      preview: preview is String ? preview : '',
    );
  } catch (_) {
    return null;
  }
}

/// `kind_label` → kısa Türkçe başlık.
String auditTitle(String kindLabel) {
  switch (kindLabel) {
    case 'governor_decision':
      return 'Güvenlik kararı';
    case 'tool_invoked':
      return 'Araç çalıştı';
    case 'execution_paused':
      return 'Onay bekliyor';
    case 'execution_resumed':
      return 'Onaylandı, devam';
    case 'approval_denied':
      return 'Reddedildi';
    case 'execution_completed':
      return 'Tamamlandı';
    case 'execution_failed':
      return 'Başarısız';
    default:
      return kindLabel;
  }
}

/// Olayın kötü sonuçlu olup olmadığı (kırmızı gösterim için).
bool auditIsProblem(String kindLabel, String summary) {
  if (kindLabel == 'execution_failed' || kindLabel == 'approval_denied') {
    return true;
  }
  if (kindLabel == 'tool_invoked' && summary.contains('başarısız')) return true;
  if (kindLabel == 'governor_decision' && summary.contains('→ deny')) return true;
  return false;
}

/// Çıktı özetini tek satırlık insan diline çevirir.
String outputLabel(OutputPreview o) {
  if (o.bytes == 0) return 'Çıktı yok (komut sessizce başarılı olabilir).';
  final p = o.preview.trim();
  return p.isEmpty ? '${o.bytes} bayt çıktı' : '${o.bytes} bayt · $p';
}

/// ms epoch → "SS:DD:SS" (yerel saat).
String clockLabel(int ms) {
  final t = DateTime.fromMillisecondsSinceEpoch(ms);
  String two(int n) => n.toString().padLeft(2, '0');
  return '${two(t.hour)}:${two(t.minute)}:${two(t.second)}';
}

/// Denetim ekranı filtresi.
enum AuditFilter { all, problems, tools, approvals }

String auditFilterLabel(AuditFilter f) {
  switch (f) {
    case AuditFilter.all:
      return 'Tümü';
    case AuditFilter.problems:
      return 'Sorunlu';
    case AuditFilter.tools:
      return 'Araç';
    case AuditFilter.approvals:
      return 'Onay';
  }
}

/// Olay, seçili filtreye uyuyor mu?
bool auditMatches(AuditFilter f, String kindLabel, String summary) {
  switch (f) {
    case AuditFilter.all:
      return true;
    case AuditFilter.problems:
      return auditIsProblem(kindLabel, summary);
    case AuditFilter.tools:
      return kindLabel == 'tool_invoked' || kindLabel == 'governor_decision';
    case AuditFilter.approvals:
      return kindLabel == 'execution_paused' ||
          kindLabel == 'execution_resumed' ||
          kindLabel == 'approval_denied';
  }
}

/// UUID'nin ilk 8 hanesi (listelerde kısa gösterim).
String shortId(String id) => id.length <= 8 ? id : id.substring(0, 8);
