// ============================================================
// flutter_app/lib/screens/approvals_screen.dart
//
// B5 / Faz 1c — Bekleyen Onaylar
//
// SecurityGovernor bir çağrıya "RequiresApproval" dediğinde agent
// duraklar. Bu ekran o çağrıları KOMUTUN KENDİSİYLE (argümanlar dahil)
// gösterir; kullanıcı onaylar ya da reddeder.
//   • Onayla  → çağrı çalışır, agent kalan adımlarla devam eder.
//               (Yanlışlıkla dokunmaya karşı onay penceresi var.)
//   • Reddet  → execution kalıcı olarak reddedilir, hiçbir şey çalışmaz.
// Onaylar uygulama kapansa da korunur; ~60 dk sonra otomatik reddedilir.
//
// Not: Kesin yasaklı komutlar (ör. `sh -c`) buraya HİÇ düşmez — politika
// onları baştan reddeder.
// ============================================================

import 'package:flutter/material.dart';

import '../api/aetheros_api.dart';
import '../core/app_error.dart';
import '../core/app_theme.dart';
import '../core/approval_format.dart';
import '../core/lifecycle_poller.dart';
import '../src/rust/api/aetheros.dart' as rust;

class ApprovalsScreen extends StatefulWidget {
  const ApprovalsScreen({super.key});

  @override
  State<ApprovalsScreen> createState() => _ApprovalsScreenState();
}

class _ApprovalsScreenState extends State<ApprovalsScreen> {
  List<rust.PendingApproval> _items = [];
  bool _loading = true;
  String? _error;
  final Set<String> _busy = {};
  late final LifecyclePoller _poller;

  @override
  void initState() {
    super.initState();
    _poller = LifecyclePoller(
      interval: const Duration(seconds: 3),
      onTick: () => _load(silent: true),
    );
    _poller.start();
  }

  @override
  void dispose() {
    _poller.stop();
    super.dispose();
  }

  Future<void> _load({bool silent = false}) async {
    if (!silent && mounted) setState(() => _loading = true);
    try {
      final items = await AetherApi.listPendingApprovals();
      // En eski önce: ilk bekleyen en üstte.
      items.sort((a, b) => a.createdAt.compareTo(b.createdAt));
      if (!mounted) return;
      setState(() {
        _items = items;
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

  Future<void> _respond(rust.PendingApproval item, bool approved) async {
    if (_busy.contains(item.id)) return;

    if (approved) {
      final ok = await _confirmApprove(item);
      if (ok != true) return;
    }

    setState(() => _busy.add(item.id));
    try {
      await AetherApi.respondToApproval(approvalId: item.id, approved: approved);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text(approved
            ? 'Onaylandı — agent devam ediyor.'
            : 'Reddedildi — hiçbir şey çalıştırılmadı.'),
      ));
    } catch (e) {
      if (!mounted) return;
      // Süresi dolmuş / zaten cevaplanmış olabilir; liste yenilenecek.
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text(userFacingError(e)),
        backgroundColor: AetherColors.danger,
      ));
    } finally {
      if (mounted) setState(() => _busy.remove(item.id));
      await _load(silent: true);
    }
  }

  Future<bool?> _confirmApprove(rust.PendingApproval item) {
    return showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: AetherColors.surface,
        title: const Text('Bu komut çalıştırılsın mı?'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _CommandBox(text: commandLine(item.toolName, item.arguments)),
            const SizedBox(height: AetherSpacing.md),
            const Text(
              'Onayladığında bu komut cihazında çalışır ve geri alınamayabilir.',
              style: TextStyle(color: AetherColors.textMuted, fontSize: 13),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Vazgeç'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Çalıştır'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AetherColors.background,
      appBar: AppBar(
        title: const Text('Bekleyen Onaylar'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: 'Yenile',
            onPressed: () => _load(),
          ),
        ],
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }

    final children = <Widget>[];

    if (_error != null) {
      children.add(Padding(
        padding: const EdgeInsets.only(bottom: AetherSpacing.md),
        child: Text(
          'Liste güncellenemedi: $_error',
          style: const TextStyle(color: AetherColors.warning, fontSize: 12),
        ),
      ));
    }

    if (_items.isEmpty) {
      children.add(const Padding(
        padding: EdgeInsets.only(top: 80),
        child: Column(
          children: [
            Icon(Icons.verified_user_outlined,
                size: 56, color: AetherColors.textSubtle),
            SizedBox(height: AetherSpacing.md),
            Text('Bekleyen onay yok',
                style: TextStyle(color: AetherColors.textMuted, fontSize: 16)),
            SizedBox(height: AetherSpacing.xs),
            Text(
              'Riskli bir komut çalıştırılmak istendiğinde burada görünür.',
              textAlign: TextAlign.center,
              style: TextStyle(color: AetherColors.textSubtle, fontSize: 12),
            ),
          ],
        ),
      ));
    } else {
      final now = DateTime.now().millisecondsSinceEpoch;
      for (final item in _items) {
        children.add(_ApprovalCard(
          item: item,
          nowMs: now,
          busy: _busy.contains(item.id),
          onApprove: () => _respond(item, true),
          onReject: () => _respond(item, false),
        ));
        children.add(const SizedBox(height: AetherSpacing.md));
      }
    }

    return RefreshIndicator(
      onRefresh: () => _load(silent: true),
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(AetherSpacing.lg),
        children: children,
      ),
    );
  }
}

