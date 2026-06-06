// ============================================================
// flutter_app/lib/screens/task_list_screen.dart
// ============================================================

import 'dart:async';
import 'package:flutter/material.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;

class TaskListScreen extends StatefulWidget {
  const TaskListScreen({super.key});
  @override
  State<TaskListScreen> createState() => _TaskListScreenState();
}

class _TaskListScreenState extends State<TaskListScreen> {
  List<rust.TaskStatusResponse> _tasks = [];
  bool _loading = true;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _load();
    _timer = Timer.periodic(const Duration(seconds: 3), (_) => _load());
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final tasks = await AetherApi.listTasks(limit: 50);
      if (mounted) setState(() { _tasks = tasks; _loading = false; });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: const Text('Task Listesi', style: TextStyle(color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh, color: Colors.white70),
            onPressed: _load,
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator(color: Color(0xFF6C63FF)))
          : _tasks.isEmpty
              ? const Center(
                  child: Text(
                    'Henüz task yok',
                    style: TextStyle(color: Colors.white38),
                  ),
                )
              : ListView.separated(
                  padding: const EdgeInsets.all(16),
                  itemCount: _tasks.length,
                  separatorBuilder: (_, __) => const SizedBox(height: 8),
                  itemBuilder: (ctx, i) => _TaskCard(task: _tasks[i]),
                ),
    );
  }
}

// ── Task kartı ────────────────────────────────────────────

class _TaskCard extends StatelessWidget {
  final rust.TaskStatusResponse task;
  const _TaskCard({required this.task});

  @override
  Widget build(BuildContext context) {
    final (color, icon) = _stateStyle(task.state);

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFF1A1A2E),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withOpacity(0.25)),
      ),
      child: Row(
        children: [
          Icon(icon, color: color, size: 20),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  task.taskId.substring(0, 8).toUpperCase(),
                  style: const TextStyle(
                    color: Colors.white,
                    fontFamily: 'monospace',
                    fontWeight: FontWeight.bold,
                    fontSize: 13,
                  ),
                ),
                const SizedBox(height: 3),
                Row(
                  children: [
                    _chip(task.state, color),
                    const SizedBox(width: 6),
                    Text(
                      'deneme: ${task.attempts}',
                      style: const TextStyle(color: Colors.white38, fontSize: 11),
                    ),
                  ],
                ),
                if (task.errorMessage != null) ...[
                  const SizedBox(height: 4),
                  Text(
                    task.errorMessage!,
                    style: const TextStyle(color: Color(0xFFEF5350), fontSize: 11),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ],
            ),
          ),
          Text(
            _timeAgo(task.updatedAt),
            style: const TextStyle(color: Colors.white24, fontSize: 10),
          ),
        ],
      ),
    );
  }

  Widget _chip(String label, Color color) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
    decoration: BoxDecoration(
      color: color.withOpacity(0.12),
      borderRadius: BorderRadius.circular(4),
    ),
    child: Text(
      label,
      style: TextStyle(color: color, fontSize: 10, fontWeight: FontWeight.bold),
    ),
  );

  (Color, IconData) _stateStyle(String state) => switch (state) {
    'Completed'  => (const Color(0xFF4CAF50), Icons.check_circle_outline_rounded),
    'Failed'     => (const Color(0xFFEF5350), Icons.cancel_outlined),
    'Executing'  => (const Color(0xFF6C63FF), Icons.play_circle_outline_rounded),
    'Queued'     => (const Color(0xFFFFB74D), Icons.schedule_rounded),
    'Retrying'   => (const Color(0xFF7E57C2), Icons.refresh_rounded),
    'Cancelled'  => (Colors.white38,          Icons.do_not_disturb_rounded),
    _            => (Colors.white38,          Icons.circle_outlined),
  };

  String _timeAgo(int ms) {
    final diff = DateTime.now().difference(
      DateTime.fromMillisecondsSinceEpoch(ms),
    );
    if (diff.inSeconds < 60)  return '${diff.inSeconds}s';
    if (diff.inMinutes < 60)  return '${diff.inMinutes}m';
    return '${diff.inHours}h';
  }
}
