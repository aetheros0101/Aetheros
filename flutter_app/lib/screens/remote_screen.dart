// ============================================================
// flutter_app/lib/screens/remote_screen.dart
//
// Faz-1 UI — Cluster / Remote yönetimi
//   • Cluster sağlık durumu
//   • Node listesi (IP, kapasite, CPU/bellek)
//   • Yeni node kayıt
// ============================================================

import 'dart:async';
import 'package:flutter/material.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;

class RemoteScreen extends StatefulWidget {
  const RemoteScreen({super.key});
  @override
  State<RemoteScreen> createState() => _RemoteScreenState();
}

class _RemoteScreenState extends State<RemoteScreen> {
  rust.ClusterStatusResponse? _cluster;
  bool   _loading = true;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _load();
    _timer = Timer.periodic(const Duration(seconds: 5), (_) => _load(silent: true));
  }

  @override
  void dispose() { _timer?.cancel(); super.dispose(); }

  Future<void> _load({bool silent = false}) async {
    if (!silent && mounted) setState(() => _loading = true);
    try {
      final c = await AetherApi.getClusterStatus();
      if (mounted) setState(() { _cluster = c; _loading = false; });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _showRegisterDialog() {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: const Color(0xFF1A1A2E),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => _RegisterNodeSheet(onRegistered: _load),
    );
  }

  @override
  Widget build(BuildContext context) {
    final health     = _cluster?.health ?? 'unknown';
    final healthColor = _healthColor(health);

    return Scaffold(
      backgroundColor: const Color(0xFF0D0D1A),
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: const Text('Cluster / Remote',
            style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh, color: Colors.white70),
            onPressed: _load,
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _showRegisterDialog,
        backgroundColor: const Color(0xFF26A69A),
        icon: const Icon(Icons.add_link, color: Colors.white),
        label: const Text('Node Ekle', style: TextStyle(color: Colors.white)),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator(
              color: Color(0xFF26A69A)))
          : ListView(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 100),
              children: [
                // ── Cluster genel durum kartı ──────────────
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: const Color(0xFF1A1A2E),
                    borderRadius: BorderRadius.circular(16),
                    border: Border.all(color: healthColor.withOpacity(0.4)),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(children: [
                        Container(
                          width: 10, height: 10,
                          decoration: BoxDecoration(
                            color: healthColor,
                            shape: BoxShape.circle,
                            boxShadow: [BoxShadow(
                              color: healthColor.withOpacity(0.5),
                              blurRadius: 6,
                            )],
                          ),
                        ),
                        const SizedBox(width: 8),
                        Text(
                          _healthLabel(health),
                          style: TextStyle(
                            color: healthColor,
                            fontWeight: FontWeight.bold,
                            fontSize: 16,
                          ),
                        ),
                        const Spacer(),
                        if (_cluster?.hasQuorum == true)
                          Container(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 8, vertical: 3),
                            decoration: BoxDecoration(
                              color: const Color(0xFF4CAF50).withOpacity(0.15),
                              borderRadius: BorderRadius.circular(8),
                            ),
                            child: const Text('Quorum ✓',
                                style: TextStyle(
                                    color: Color(0xFF4CAF50), fontSize: 11)),
                          ),
                      ]),
                      const SizedBox(height: 12),
                      Row(children: [
                        _StatPill(
                          label: 'Toplam Node',
                          value: '${_cluster?.total ?? 0}',
                          color: Colors.white70,
                        ),
                        const SizedBox(width: 10),
                        _StatPill(
                          label: 'Sağlıklı',
                          value: '${_cluster?.healthy ?? 0}',
                          color: const Color(0xFF4CAF50),
                        ),
                        if (_cluster?.leader != null) ...[
                          const SizedBox(width: 10),
                          _StatPill(
                            label: 'Lider',
                            value: (_cluster!.leader!)
                                .substring(0, 8)
                                .toUpperCase(),
                            color: const Color(0xFF26A69A),
                          ),
                        ],
                      ]),
                    ],
                  ),
                ),
                const SizedBox(height: 20),

                // ── Node listesi ──────────────────────────────
                const Text('Node\'lar',
                    style: TextStyle(color: Colors.white70,
                        fontSize: 15, fontWeight: FontWeight.w700)),
                const SizedBox(height: 10),

                if (_cluster?.nodes.isEmpty ?? true)
                  _EmptyNodes(onAdd: _showRegisterDialog)
                else
                  ...(_cluster!.nodes.map((n) => _NodeCard(node: n))),
              ],
            ),
    );
  }

  Color _healthColor(String h) => switch (h) {
    'Healthy'  => const Color(0xFF4CAF50),
    'Degraded' => const Color(0xFFFF9800),
    'Critical' => const Color(0xFFFF5252),
    _          => Colors.white38,
  };

  String _healthLabel(String h) => switch (h) {
    'Healthy'  => 'Cluster Sağlıklı',
    'Degraded' => 'Cluster Bozuk',
    'Critical' => 'Cluster Kritik',
    _          => 'Durum Bilinmiyor',
  };
}

