// ============================================================
// flutter_app/lib/screens/agent_screen.dart
//
// Faz-1 UI — Agent yönetimi
//   • Yeni agent başlat (objective + ayarlar)
//   • Aktif/tamamlanmış execution'ları listele
//   • Execution detayı (durum, süre, hata)
// ============================================================

import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;

class AgentScreen extends StatefulWidget {
  const AgentScreen({super.key});
  @override
  State<AgentScreen> createState() => _AgentScreenState();
}

class _AgentScreenState extends State<AgentScreen> {
  List<rust.AgentStatusResponse> _agents = [];
  bool   _loading   = true;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _load();
    _timer = Timer.periodic(const Duration(seconds: 3), (_) => _load(silent: true));
  }

  @override
  void dispose() { _timer?.cancel(); super.dispose(); }

  Future<void> _load({bool silent = false}) async {
    if (!silent && mounted) setState(() => _loading = true);
    try {
      final agents = await AetherApi.listAgents(limit: 50);
      if (mounted) setState(() { _agents = agents; _loading = false; });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _showNewAgentDialog() {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: const Color(0xFF1A1A2E),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => _NewAgentSheet(onCreated: _load),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0D0D1A),
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: const Text('Agent Yönetimi',
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
        onPressed: _showNewAgentDialog,
        backgroundColor: const Color(0xFF6C63FF),
        icon: const Icon(Icons.smart_toy, color: Colors.white),
        label: const Text('Yeni Agent', style: TextStyle(color: Colors.white)),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator(color: Color(0xFF6C63FF)))
          : _agents.isEmpty
              ? _EmptyState(onNew: _showNewAgentDialog)
              : ListView.builder(
                  padding: const EdgeInsets.fromLTRB(12, 8, 12, 100),
                  itemCount: _agents.length,
                  itemBuilder: (_, i) => _AgentCard(
                    agent: _agents[i],
                    onTap: () => _showDetail(_agents[i]),
                  ),
                ),
    );
  }

  void _showDetail(rust.AgentStatusResponse agent) {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1A1A2E),
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => _AgentDetailSheet(agent: agent),
    );
  }
}

// ── Yeni Agent formu ──────────────────────────────────────────

class _NewAgentSheet extends StatefulWidget {
  final VoidCallback onCreated;
  const _NewAgentSheet({required this.onCreated});
  @override State<_NewAgentSheet> createState() => _NewAgentSheetState();
}

class _NewAgentSheetState extends State<_NewAgentSheet> {
  final _objCtrl  = TextEditingController();
  int    _maxSteps   = 10;
  int    _maxTokens  = 4096;
  bool   _submitting = false;
  String? _error;

  @override
  void dispose() { _objCtrl.dispose(); super.dispose(); }

