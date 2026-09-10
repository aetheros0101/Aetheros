// ============================================================
// flutter_app/lib/screens/task_list_screen.dart
// Sprint 2: filtreler, canlı dot, detay modal, retry, ikon fix
// ============================================================

import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;

enum _Filter { all, active, failed, completed }

extension _FilterLabel on _Filter {
  String get label => switch (this) {
        _Filter.all       => 'Tümü',
        _Filter.active    => 'Aktif',
        _Filter.failed    => 'Başarısız',
        _Filter.completed => 'Tamamlandı',
      };
}

class TaskListScreen extends StatefulWidget {
  const TaskListScreen({super.key});
  @override
  State<TaskListScreen> createState() => _TaskListScreenState();
}

class _TaskListScreenState extends State<TaskListScreen> {
  List<rust.TaskStatusResponse> _tasks = [];
  bool _loading = true;
  bool _refreshing = false;
  Timer? _timer;
  _Filter _filter = _Filter.all;

  @override
  void initState() {
    super.initState();
    _load();
    _timer = Timer.periodic(const Duration(seconds: 3), (_) => _load(silent: true));
  }

  @override
  void dispose() { _timer?.cancel(); super.dispose(); }

  Future<void> _load({bool silent = false}) async {
    if (!silent && mounted) setState(() => _refreshing = true);
    try {
      final tasks = await AetherApi.listTasks(limit: 100);
      if (mounted) setState(() { _tasks = tasks; _loading = false; _refreshing = false; });
    } catch (_) {
      if (mounted) setState(() { _loading = false; _refreshing = false; });
    }
  }

  List<rust.TaskStatusResponse> get _filtered => _tasks.where((t) => switch (_filter) {
    _Filter.all       => true,
    _Filter.active    => ['Created','Queued','Executing','Retrying'].contains(t.state),
    _Filter.failed    => t.state == 'Failed',
    _Filter.completed => t.state == 'Completed',
  }).toList();

  int _count(_Filter f) => _tasks.where((t) => switch (f) {
    _Filter.all       => true,
    _Filter.active    => ['Created','Queued','Executing','Retrying'].contains(t.state),
    _Filter.failed    => t.state == 'Failed',
    _Filter.completed => t.state == 'Completed',
  }).length;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: Row(children: [
          const Text('Task Listesi', style: TextStyle(color: Colors.white)),
          const SizedBox(width: 8),
          _LiveDot(active: !_loading),
        ]),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          _refreshing
              ? const Padding(
                  padding: EdgeInsets.all(14),
                  child: SizedBox(width: 18, height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2, color: Color(0xFF6C63FF))),
                )
              : IconButton(icon: const Icon(Icons.refresh, color: Colors.white70), onPressed: _load),
        ],
      ),
      body: Column(children: [
        _FilterBar(
          selected: _filter,
          counts: {for (var f in _Filter.values) f: _count(f)},
          onSelected: (f) => setState(() => _filter = f),
        ),
        Expanded(
          child: _loading
              ? const Center(child: CircularProgressIndicator(color: Color(0xFF6C63FF)))
              : _filtered.isEmpty
                  ? _EmptyState(filter: _filter)
                  : RefreshIndicator(
                      color: const Color(0xFF6C63FF),
                      backgroundColor: const Color(0xFF1A1A2E),
                      onRefresh: _load,
                      child: ListView.separated(
                        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
                        itemCount: _filtered.length,
                        separatorBuilder: (_, __) => const SizedBox(height: 8),
                        itemBuilder: (ctx, i) => _TaskCard(
                          task: _filtered[i],
                          onTap: () => _showDetail(_filtered[i]),
                          onRetry: () => _retry(_filtered[i]),
                        ),
                      ),
                    ),
        ),
      ]),
    );
  }

  void _showDetail(rust.TaskStatusResponse task) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (_) => _TaskDetailSheet(
        task: task,
        onRetry: () { Navigator.pop(context); _retry(task); },
      ),
    );
  }

  Future<void> _retry(rust.TaskStatusResponse task) async {
    try {
      final newTaskId = await AetherApi.resubmitTask(task.taskId);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text('Task yeniden kuyruğa alındı: ${newTaskId.substring(0, 8)}…'),
        backgroundColor: const Color(0xFF6C63FF),
        duration: const Duration(seconds: 2),
      ));
      await _load();
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text('Yeniden deneme başarısız: $e'),
        backgroundColor: const Color(0xFFEF5350),
        duration: const Duration(seconds: 3),
      ));
    }
  }
}