// ── Node kartı ─────────────────────────────────────────────────

class _NodeCard extends StatelessWidget {
  final rust.ClusterNode node;
  const _NodeCard({required this.node});

  @override
  Widget build(BuildContext context) {
    final healthy = node.healthy;
    final color   = healthy
        ? const Color(0xFF4CAF50)
        : const Color(0xFFFF5252);

    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFF1A1A2E),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: color.withOpacity(0.3)),
      ),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        // Üst satır
        Row(children: [
          Container(
            width: 8, height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              node.address,
              style: const TextStyle(color: Colors.white,
                  fontWeight: FontWeight.bold, fontFamily: 'monospace'),
            ),
          ),
          Text(
            node.nodeId.substring(0, 8).toUpperCase(),
            style: const TextStyle(color: Colors.white38,
                fontSize: 11, fontFamily: 'monospace'),
          ),
        ]),
        const SizedBox(height: 10),

        // Kapasiteler
        if (node.capabilities.isNotEmpty)
          Wrap(spacing: 6, runSpacing: 4,
            children: node.capabilities.map((c) => _CapChip(c)).toList()),

        const SizedBox(height: 8),

        // Metrikler
        Row(children: [
          _Metric(label: 'CPU',
              value: '${node.cpuPercent.toStringAsFixed(1)}%',
              color: _cpuColor(node.cpuPercent)),
          const SizedBox(width: 16),
          _Metric(label: 'Bellek',
              value: '${node.memoryMb} MB',
              color: Colors.white60),
          const SizedBox(width: 16),
          _Metric(label: 'Çalışan',
              value: '${node.activeExecutions}',
              color: node.activeExecutions > 0
                  ? const Color(0xFF6C63FF)
                  : Colors.white38),
        ]),
      ]),
    );
  }

  Color _cpuColor(double pct) {
    if (pct < 50) return const Color(0xFF4CAF50);
    if (pct < 80) return const Color(0xFFFF9800);
    return const Color(0xFFFF5252);
  }
}

// ── Node kayıt formu ──────────────────────────────────────────

class _RegisterNodeSheet extends StatefulWidget {
  final VoidCallback onRegistered;
  const _RegisterNodeSheet({required this.onRegistered});
  @override
  State<_RegisterNodeSheet> createState() => _RegisterNodeSheetState();
}

class _RegisterNodeSheetState extends State<_RegisterNodeSheet> {
  final _addrCtrl = TextEditingController(text: 'localhost:8081');
  final Set<String> _caps = {'wasm'};
  bool _submitting = false;
  String? _error;

  @override
  void dispose() { _addrCtrl.dispose(); super.dispose(); }

  Future<void> _submit() async {
    if (_addrCtrl.text.trim().isEmpty) {
      setState(() => _error = 'Adres boş olamaz.');
      return;
    }
    setState(() { _submitting = true; _error = null; });
    try {
      await AetherApi.registerNode(
        address:      _addrCtrl.text.trim(),
        capabilities: _caps.toList(),
      );
      if (mounted) {
        Navigator.pop(context);
        widget.onRegistered();
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
          content: Text('Node kaydedildi'),
          backgroundColor: Color(0xFF26A69A),
        ));
      }
    } catch (e) {
      if (!mounted) return;
      setState(() { _submitting = false; _error = e.toString(); });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.fromLTRB(
          20, 20, 20, MediaQuery.of(context).viewInsets.bottom + 20),
      child: Column(mainAxisSize: MainAxisSize.min, children: [
        Container(width: 40, height: 4,
            margin: const EdgeInsets.only(bottom: 20),
            decoration: BoxDecoration(color: Colors.white24,
                borderRadius: BorderRadius.circular(2))),
        const Text('Yeni Node Ekle',
            style: TextStyle(color: Colors.white,
                fontSize: 18, fontWeight: FontWeight.bold)),
        const SizedBox(height: 20),

        TextField(
          controller: _addrCtrl,
          style: const TextStyle(color: Colors.white, fontFamily: 'monospace'),
          decoration: _inputDeco('Node adresi', 'host:port'),
        ),
        const SizedBox(height: 16),

        // Kapasite seçimi
        const Align(alignment: Alignment.centerLeft,
          child: Text('Kapasiteler:',
              style: TextStyle(color: Colors.white70, fontSize: 13))),
        const SizedBox(height: 8),
        Wrap(spacing: 8, children: ['wasm', 'agent', 'ai', 'workflow', 'plugin']
            .map((c) => FilterChip(
              label: Text(c),
              selected: _caps.contains(c),
              onSelected: (v) => setState(() =>
                  v ? _caps.add(c) : _caps.remove(c)),
              selectedColor: const Color(0xFF26A69A).withOpacity(0.3),
              checkmarkColor: const Color(0xFF26A69A),
              labelStyle: TextStyle(
                  color: _caps.contains(c)
                      ? const Color(0xFF26A69A)
                      : Colors.white54,
                  fontSize: 12),
              backgroundColor: const Color(0xFF252540),
              side: BorderSide(
                  color: _caps.contains(c)
                      ? const Color(0xFF26A69A)
                      : Colors.white24),
            )).toList()),

        if (_error != null) ...[
          const SizedBox(height: 12),
          Text(_error!,
              style: const TextStyle(color: Color(0xFFFF5252), fontSize: 12)),
        ],
        const SizedBox(height: 20),

        SizedBox(width: double.infinity,
          child: ElevatedButton.icon(
            onPressed: _submitting ? null : _submit,
            icon: _submitting
                ? const SizedBox(width: 16, height: 16,
                    child: CircularProgressIndicator(
                        strokeWidth: 2, color: Colors.white))
                : const Icon(Icons.add_link, color: Colors.white),
            label: Text(_submitting ? 'Kaydediliyor…' : 'Kaydet',
                style: const TextStyle(color: Colors.white, fontSize: 16)),
            style: ElevatedButton.styleFrom(
              backgroundColor: const Color(0xFF26A69A),
              padding: const EdgeInsets.symmetric(vertical: 14),
              shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12)),
            ),
          ),
        ),
      ]),
    );
  }
}

