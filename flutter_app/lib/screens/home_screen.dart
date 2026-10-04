// ============================================================
// flutter_app/lib/screens/home_screen.dart
// Faz-1 UI Tamamlama — Agent, Workflow, Log, Remote, Settings eklendi
// ============================================================

import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;
import 'submit_task_screen.dart';
import 'task_list_screen.dart';
import 'wasm_module_screen.dart';
import 'script_editor_screen.dart';
import 'ai_chat_screen.dart';
import 'backup_screen.dart';
import 'log_screen.dart';
import 'agent_screen.dart';
import 'approvals_screen.dart';
import 'workflow_screen.dart';
import 'remote_screen.dart';
import 'settings_screen.dart';
import 'audit_screen.dart';
import '../core/app_theme.dart';

// ── Provider ──────────────────────────────────────────────────

final metricsProvider = StreamProvider<rust.MetricsSnapshot>((ref) {
  return Stream.periodic(const Duration(seconds: 2), (_) => AetherApi.getMetrics())
      .asyncMap((f) => f);
});

/// B5: onay bekleyen çağrı sayısı (rozet için). Hata olursa 0 — rozet
/// asla ana ekranı bozmasın.
final pendingApprovalsProvider = StreamProvider.autoDispose<int>((ref) async* {
  while (true) {
    try {
      yield (await AetherApi.listPendingApprovals()).length;
    } catch (_) {
      yield 0;
    }
    await Future<void>.delayed(const Duration(seconds: 3));
  }
});

// ── Ekran ─────────────────────────────────────────────────────

class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final metrics = ref.watch(metricsProvider);
    final pending = ref.watch(pendingApprovalsProvider).valueOrNull ?? 0;

    return Scaffold(
      backgroundColor: AetherColors.background,
      appBar: AppBar(
        backgroundColor: AetherColors.background,
        title: const Row(children: [
          _AetherLogo(),
          SizedBox(width: 10),
          Text('AetherOS',
              style: TextStyle(
                  color: AetherColors.primary,
                  fontWeight: FontWeight.bold,
                  fontSize: 22,
                  letterSpacing: 1.2)),
        ]),
        actions: [
          IconButton(
            icon: Badge(
              isLabelVisible: pending > 0,
              label: Text('$pending'),
              child: Icon(
                Icons.gpp_maybe_outlined,
                color: pending > 0 ? AetherColors.warning : Colors.white70,
              ),
            ),
            tooltip: 'Bekleyen onaylar',
            onPressed: () => Navigator.push(context,
                MaterialPageRoute(builder: (_) => const ApprovalsScreen())),
          ),
          IconButton(
            icon: const Icon(Icons.history, color: Colors.white70),
            tooltip: 'Denetim kayıtları',
            onPressed: () => Navigator.push(context,
                MaterialPageRoute(builder: (_) => const AuditScreen())),
          ),
          IconButton(
            icon: const Icon(Icons.settings, color: Colors.white70),
            tooltip: 'Ayarlar',
            onPressed: () => Navigator.push(context,
                MaterialPageRoute(builder: (_) => const SettingsScreen())),
          ),
          IconButton(
            icon: const Icon(Icons.list_alt, color: Colors.white70),
            tooltip: 'Task listesi',
            onPressed: () => Navigator.push(context,
                MaterialPageRoute(builder: (_) => const TaskListScreen())),
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // ── Runtime durumu ──────────────────────────
            const _RuntimeStatusCard(),
            const SizedBox(height: 16),

            // ── Metrikler ────────────────────────────────
            const Text('Metrikler',
                style: TextStyle(
                    color: Colors.white60,
                    fontSize: 13,
                    letterSpacing: 1.1)),
            const SizedBox(height: 8),
            metrics.when(
              data:    (snap) => _MetricsGrid(snapshot: snap),
              loading: ()     => const _MetricsGrid(snapshot: null),
              error:   (e, _) => Text('Metrik hatası: $e',
                  style: const TextStyle(color: Colors.redAccent)),
            ),
            const SizedBox(height: 20),

            // ── Hızlı eylemler ───────────────────────────
            const Text('Hızlı Eylemler',
                style: TextStyle(
                    color: Colors.white60,
                    fontSize: 13,
                    letterSpacing: 1.1)),
            const SizedBox(height: 8),

            Expanded(
              child: _QuickActions(
                onWasm:     () => _go(context, const WasmModuleScreen()),
                onScript:   () => _go(context, const ScriptEditorScreen()),
                onTasks:    () => _go(context, const TaskListScreen()),
                onAi:       () => _go(context, const AiChatScreen()),
                onBackup:   () => _go(context, const BackupScreen()),
                onAgent:    () => _go(context, const AgentScreen()),
                onWorkflow: () => _go(context, const WorkflowScreen()),
                onLogs:     () => _go(context, const LogScreen()),
                onRemote:   () => _go(context, const RemoteScreen()),
              ),
            ),

            // ── Task gönder butonu ────────────────────────
            const SizedBox(height: 12),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                style: FilledButton.styleFrom(
                  backgroundColor: AetherColors.primary,
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12)),
                ),
                onPressed: () => _go(context, const SubmitTaskScreen()),
                icon: const Icon(Icons.rocket_launch),
                label: const Text('Task Gönder',
                    style: TextStyle(
                        fontSize: 16, fontWeight: FontWeight.bold)),
              ),
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }

  void _go(BuildContext ctx, Widget screen) =>
      Navigator.push(ctx, MaterialPageRoute(builder: (_) => screen));
}

