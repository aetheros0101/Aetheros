/// AetherOS Application State — Bottom / Side Panel Content
class PanelState {
  const PanelState({
    this.panels = const [
      PanelViewState(id: 'terminal', label: 'Terminal'),
      PanelViewState(id: 'problems', label: 'Problems'),
      PanelViewState(id: 'output', label: 'Output'),
      PanelViewState(id: 'debug', label: 'Debug Console'),
    ],
    this.activePanelId = 'terminal',
  });

  final List<PanelViewState> panels;
  final String activePanelId;

  PanelViewState? get activePanel {
    for (final p in panels) {
      if (p.id == activePanelId) return p;
    }
    return panels.isEmpty ? null : panels.first;
  }

  PanelState activate(String id) {
    if (!panels.any((p) => p.id == id)) return this;
    return copyWith(activePanelId: id);
  }

  PanelState setBadge(String id, int count) {
    final updated = panels.map((p) => p.id == id ? p.copyWith(badgeCount: count) : p).toList();
    return copyWith(panels: updated);
  }

  PanelState copyWith({List<PanelViewState>? panels, String? activePanelId}) =>
      PanelState(panels: panels ?? this.panels, activePanelId: activePanelId ?? this.activePanelId);

  Map<String, dynamic> toJson() => {'panels': panels.map((p) => p.toJson()).toList(), 'activePanelId': activePanelId};

  factory PanelState.fromJson(Map<String, dynamic> json) {
    final raw = json['panels'] as List<dynamic>?;
    return PanelState(
      panels: raw == null
          ? const [
              PanelViewState(id: 'terminal', label: 'Terminal'),
              PanelViewState(id: 'problems', label: 'Problems'),
              PanelViewState(id: 'output', label: 'Output'),
              PanelViewState(id: 'debug', label: 'Debug Console'),
            ]
          : raw.map((e) => PanelViewState.fromJson(e as Map<String, dynamic>)).toList(),
      activePanelId: json['activePanelId'] as String? ?? 'terminal',
    );
  }
}

class PanelViewState {
  const PanelViewState({required this.id, required this.label, this.badgeCount = 0});
  final String id;
  final String label;
  final int badgeCount;
  PanelViewState copyWith({String? id, String? label, int? badgeCount}) =>
      PanelViewState(id: id ?? this.id, label: label ?? this.label, badgeCount: badgeCount ?? this.badgeCount);
  Map<String, dynamic> toJson() => {'id': id, 'label': label, 'badgeCount': badgeCount};
  factory PanelViewState.fromJson(Map<String, dynamic> json) => PanelViewState(
        id: json['id'] as String? ?? '', label: json['label'] as String? ?? '',
        badgeCount: json['badgeCount'] as int? ?? 0);
}