// ── Küçük yardımcılar ─────────────────────────────────────────

class _EmptyNodes extends StatelessWidget {
  final VoidCallback onAdd;
  const _EmptyNodes({required this.onAdd});
  @override
  Widget build(BuildContext context) => Center(child: Column(children: [
    const SizedBox(height: 32),
    const Icon(Icons.cloud_off, size: 56, color: Colors.white24),
    const SizedBox(height: 12),
    const Text('Kayıtlı node yok',
        style: TextStyle(color: Colors.white54, fontSize: 15)),
    const SizedBox(height: 6),
    const Text('Bu cihaz tek node olarak çalışıyor',
        style: TextStyle(color: Colors.white38, fontSize: 12)),
    const SizedBox(height: 20),
    ElevatedButton.icon(
      onPressed: onAdd,
      icon: const Icon(Icons.add, color: Colors.white),
      label: const Text('Node Ekle', style: TextStyle(color: Colors.white)),
      style: ElevatedButton.styleFrom(
        backgroundColor: const Color(0xFF26A69A),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    ),
  ]));
}

class _StatPill extends StatelessWidget {
  final String label;
  final String value;
  final Color color;
  const _StatPill({required this.label, required this.value, required this.color});
  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
    decoration: BoxDecoration(
      color: color.withOpacity(0.1),
      borderRadius: BorderRadius.circular(8),
      border: Border.all(color: color.withOpacity(0.3)),
    ),
    child: Column(children: [
      Text(value, style: TextStyle(
          color: color, fontSize: 16, fontWeight: FontWeight.bold)),
      Text(label, style: const TextStyle(
          color: Colors.white38, fontSize: 10)),
    ]),
  );
}

class _CapChip extends StatelessWidget {
  final String label;
  const _CapChip(this.label);
  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
    decoration: BoxDecoration(
      color: const Color(0xFF26A69A).withOpacity(0.15),
      borderRadius: BorderRadius.circular(6),
      border: Border.all(color: const Color(0xFF26A69A).withOpacity(0.4)),
    ),
    child: Text(label,
        style: const TextStyle(
            color: Color(0xFF26A69A), fontSize: 11,
            fontWeight: FontWeight.w600)),
  );
}

class _Metric extends StatelessWidget {
  final String label;
  final String value;
  final Color color;
  const _Metric({required this.label, required this.value, required this.color});
  @override
  Widget build(BuildContext context) => Row(mainAxisSize: MainAxisSize.min, children: [
    Text('$label: ', style: const TextStyle(color: Colors.white38, fontSize: 12)),
    Text(value, style: TextStyle(
        color: color, fontSize: 12, fontWeight: FontWeight.w600)),
  ]);
}

InputDecoration _inputDeco(String label, String hint) => InputDecoration(
  labelText: label,
  hintText: hint,
  labelStyle: const TextStyle(color: Colors.white54),
  hintStyle: const TextStyle(color: Colors.white24),
  enabledBorder: OutlineInputBorder(
    borderRadius: BorderRadius.circular(10),
    borderSide: const BorderSide(color: Colors.white24),
  ),
  focusedBorder: OutlineInputBorder(
    borderRadius: BorderRadius.circular(10),
    borderSide: const BorderSide(color: Color(0xFF26A69A)),
  ),
  filled: true,
  fillColor: const Color(0xFF252540),
);