// ── Hızlı Eylemler ────────────────────────────────────────────

class _QuickActions extends StatelessWidget {
  final VoidCallback onWasm;
  final VoidCallback onScript;
  final VoidCallback onTasks;
  final VoidCallback onAi;
  final VoidCallback onBackup;
  final VoidCallback onAgent;
  final VoidCallback onWorkflow;
  final VoidCallback onLogs;
  final VoidCallback onRemote;

  const _QuickActions({
    required this.onWasm,
    required this.onScript,
    required this.onTasks,
    required this.onAi,
    required this.onBackup,
    required this.onAgent,
    required this.onWorkflow,
    required this.onLogs,
    required this.onRemote,
  });

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    child: Column(children: [
      // Satır 1: WASM + Script
      Row(children: [
        _Tile(icon: Icons.memory,     label: 'WASM\nModüller',
            color: const Color(0xFF4DB6AC), onTap: onWasm),
        const SizedBox(width: 10),
        _Tile(icon: Icons.code,       label: 'Script\nEditör',
            color: AetherColors.warning, onTap: onScript),
      ]),
      const SizedBox(height: 10),

      // Satır 2: Task + AI
      Row(children: [
        _Tile(icon: Icons.format_list_bulleted, label: 'Task\nListesi',
            color: const Color(0xFF7E57C2), onTap: onTasks),
        const SizedBox(width: 10),
        _Tile(icon: Icons.auto_awesome,         label: 'AI\nAsistan',
            color: AetherColors.primary, onTap: onAi),
      ]),
      const SizedBox(height: 10),

      // Satır 3: Agent + Workflow
      Row(children: [
        _Tile(icon: Icons.smart_toy,    label: 'Agent\nYönetimi',
            color: const Color(0xFF26A69A), onTap: onAgent),
        const SizedBox(width: 10),
        _Tile(icon: Icons.account_tree, label: 'Workflow\nYönetimi',
            color: const Color(0xFFE8A838), onTap: onWorkflow),
      ]),
      const SizedBox(height: 10),

      // Satır 4: Log + Remote
      Row(children: [
        _Tile(icon: Icons.terminal,     label: 'Log\nİzle',
            color: const Color(0xFF78909C), onTap: onLogs),
        const SizedBox(width: 10),
        _Tile(icon: Icons.hub,          label: 'Remote\nCluster',
            color: AetherColors.info, onTap: onRemote),
      ]),
      const SizedBox(height: 10),

      // Satır 5: Yedekleme — tam genişlik
      InkWell(
        onTap: onBackup,
        child: Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(vertical: 12),
          decoration: BoxDecoration(
            color: AetherColors.info.withOpacity(0.08),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
                color: AetherColors.info.withOpacity(0.25)),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(Icons.shield_outlined,
                  color: AetherColors.info.withOpacity(0.85),
                  size: 20),
              const SizedBox(width: 8),
              Text('Yedekleme / Geri Yükleme',
                  style: TextStyle(
                      color: AetherColors.info.withOpacity(0.85),
                      fontSize: 12,
                      fontWeight: FontWeight.bold)),
            ],
          ),
        ),
      ),
    ]),
  );
}

class _Tile extends StatelessWidget {
  final IconData icon;
  final String label;
  final Color color;
  final VoidCallback onTap;

