// ============================================================
// flutter_app/lib/screens/audit_screen.dart
//
// Faz 2 — Denetim Kayıtları
//
// Tüm agent'ların güvenlik kararı, onay, araç çalıştırma ve sonuç
// olayları tek akışta (en yeni üstte). Filtre: Tümü / Sorunlu / Araç / Onay.
// Kayıtlar Rust tarafında kalıcıdır (sled); ham çıktı değil, özet tutulur.
// ============================================================

import 'package:flutter/material.dart';

import '../api/aetheros_api.dart';
import '../core/app_error.dart';
import '../core/app_theme.dart';
import '../core/audit_format.dart';
import '../core/lifecycle_poller.dart';
import '../src/rust/api/aetheros.dart' as rust;

class AuditScreen extends StatefulWidget {
  const AuditScreen({super.key});

  @override
  State<AuditScreen> createState() => _AuditScreenState();
}

class _AuditScreenState extends State<AuditScreen> {
  List<rust.AuditEvent> _events = [];
  bool _loading = true;
  String? _error;
  AuditFilter _filter = AuditFilter.all;
  late final LifecyclePoller _poller;

  @override
  void initState() {
    super.initState();
    _poller = LifecyclePoller(
      interval: const Duration(seconds: 3),
      onTick: _load,
    );
    _poller.start();
  }

  @override
  void dispose() {
    _poller.stop();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final ev = await AetherApi.listAllAuditEvents();
      ev.sort((a, b) => b.createdAt.compareTo(a.createdAt)); // en yeni üstte
      if (!mounted) return;
      setState(() {
        _events = ev;
        _loading = false;
        _error = null;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = userFacingError(e);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final shown = _events
        .where((e) => auditMatches(_filter, e.kindLabel, e.summary))
        .toList();

    return Scaffold(
      backgroundColor: AetherColors.background,
      appBar: AppBar(
        title: const Text('Denetim Kayıtları'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: 'Yenile',
            onPressed: _load,
          ),
        ],
      ),
      body: Column(children: [
        SizedBox(
          height: 48,
          child: ListView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(
                horizontal: AetherSpacing.lg, vertical: 6),
            children: [
              for (final f in AuditFilter.values)
                Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: ChoiceChip(
                    label: Text(auditFilterLabel(f)),
                    selected: _filter == f,
                    onSelected: (_) => setState(() => _filter = f),
                  ),
                ),
            ],
          ),
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.lg),
            child: Text('Liste güncellenemedi: $_error',
                style: const TextStyle(
                    color: AetherColors.warning, fontSize: 12)),
          ),
        Expanded(child: _body(shown)),
      ]),
    );
  }

  Widget _body(List<rust.AuditEvent> shown) {
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (shown.isEmpty) {
      return const Center(
        child: Text('Kayıt yok',
            style: TextStyle(color: AetherColors.textMuted, fontSize: 15)),
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.builder(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(AetherSpacing.lg),
        itemCount: shown.length,
        itemBuilder: (_, i) => AuditTile(event: shown[i], showExecution: true),
      ),
    );
  }
}

/// Tek denetim olayı kartı (genel ekran + agent detayı ortak kullanır).
class AuditTile extends StatelessWidget {
  final rust.AuditEvent event;
  final bool showExecution;
  const AuditTile({super.key, required this.event, this.showExecution = false});

  @override
  Widget build(BuildContext context) {
    final problem = auditIsProblem(event.kindLabel, event.summary);
    final color = problem ? const Color(0xFFFF5252) : Colors.white;
    final out = parseOutputPreview(event.detailsJson);
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: Colors.black26,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
            color: problem ? const Color(0x55FF5252) : Colors.white12),
      ),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [
          Expanded(
            child: Text(auditTitle(event.kindLabel),
                style: TextStyle(
                    color: color, fontWeight: FontWeight.bold, fontSize: 13)),
          ),
          Text(clockLabel(event.createdAt),
              style: const TextStyle(color: Colors.white38, fontSize: 11)),
        ]),
        if (showExecution)
          Text('execution ${shortId(event.executionId)}',
              style: const TextStyle(color: Colors.white38, fontSize: 11)),
        const SizedBox(height: 4),
        SelectableText(event.summary,
            style: const TextStyle(
                color: Colors.white70,
                fontFamily: 'monospace',
                fontSize: 12,
                height: 1.35)),
        if (out != null) ...[
          const SizedBox(height: 4),
          Text(outputLabel(out),
              style: const TextStyle(color: Colors.white54, fontSize: 11)),
        ],
      ]),
    );
  }
}