// ── Filter bar ────────────────────────────────────────────

class _FilterBar extends StatelessWidget {
  final _Filter selected;
  final Map<_Filter, int> counts;
  final ValueChanged<_Filter> onSelected;
  const _FilterBar({required this.selected, required this.counts, required this.onSelected});

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 44,
      color: const Color(0xFF0F0F1A),
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
        children: _Filter.values.map((f) {
          final sel = f == selected;
          final cnt = counts[f] ?? 0;
          return GestureDetector(
            onTap: () => onSelected(f),
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 180),
              margin: const EdgeInsets.only(right: 8),
              padding: const EdgeInsets.symmetric(horizontal: 12),
              decoration: BoxDecoration(
                color: sel ? const Color(0xFF6C63FF) : const Color(0xFF1A1A2E),
                borderRadius: BorderRadius.circular(20),
                border: Border.all(color: sel ? const Color(0xFF6C63FF) : Colors.white12),
              ),
              child: Row(children: [
                Text(f.label,
                    style: TextStyle(
                        color: sel ? Colors.white : Colors.white54,
                        fontSize: 12,
                        fontWeight: sel ? FontWeight.bold : FontWeight.normal)),
                if (cnt > 0) ...[
                  const SizedBox(width: 5),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                    decoration: BoxDecoration(
                      color: sel ? Colors.white24 : const Color(0xFF6C63FF).withOpacity(0.3),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text('$cnt',
                        style: TextStyle(
                            color: sel ? Colors.white : Colors.white70,
                            fontSize: 10, fontWeight: FontWeight.bold)),
                  ),
                ],
              ]),
            ),
          );
        }).toList(),
      ),
    );
  }
}

// ── Canlı nokta animasyonu ────────────────────────────────

class _LiveDot extends StatefulWidget {
  final bool active;
  const _LiveDot({required this.active});
  @override
  State<_LiveDot> createState() => _LiveDotState();
}

class _LiveDotState extends State<_LiveDot> with SingleTickerProviderStateMixin {
  late final AnimationController _ctrl;
  late final Animation<double> _anim;

  @override
  void initState() {
    super.initState();
    _ctrl = AnimationController(duration: const Duration(milliseconds: 900), vsync: this)
      ..repeat(reverse: true);
    _anim = Tween(begin: 0.2, end: 1.0).animate(_ctrl);
  }

  @override
  void dispose() { _ctrl.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) {
    if (!widget.active) return const SizedBox.shrink();
    return FadeTransition(
      opacity: _anim,
      child: Container(
        width: 6, height: 6,
        decoration: const BoxDecoration(shape: BoxShape.circle, color: Color(0xFF4CAF50)),
      ),
    );
  }
}

// ── Boş durum ─────────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  final _Filter filter;
  const _EmptyState({required this.filter});
  @override
  Widget build(BuildContext context) => Center(
    child: Column(mainAxisSize: MainAxisSize.min, children: [
      const Icon(Icons.inbox, color: Colors.white12, size: 48),
      const SizedBox(height: 12),
      Text(
        filter == _Filter.all ? 'Henüz task gönderilmedi' : '${filter.label} task yok',
        style: const TextStyle(color: Colors.white38, fontSize: 14),
      ),
    ]),
  );
}