  Future<void> _submit() async {
    if (_objCtrl.text.trim().isEmpty) {
      setState(() => _error = 'Hedef boş olamaz.');
      return;
    }
    setState(() { _submitting = true; _error = null; });
    try {
      await AetherApi.startAgent(
        objective: _objCtrl.text.trim(),
        maxSteps:  _maxSteps,
        maxTokens: _maxTokens,
      );
      if (mounted) {
        Navigator.pop(context);
        widget.onCreated();
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
          content: Text('Agent başlatıldı'),
          backgroundColor: Color(0xFF6C63FF),
        ));
      }
    } catch (e) {
      setState(() { _submitting = false; _error = e.toString(); });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.fromLTRB(
          20, 20, 20, MediaQuery.of(context).viewInsets.bottom + 20),
      child: Column(mainAxisSize: MainAxisSize.min, children: [
        // Handle
        Container(width: 40, height: 4,
            margin: const EdgeInsets.only(bottom: 20),
            decoration: BoxDecoration(
                color: Colors.white24,
                borderRadius: BorderRadius.circular(2))),
        const Text('Yeni Agent Başlat',
            style: TextStyle(color: Colors.white,
                fontSize: 18, fontWeight: FontWeight.bold)),
        const SizedBox(height: 20),

        // Objective
        TextField(
          controller: _objCtrl,
          style: const TextStyle(color: Colors.white),
          maxLines: 3,
          decoration: _inputDeco('Hedef / Görev tanımı',
              'Örn: Proje dosyalarını analiz et ve özet çıkar'),
        ),
        const SizedBox(height: 16),

        // Max Steps
        Row(children: [
          const Text('Maks. adım:', style: TextStyle(color: Colors.white70)),
          const Spacer(),
          _StepChip(label: '5',  value: 5,  selected: _maxSteps == 5,
              onTap: () => setState(() => _maxSteps = 5)),
          _StepChip(label: '10', value: 10, selected: _maxSteps == 10,
              onTap: () => setState(() => _maxSteps = 10)),
          _StepChip(label: '20', value: 20, selected: _maxSteps == 20,
              onTap: () => setState(() => _maxSteps = 20)),
          _StepChip(label: '50', value: 50, selected: _maxSteps == 50,
              onTap: () => setState(() => _maxSteps = 50)),
        ]),
        const SizedBox(height: 12),

        // Max Tokens
        Row(children: [
          const Text('Maks. token:', style: TextStyle(color: Colors.white70)),
          const Spacer(),
          _StepChip(label: '1K',  value: 1024,  selected: _maxTokens == 1024,
              onTap: () => setState(() => _maxTokens = 1024)),
          _StepChip(label: '4K',  value: 4096,  selected: _maxTokens == 4096,
              onTap: () => setState(() => _maxTokens = 4096)),
          _StepChip(label: '8K',  value: 8192,  selected: _maxTokens == 8192,
              onTap: () => setState(() => _maxTokens = 8192)),
          _StepChip(label: '32K', value: 32768, selected: _maxTokens == 32768,
              onTap: () => setState(() => _maxTokens = 32768)),
        ]),

        if (_error != null) ...[
          const SizedBox(height: 12),
          Text(_error!, style: const TextStyle(color: Color(0xFFFF5252), fontSize: 12)),
        ],
        const SizedBox(height: 20),

        SizedBox(width: double.infinity,
          child: ElevatedButton.icon(
            onPressed: _submitting ? null : _submit,
            icon: _submitting
                ? const SizedBox(width: 16, height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                : const Icon(Icons.play_arrow, color: Colors.white),
            label: Text(_submitting ? 'Başlatılıyor…' : 'Başlat',
                style: const TextStyle(color: Colors.white, fontSize: 16)),
            style: ElevatedButton.styleFrom(
              backgroundColor: const Color(0xFF6C63FF),
              padding: const EdgeInsets.symmetric(vertical: 14),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
            ),
          ),
        ),
      ]),
    );
  }
}

// ── Agent kartı ───────────────────────────────────────────────

class _AgentCard extends StatelessWidget {
  final rust.AgentStatusResponse agent;
  final VoidCallback onTap;
  const _AgentCard({required this.agent, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final status  = agent.status;
    final color   = _statusColor(status);
    final icon    = _statusIcon(status);
    final elapsed = _elapsed(agent.startedAt, agent.finishedAt);

    return GestureDetector(
      onTap: onTap,
      child: Container(
        margin: const EdgeInsets.only(bottom: 10),
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: const Color(0xFF1A1A2E),
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: color.withOpacity(0.3)),
        ),
        child: Row(children: [
          Container(
            width: 40, height: 40,
            decoration: BoxDecoration(
              color: color.withOpacity(0.15),
              shape: BoxShape.circle,
            ),
            child: Icon(icon, color: color, size: 20),
          ),
          const SizedBox(width: 12),
          Expanded(child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                agent.executionId.substring(0, 8).toUpperCase(),
                style: const TextStyle(color: Colors.white,
                    fontWeight: FontWeight.bold, fontFamily: 'monospace'),
              ),
              const SizedBox(height: 2),
              Text(
                agent.objective.length > 60
                    ? '${agent.objective.substring(0, 60)}…'
                    : agent.objective,
                style: const TextStyle(color: Colors.white54, fontSize: 12),
              ),
            ],
          )),
          const SizedBox(width: 8),
          Column(crossAxisAlignment: CrossAxisAlignment.end, children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
              decoration: BoxDecoration(
                color: color.withOpacity(0.15),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(status,
                  style: TextStyle(color: color, fontSize: 11,
                      fontWeight: FontWeight.w600)),
            ),
            const SizedBox(height: 4),
            Text(elapsed, style: const TextStyle(color: Colors.white38, fontSize: 11)),
          ]),
        ]),
      ),
    );
  }

  Color _statusColor(String s) => switch (s) {
    'completed' => const Color(0xFF4CAF50),
    'failed'    => const Color(0xFFFF5252),
    'running'   => const Color(0xFF6C63FF),
    _           => Colors.white38,
  };

  IconData _statusIcon(String s) => switch (s) {
    'completed' => Icons.check_circle,
    'failed'    => Icons.error,
    'running'   => Icons.smart_toy,
    _           => Icons.hourglass_empty,
  };

  String _elapsed(int startMs, int? endMs) {
    final end   = endMs ?? DateTime.now().millisecondsSinceEpoch;
    final diff  = Duration(milliseconds: end - startMs);
    if (diff.inSeconds < 60) return '${diff.inSeconds}s';
    if (diff.inMinutes < 60) return '${diff.inMinutes}dk';
    return '${diff.inHours}s';
  }
}

