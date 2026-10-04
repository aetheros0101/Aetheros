// ============================================================
// flutter_app/lib/core/agent_capabilities.dart
//
// B11: Agent başlatılırken seçilebilen yetkiler. `id` değerleri Rust
// tarafındaki `bridge::api::parse_capability` ile BİREBİR aynı olmalı;
// bilinmeyen bir id `start_agent`'ı hata ile reddettirir.
//
// Saf Dart (Flutter'dan bağımsız) — `flutter test` ile test edilir.
// ============================================================

class CapabilityOption {
  /// Rust'a giden ad (ör. "terminal_execution").
  final String id;
  final String label;
  final String description;

  /// true ise seçildiğinde kullanıcıya ayrıca uyarı gösterilir.
  final bool sensitive;

  const CapabilityOption({
    required this.id,
    required this.label,
    required this.description,
    this.sensitive = false,
  });
}

const List<CapabilityOption> kAgentCapabilities = [
  CapabilityOption(
    id: 'terminal_execution',
    label: 'Terminal',
    description:
        'Cihazda komut çalıştırır. Güvenli okuma komutları (ör. git status) '
        'onaysız çalışır; diğerleri Bekleyen Onaylar\'a düşer. sh/python gibi '
        'yorumlayıcılar her zaman reddedilir.',
    sensitive: true,
  ),
  CapabilityOption(
    id: 'wasm_execution',
    label: 'WASM',
    description: 'Kayıtlı WASM modüllerini / script\'leri çalıştırır.',
  ),
  CapabilityOption(
    id: 'workflow_execution',
    label: 'Workflow',
    description: 'Workflow başlatabilir.',
  ),
  CapabilityOption(
    id: 'remote_execution',
    label: 'Uzak düğüm',
    description: 'Cluster\'daki diğer düğümlerde iş çalıştırabilir.',
    sensitive: true,
  ),
  CapabilityOption(
    id: 'ai_reasoning',
    label: 'AI akıl yürütme',
    description: 'AI sağlayıcısını çağırabilen araçlara izin verir.',
  ),
];

/// Seçili id'lerin kataloğa göre sıralı ve yalnız GEÇERLİ olanlarını döner
/// (UI dışı kaynaktan gelen bozuk bir id Rust'a hiç gitmesin).
List<String> normalizeCapabilities(Iterable<String> ids) {
  final wanted = ids.toSet();
  return [
    for (final c in kAgentCapabilities)
      if (wanted.contains(c.id)) c.id,
  ];
}

/// "Yetki yok" / "Terminal, WASM".
String capabilitySummary(Iterable<String> ids) {
  final labels = [
    for (final c in kAgentCapabilities)
      if (ids.contains(c.id)) c.label,
  ];
  return labels.isEmpty ? 'Yetki yok' : labels.join(', ');
}

/// Terminal yetkisinin Rust'taki adı (Chat'ten "Agent ile yap" önceden seçer).
const String kTerminalCapabilityId = 'terminal_execution';