// ── Task kartı ────────────────────────────────────────────

class _TaskCard extends StatelessWidget {
  final rust.TaskStatusResponse task;
  final VoidCallback onTap;
  final VoidCallback onRetry;
  const _TaskCard({required this.task, required this.onTap, required this.onRetry});

  @override
  Widget build(BuildContext context) {
    final s = _style(task.state);
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: const Color(0xFF1A1A2E),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: s.color.withOpacity(0.22)),
        ),
        child: Row(children: [
          Container(
            width: 36, height: 36,
            decoration: BoxDecoration(
              color: s.color.withOpacity(0.1),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(s.icon, color: s.color, size: 18),
          ),
          const SizedBox(width: 12),
          Expanded(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Row(children: [
              Text(
                task.taskId.substring(0, 8).toUpperCase(),
                style: const TextStyle(
                  color: Colors.white, fontFamily: 'monospace',
                  fontWeight: FontWeight.bold, fontSize: 13, letterSpacing: 0.5),
              ),
              const SizedBox(width: 8),
              _Badge(state: task.state, color: s.color),
            ]),
            const SizedBox(height: 4),
            Row(children: [
              Text('deneme: ${task.attempts}',
                  style: const TextStyle(color: Colors.white38, fontSize: 11)),
              const SizedBox(width: 8),
              Text(_ago(task.updatedAt),
                  style: const TextStyle(color: Colors.white24, fontSize: 11)),
            ]),
            if (task.errorMessage != null) ...[
              const SizedBox(height: 3),
              Text(task.errorMessage!,
                  style: const TextStyle(color: Color(0xFFEF5350), fontSize: 11),
                  maxLines: 1, overflow: TextOverflow.ellipsis),
            ],
          ])),
          if (task.state == 'Failed')
            GestureDetector(
              onTap: onRetry,
              child: Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: const Color(0xFF6C63FF).withOpacity(0.12),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: const Icon(Icons.refresh, color: Color(0xFF6C63FF), size: 18),
              ),
            ),
        ]),
      ),
    );
  }

  _S _style(String state) => switch (state) {
    'Completed' => _S(const Color(0xFF4CAF50), Icons.check_circle_outline),
    'Failed'    => _S(const Color(0xFFEF5350), Icons.cancel_outlined),
    'Executing' => _S(const Color(0xFF6C63FF), Icons.play_circle_outline),
    'Queued'    => _S(const Color(0xFFFFB74D), Icons.schedule),
    'Retrying'  => _S(const Color(0xFF7E57C2), Icons.refresh),
    'Cancelled' => _S(Colors.white38,           Icons.do_not_disturb),
    'Created'   => _S(const Color(0xFF42A5F5), Icons.fiber_new),
    _           => _S(Colors.white38,           Icons.radio_button_unchecked),
  };

  String _ago(int ms) {
    final d = DateTime.now().difference(DateTime.fromMillisecondsSinceEpoch(ms));
    if (d.inSeconds < 60) return '${d.inSeconds}s';
    if (d.inMinutes < 60) return '${d.inMinutes}m';
    return '${d.inHours}h';
  }
}

class _S { final Color color; final IconData icon; const _S(this.color, this.icon); }

class _Badge extends StatelessWidget {
  final String state; final Color color;
  const _Badge({required this.state, required this.color});
  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
    decoration: BoxDecoration(color: color.withOpacity(0.12), borderRadius: BorderRadius.circular(4)),
    child: Text(state,
        style: TextStyle(color: color, fontSize: 10, fontWeight: FontWeight.bold, letterSpacing: 0.3)),
  );
}

// ── Detay modal ───────────────────────────────────────────

class _TaskDetailSheet extends StatelessWidget {
  final rust.TaskStatusResponse task;
  final VoidCallback onRetry;
  const _TaskDetailSheet({required this.task, required this.onRetry});