// ── Kart ──────────────────────────────────────────────────────

class _ApprovalCard extends StatelessWidget {
  final rust.PendingApproval item;
  final int nowMs;
  final bool busy;
  final VoidCallback onApprove;
  final VoidCallback onReject;

  const _ApprovalCard({
    required this.item,
    required this.nowMs,
    required this.busy,
    required this.onApprove,
    required this.onReject,
  });

  @override
  Widget build(BuildContext context) {
    final remaining = remainingLabel(item.createdAt, nowMs);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(AetherSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(children: [
              const Icon(Icons.gpp_maybe_outlined,
                  color: AetherColors.warning, size: 20),
              const SizedBox(width: AetherSpacing.sm),
              Expanded(
                child: Text(
                  item.objective.isEmpty ? '(hedef belirtilmemiş)' : item.objective,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                      color: AetherColors.text,
                      fontWeight: FontWeight.bold,
                      fontSize: 14),
                ),
              ),
            ]),
            const SizedBox(height: AetherSpacing.xs),
            Text(
              [
                ageLabel(item.createdAt, nowMs),
                if (remaining != null) remaining else 'süresi doluyor',
              ].join(' · '),
              style: const TextStyle(color: AetherColors.textSubtle, fontSize: 11),
            ),
            const SizedBox(height: AetherSpacing.md),
            _CommandBox(text: commandLine(item.toolName, item.arguments)),
            const SizedBox(height: AetherSpacing.sm),
            Text(
              item.reason,
              style: const TextStyle(color: AetherColors.textMuted, fontSize: 12),
            ),
            const SizedBox(height: AetherSpacing.lg),
            Row(children: [
              Expanded(
                child: OutlinedButton.icon(
                  style: OutlinedButton.styleFrom(
                    foregroundColor: AetherColors.danger,
                    side: const BorderSide(color: AetherColors.danger),
                  ),
                  onPressed: busy ? null : onReject,
                  icon: const Icon(Icons.close),
                  label: const Text('Reddet'),
                ),
              ),
              const SizedBox(width: AetherSpacing.md),
              Expanded(
                child: FilledButton.icon(
                  style: FilledButton.styleFrom(
                      backgroundColor: AetherColors.success),
                  onPressed: busy ? null : onApprove,
                  icon: busy
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.check),
                  label: const Text('Onayla'),
                ),
              ),
            ]),
          ],
        ),
      ),
    );
  }
}

/// Komutu monospace, seçilebilir ve kaydırılabilir kutuda gösterir
/// (uzun komutlar kırpılmaz — onaylamadan önce TAMAMI görülmeli).
class _CommandBox extends StatelessWidget {
  final String text;
  const _CommandBox({required this.text});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      constraints: const BoxConstraints(maxHeight: 160),
      padding: const EdgeInsets.all(AetherSpacing.md),
      decoration: BoxDecoration(
        color: AetherColors.backgroundDeep,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white12),
      ),
      child: SingleChildScrollView(
        child: SelectableText(
          text,
          style: const TextStyle(
            color: AetherColors.text,
            fontFamily: 'monospace',
            fontSize: 12.5,
            height: 1.4,
          ),
        ),
      ),
    );
  }
}