// ── Agent detay modal ─────────────────────────────────────────

class _AgentDetailSheet extends StatelessWidget {
  final rust.AgentStatusResponse agent;
  const _AgentDetailSheet({required this.agent});

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.6,
      maxChildSize: 0.9,
      builder: (_, ctrl) => Container(
        padding: const EdgeInsets.all(20),
        child: ListView(controller: ctrl, children: [
          // Handle
          Center(child: Container(width: 40, height: 4,
              margin: const EdgeInsets.only(bottom: 16),
              decoration: BoxDecoration(color: Colors.white24,
                  borderRadius: BorderRadius.circular(2)))),
          const Text('Agent Detayı',
              style: TextStyle(color: Colors.white,
                  fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 16),

          _DetailRow('Execution ID', agent.executionId),
          _DetailRow('Agent ID',     agent.agentId),
          _DetailRow('Durum',        agent.status),
          _DetailRow('Hedef',        agent.objective),
          if (agent.error != null)
            _DetailRow('Hata', agent.error!, valueColor: const Color(0xFFFF5252)),
          const SizedBox(height: 16),

          // Kopyala butonu
          SizedBox(width: double.infinity,
            child: OutlinedButton.icon(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: agent.executionId));
                ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
                  content: Text('Execution ID kopyalandı'),
                  duration: Duration(seconds: 1),
                ));
              },
              icon: const Icon(Icons.copy, size: 16, color: Color(0xFF6C63FF)),
              label: const Text('Execution ID Kopyala',
                  style: TextStyle(color: Color(0xFF6C63FF))),
              style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: Color(0xFF6C63FF))),
            ),
          ),
        ]),
      ),
    );
  }
}

// ── Yardımcı widget'lar ───────────────────────────────────────

class _EmptyState extends StatelessWidget {
  final VoidCallback onNew;
  const _EmptyState({required this.onNew});
  @override
  Widget build(BuildContext context) => Center(child: Column(
    mainAxisAlignment: MainAxisAlignment.center,
    children: [
      const Icon(Icons.smart_toy_outlined, size: 64, color: Colors.white24),
      const SizedBox(height: 16),
      const Text('Henüz agent yok',
          style: TextStyle(color: Colors.white54, fontSize: 16)),
      const SizedBox(height: 8),
      const Text('Bir hedef belirleyerek agent başlat',
          style: TextStyle(color: Colors.white38, fontSize: 13)),
      const SizedBox(height: 24),
      ElevatedButton.icon(
        onPressed: onNew,
        icon: const Icon(Icons.add, color: Colors.white),
        label: const Text('Yeni Agent', style: TextStyle(color: Colors.white)),
        style: ElevatedButton.styleFrom(
          backgroundColor: const Color(0xFF6C63FF),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        ),
      ),
    ],
  ));
}

class _StepChip extends StatelessWidget {
  final String label;
  final int value;
  final bool selected;
  final VoidCallback onTap;
  const _StepChip({required this.label, required this.value,
      required this.selected, required this.onTap});
  @override
  Widget build(BuildContext context) => GestureDetector(
    onTap: onTap,
    child: Container(
      margin: const EdgeInsets.only(left: 6),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: selected ? const Color(0xFF6C63FF) : const Color(0xFF252540),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(label,
          style: TextStyle(
              color: selected ? Colors.white : Colors.white54,
              fontSize: 12, fontWeight: FontWeight.w600)),
    ),
  );
}

class _DetailRow extends StatelessWidget {
  final String label;
  final String value;
  final Color? valueColor;
  const _DetailRow(this.label, this.value, {this.valueColor});
  @override
  Widget build(BuildContext context) => Container(
    margin: const EdgeInsets.only(bottom: 10),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: const Color(0xFF252540),
      borderRadius: BorderRadius.circular(10),
    ),
    child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
      SizedBox(width: 110,
          child: Text(label,
              style: const TextStyle(color: Colors.white54, fontSize: 13))),
      Expanded(child: Text(value,
          style: TextStyle(
              color: valueColor ?? Colors.white,
              fontSize: 13, fontWeight: FontWeight.w500))),
    ]),
  );
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
    borderSide: const BorderSide(color: Color(0xFF6C63FF)),
  ),
  filled: true,
  fillColor: const Color(0xFF252540),
);