  const _Tile({
    required this.icon,
    required this.label,
    required this.color,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) => Expanded(
    child: InkWell(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 14),
        decoration: BoxDecoration(
          color: color.withOpacity(0.08),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: color.withOpacity(0.25)),
        ),
        child: Column(children: [
          Icon(icon, color: color, size: 24),
          const SizedBox(height: 6),
          Text(label,
              textAlign: TextAlign.center,
              style: TextStyle(
                  color: color.withOpacity(0.8),
                  fontSize: 11,
                  fontWeight: FontWeight.bold,
                  height: 1.3)),
        ]),
      ),
    ),
  );
}

// ── Runtime kartı ──────────────────────────────────────────────

class _RuntimeStatusCard extends StatelessWidget {
  const _RuntimeStatusCard();

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<rust.RuntimeInfo>(
      future: AetherApi.getRuntimeInfo(),
      builder: (ctx, snap) {
        final info      = snap.data;
        final isRunning = info?.isRunning ?? false;

        return Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: AetherColors.surface,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: isRunning
                  ? AetherColors.primary.withOpacity(0.4)
                  : Colors.red.withOpacity(0.3),
            ),
          ),
          child: Row(children: [
            Container(
              width: 10, height: 10,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: isRunning ? AetherColors.success : Colors.red,
                boxShadow: [BoxShadow(
                  color: (isRunning
                      ? AetherColors.success
                      : Colors.red).withOpacity(0.6),
                  blurRadius: 6,
                )],
              ),
            ),
            const SizedBox(width: 10),
            Expanded(child: Column(
                crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(isRunning ? 'Runtime Aktif' : 'Runtime Kapalı',
                  style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.bold,
                      fontSize: 14)),
              if (info != null)
                Text(
                  'v${info.version} · ${info.backend} · ${info.workerCount} worker',
                  style: const TextStyle(color: Colors.white38, fontSize: 11),
                ),
            ])),
          ]),
        );
      },
    );
  }
}

// ── Metrik grid ────────────────────────────────────────────────

class _MetricsGrid extends StatelessWidget {
  final rust.MetricsSnapshot? snapshot;
  const _MetricsGrid({required this.snapshot});

  @override
  Widget build(BuildContext context) => GridView.count(
    crossAxisCount: 2,
    shrinkWrap: true,
    crossAxisSpacing: 10,
    mainAxisSpacing: 10,
    childAspectRatio: 2.0,
    physics: const NeverScrollableScrollPhysics(),
    children: [
      _MetricTile(label: 'Tamamlandı',     value: snapshot?.completedTasks,
          color: AetherColors.success,  icon: Icons.check_circle_outline),
      _MetricTile(label: 'Başarısız',      value: snapshot?.failedTasks,
          color: AetherColors.danger,  icon: Icons.error_outline),
      _MetricTile(label: 'Kuyrukta',       value: snapshot?.queuedTasks,
          color: AetherColors.warning,  icon: Icons.hourglass_bottom),
      _MetricTile(label: 'Yeniden Deneme', value: snapshot?.retriedTasks,
          color: const Color(0xFF7E57C2),  icon: Icons.refresh),
    ],
  );
}

class _MetricTile extends StatelessWidget {
  final String   label;
  final int?     value;
  final Color    color;
  final IconData icon;

  const _MetricTile({
    required this.label,  required this.value,
    required this.color,  required this.icon,
  });

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
    decoration: BoxDecoration(
      color: color.withOpacity(0.08),
      borderRadius: BorderRadius.circular(12),
      border: Border.all(color: color.withOpacity(0.2)),
    ),
    child: Row(children: [
      Icon(icon, color: color, size: 22),
      const SizedBox(width: 10),
      Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
        Text(value != null ? '$value' : '—',
            style: TextStyle(
                color: color, fontSize: 20, fontWeight: FontWeight.bold)),
        Text(label,
            style: const TextStyle(color: Colors.white38, fontSize: 11)),
      ]),
    ]),
  );
}

// ── Logo ──────────────────────────────────────────────────────

class _AetherLogo extends StatelessWidget {
  const _AetherLogo();
  @override
  Widget build(BuildContext context) => Container(
    width: 32, height: 32,
    decoration: BoxDecoration(
      gradient: const LinearGradient(
          colors: [AetherColors.primary, Color(0xFF3F51B5)],
          begin: Alignment.topLeft, end: Alignment.bottomRight),
      borderRadius: BorderRadius.circular(8),
    ),
    child: const Icon(Icons.offline_bolt, color: Colors.white, size: 20),
  );
}
