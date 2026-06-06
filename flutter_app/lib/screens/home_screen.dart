// ============================================================
// flutter_app/lib/screens/home_screen.dart
//
// Ana ekran: metrik dashboard + hızlı task gönderme.
// ============================================================

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;
import 'task_list_screen.dart';
import 'submit_task_screen.dart';

// ── Provider: metrikler her 2s yenilenir ──────────────────

final metricsProvider = StreamProvider<rust.MetricsSnapshot>((ref) {
  return Stream.periodic(
    const Duration(seconds: 2),
    (_) => AetherApi.getMetrics(),
  ).asyncMap((f) => f);
});

// ── Ekran ─────────────────────────────────────────────────

class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final metrics = ref.watch(metricsProvider);

    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: const Row(
          children: [
            _AetherLogo(),
            SizedBox(width: 10),
            Text(
              'AetherOS',
              style: TextStyle(
                color: Color(0xFF6C63FF),
                fontWeight: FontWeight.bold,
                fontSize: 22,
                letterSpacing: 1.2,
              ),
            ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.list_alt, color: Colors.white70),
            tooltip: 'Task listesi',
            onPressed: () => Navigator.push(
              context,
              MaterialPageRoute(
                builder: (_) => const TaskListScreen(),
              ),
            ),
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // ── Runtime durumu ─────────────────────────
            const _RuntimeStatusCard(),
            const SizedBox(height: 16),

            // ── Metrik kartları ────────────────────────
            const Text(
              'Metrikler',
              style: TextStyle(
                color: Colors.white60,
                fontSize: 13,
                letterSpacing: 1.1,
              ),
            ),
            const SizedBox(height: 8),
            metrics.when(
              data: (snap) => _MetricsGrid(snapshot: snap),
              loading: () => const _MetricsGrid(snapshot: null),
              error: (e, _) => Text(
                'Metrik hatası: $e',
                style: const TextStyle(color: Colors.redAccent),
              ),
            ),

            const Spacer(),

            // ── Task gönder butonu ─────────────────────
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                style: FilledButton.styleFrom(
                  backgroundColor: const Color(0xFF6C63FF),
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                ),
                onPressed: () => Navigator.push(
                  context,
                  MaterialPageRoute(
                    builder: (_) => const SubmitTaskScreen(),
                  ),
                ),
                icon: const Icon(Icons.rocket_launch_rounded),
                label: const Text(
                  'Task Gönder',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
                ),
              ),
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}

// ── Runtime durum kartı ───────────────────────────────────

class _RuntimeStatusCard extends StatelessWidget {
  const _RuntimeStatusCard();

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<rust.RuntimeInfo>(
      future: AetherApi.getRuntimeInfo(),
      builder: (ctx, snap) {
        final info = snap.data;
        final isRunning = info?.isRunning ?? false;

        return Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: const Color(0xFF1A1A2E),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: isRunning
                  ? const Color(0xFF6C63FF).withOpacity(0.4)
                  : Colors.red.withOpacity(0.3),
            ),
          ),
          child: Row(
            children: [
              Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: isRunning ? const Color(0xFF4CAF50) : Colors.red,
                  boxShadow: [
                    BoxShadow(
                      color: (isRunning ? const Color(0xFF4CAF50) : Colors.red)
                          .withOpacity(0.6),
                      blurRadius: 6,
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      isRunning ? 'Runtime Aktif' : 'Runtime Kapalı',
                      style: const TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        fontSize: 14,
                      ),
                    ),
                    if (info != null)
                      Text(
                        'v${info.version} · ${info.backend} · ${info.workerCount} worker',
                        style: const TextStyle(
                          color: Colors.white38,
                          fontSize: 11,
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

// ── Metrik grid ───────────────────────────────────────────

class _MetricsGrid extends StatelessWidget {
  final rust.MetricsSnapshot? snapshot;
  const _MetricsGrid({required this.snapshot});

  @override
  Widget build(BuildContext context) {
    return GridView.count(
      crossAxisCount: 2,
      shrinkWrap: true,
      crossAxisSpacing: 10,
      mainAxisSpacing: 10,
      childAspectRatio: 2.0,
      physics: const NeverScrollableScrollPhysics(),
      children: [
        _MetricTile(
          label: 'Tamamlandı',
          value: snapshot?.completedTasks,
          color: const Color(0xFF4CAF50),
          icon: Icons.check_circle_outline_rounded,
        ),
        _MetricTile(
          label: 'Başarısız',
          value: snapshot?.failedTasks,
          color: const Color(0xFFEF5350),
          icon: Icons.error_outline_rounded,
        ),
        _MetricTile(
          label: 'Kuyrukta',
          value: snapshot?.queuedTasks,
          color: const Color(0xFFFFB74D),
          icon: Icons.pending_outlined,
        ),
        _MetricTile(
          label: 'Yeniden Deneme',
          value: snapshot?.retriedTasks,
          color: const Color(0xFF7E57C2),
          icon: Icons.refresh_rounded,
        ),
      ],
    );
  }
}

class _MetricTile extends StatelessWidget {
  final String label;
  final int? value;
  final Color color;
  final IconData icon;

  const _MetricTile({
    required this.label,
    required this.value,
    required this.color,
    required this.icon,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      decoration: BoxDecoration(
        color: color.withOpacity(0.08),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withOpacity(0.2)),
      ),
      child: Row(
        children: [
          Icon(icon, color: color, size: 22),
          const SizedBox(width: 10),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(
                value != null ? '$value' : '—',
                style: TextStyle(
                  color: color,
                  fontSize: 20,
                  fontWeight: FontWeight.bold,
                ),
              ),
              Text(
                label,
                style: const TextStyle(
                  color: Colors.white38,
                  fontSize: 11,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

// ── Logo ──────────────────────────────────────────────────

class _AetherLogo extends StatelessWidget {
  const _AetherLogo();

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 32,
      height: 32,
      decoration: BoxDecoration(
        gradient: const LinearGradient(
          colors: [Color(0xFF6C63FF), Color(0xFF3F51B5)],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        borderRadius: BorderRadius.circular(8),
      ),
      child: const Icon(
        Icons.bolt_rounded,
        color: Colors.white,
        size: 20,
      ),
    );
  }
}