  @override
  Widget build(BuildContext context) {
    final created = DateTime.fromMillisecondsSinceEpoch(task.createdAt).toLocal();
    final updated = DateTime.fromMillisecondsSinceEpoch(task.updatedAt).toLocal();

    return DraggableScrollableSheet(
      initialChildSize: 0.55, minChildSize: 0.35, maxChildSize: 0.85,
      expand: false,
      builder: (_, ctrl) => Container(
        decoration: const BoxDecoration(
          color: Color(0xFF1A1A2E),
          borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
        ),
        child: ListView(controller: ctrl, padding: const EdgeInsets.all(20), children: [
          Center(child: Container(
            width: 40, height: 4,
            decoration: BoxDecoration(color: Colors.white24, borderRadius: BorderRadius.circular(2)),
          )),
          const SizedBox(height: 20),
          Row(mainAxisAlignment: MainAxisAlignment.spaceBetween, children: [
            const Text('Task Detayı',
                style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold)),
            IconButton(
              icon: const Icon(Icons.close, color: Colors.white54, size: 20),
              onPressed: () => Navigator.pop(context),
            ),
          ]),
          const SizedBox(height: 16),
          _Row(label: 'Task ID', value: task.taskId, mono: true,
              trailing: IconButton(
                icon: const Icon(Icons.copy, color: Color(0xFF6C63FF), size: 16),
                onPressed: () {
                  Clipboard.setData(ClipboardData(text: task.taskId));
                  ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
                    content: Text('UUID kopyalandı'),
                    backgroundColor: Color(0xFF6C63FF),
                    duration: Duration(seconds: 1),
                  ));
                },
              )),
          const SizedBox(height: 8),
          _Row(label: 'Durum',       value: task.state),
          const SizedBox(height: 8),
          _Row(label: 'Deneme',      value: '${task.attempts}'),
          const SizedBox(height: 8),
          _Row(label: 'Oluşturuldu', value: _fmt(created)),
          const SizedBox(height: 8),
          _Row(label: 'Güncellendi', value: _fmt(updated)),
          if (task.errorMessage != null) ...[
            const SizedBox(height: 8),
            _Row(label: 'Hata', value: task.errorMessage!, error: true),
          ],
          const SizedBox(height: 24),
          if (task.state == 'Failed')
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                style: FilledButton.styleFrom(
                  backgroundColor: const Color(0xFF6C63FF),
                  padding: const EdgeInsets.symmetric(vertical: 14),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                ),
                onPressed: onRetry,
                icon: const Icon(Icons.refresh, size: 18),
                label: const Text('Yeniden Dene', style: TextStyle(fontSize: 15)),
              ),
            ),
        ]),
      ),
    );
  }

  String _fmt(DateTime d) =>
      '${d.year}-${_p(d.month)}-${_p(d.day)}  ${_p(d.hour)}:${_p(d.minute)}:${_p(d.second)}';
  String _p(int n) => n.toString().padLeft(2, '0');
}

class _Row extends StatelessWidget {
  final String label, value;
  final bool mono, error;
  final Widget? trailing;
  const _Row({required this.label, required this.value,
      this.mono = false, this.error = false, this.trailing});
  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
    decoration: BoxDecoration(
      color: error ? const Color(0xFFEF5350).withOpacity(0.07) : Colors.white.withOpacity(0.04),
      borderRadius: BorderRadius.circular(8),
      border: Border.all(color: error ? const Color(0xFFEF5350).withOpacity(0.2) : Colors.white12),
    ),
    child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
      SizedBox(width: 90, child: Text(label,
          style: const TextStyle(color: Colors.white38, fontSize: 11, letterSpacing: 0.5))),
      Expanded(child: Text(value,
          style: TextStyle(
            color: error ? const Color(0xFFEF5350) : Colors.white70,
            fontSize: 12, fontFamily: mono ? 'monospace' : null))),
      if (trailing != null) trailing!,
    ]),
  );
}
