// ============================================================
// flutter_app/lib/screens/terminal_screen.dart
//
// Faz 2 — Terminal
//
// Kendi yazdığın TEK SATIR komut, agent'larla AYNI politika motorundan
// geçer:
//   • Yasaklı (örn. `sh -c ...`)  → çalışmaz, gerekçe gösterilir
//   • Serbest (örn. `ls`)         → doğrudan çalışır
//   • Onay gerekli (örn. `touch`) → sayfa içinde onay kutusu, sonra çalışır
//
// Shell yok: `&&`, `;`, `|`, `>` düz argüman sayılır. Komutlar uygulama
// çalışma klasöründe koşar. Her çalıştırma "Denetim Kayıtları"na düşer.
// Geçmiş yalnız bu açılışta bellekte tutulur.
// ============================================================

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../api/aetheros_api.dart';
import '../core/app_error.dart';
import '../core/app_theme.dart';
import '../core/approval_format.dart';
import '../core/terminal_format.dart';

enum _Status { running, done, failed, denied, cancelled, error }

class _Entry {
  final String command;
  _Status status = _Status.running;
  String output = '';
  bool truncated = false;
  _Entry(this.command);
}

class TerminalScreen extends StatefulWidget {
  const TerminalScreen({super.key});

  @override
  State<TerminalScreen> createState() => _TerminalScreenState();
}

class _TerminalScreenState extends State<TerminalScreen> {
  final _ctrl = TextEditingController();
  final _focus = FocusNode();
  final _scroll = ScrollController();
  final List<_Entry> _entries = [];
  bool _busy = false;

  @override
  void dispose() {
    _ctrl.dispose();
    _focus.dispose();
    _scroll.dispose();
    super.dispose();
  }

  void _scrollToEnd() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scroll.hasClients) return;
      _scroll.animateTo(
        _scroll.position.maxScrollExtent,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      );
    });
  }

  Future<void> _submit([String? override]) async {
    final line = (override ?? _ctrl.text).trim();
    if (line.isEmpty || _busy) return;

    final entry = _Entry(line);
    setState(() {
      _busy = true;
      _entries.add(entry);
      _ctrl.clear();
    });
    _scrollToEnd();

    try {
      final chk = await AetherApi.terminalCheck(line);
      switch (parseVerdict(chk.verdict)) {
        case TerminalVerdict.deny:
          entry.status = _Status.denied;
          entry.output = chk.reason;
        case TerminalVerdict.ask:
          final ok = await _confirm(chk.argv, chk.reason);
          if (ok != true) {
            entry.status = _Status.cancelled;
            entry.output = 'Çalıştırılmadı (onaylanmadı).';
          } else {
            await _run(entry, line, confirmed: true);
          }
        case TerminalVerdict.allow:
          await _run(entry, line, confirmed: false);
      }
    } catch (e) {
      entry.status = _Status.error;
      entry.output = userFacingError(e);
    } finally {
      if (mounted) setState(() => _busy = false);
      _scrollToEnd();
    }
  }

  Future<void> _run(_Entry entry, String line, {required bool confirmed}) async {
    final r = await AetherApi.terminalRun(line, confirmed: confirmed);
    entry.status = r.success ? _Status.done : _Status.failed;
    entry.output = r.output;
    entry.truncated = r.truncated;
  }

  Future<bool?> _confirm(List<String> argv, String reason) {
    final cmd = argv.map(quoteArg).join(' ');
    return showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: AetherColors.surface,
        title: const Text('Bu komut çalıştırılsın mı?'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(AetherSpacing.md),
              decoration: BoxDecoration(
                color: AetherColors.backgroundDeep,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white12),
              ),
              child: SelectableText(
                cmd,
                style: const TextStyle(
                    color: AetherColors.text,
                    fontFamily: 'monospace',
                    fontSize: 13),
              ),
            ),
            const SizedBox(height: AetherSpacing.md),
            Text(reason,
                style: const TextStyle(
                    color: AetherColors.textMuted, fontSize: 12)),
            const SizedBox(height: AetherSpacing.sm),
            const Text(
              'Onayladığında komut cihazında çalışır ve geri alınamayabilir.',
              style: TextStyle(color: AetherColors.textMuted, fontSize: 12),
            ),
          ],
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('Vazgeç')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('Çalıştır')),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AetherColors.background,
      appBar: AppBar(
        title: const Text('Terminal'),
        actions: [
          IconButton(
            icon: const Icon(Icons.delete_sweep_outlined),
            tooltip: 'Ekranı temizle',
            onPressed: _entries.isEmpty
                ? null
                : () => setState(_entries.clear),
          ),
        ],
      ),
      body: Column(children: [
        Expanded(
          child: _entries.isEmpty
              ? const _Hint()
              : ListView.builder(
                  controller: _scroll,
                  padding: const EdgeInsets.all(AetherSpacing.lg),
                  itemCount: _entries.length,
                  itemBuilder: (_, i) => _EntryView(
                    entry: _entries[i],
                    onReuse: (cmd) {
                      _ctrl.text = cmd;
                      _ctrl.selection =
                          TextSelection.collapsed(offset: cmd.length);
                      _focus.requestFocus();
                    },
                  ),
                ),
        ),
        const Padding(
          padding: EdgeInsets.symmetric(
              horizontal: AetherSpacing.lg, vertical: 2),
          child: Align(
            alignment: Alignment.centerLeft,
            child: Text(
              'Gerçek komut satırı — doğal dil için Agent ekranını kullan.',
              style: TextStyle(color: Colors.white38, fontSize: 11),
            ),
          ),
        ),
        SizedBox(
          height: 40,
          child: ListView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.lg),
            children: [
              for (final c in kQuickCommands)
                Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: ActionChip(
                    label: Text(c,
                        style: const TextStyle(fontFamily: 'monospace')),
                    onPressed: _busy ? null : () => _submit(c),
                  ),
                ),
            ],
          ),
        ),
        SafeArea(
          top: false,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(
                AetherSpacing.lg, 4, AetherSpacing.lg, AetherSpacing.md),
            child: Row(children: [
              Expanded(
                child: TextField(
                  controller: _ctrl,
                  focusNode: _focus,
                  enabled: !_busy,
                  autocorrect: false,
                  enableSuggestions: false,
                  textInputAction: TextInputAction.send,
                  onSubmitted: (_) => _submit(),
                  style: const TextStyle(
                      color: AetherColors.text,
                      fontFamily: 'monospace',
                      fontSize: 14),
                  decoration: InputDecoration(
                    prefixText: '\$ ',
                    prefixStyle: const TextStyle(
                        color: AetherColors.primary,
                        fontFamily: 'monospace'),
                    hintText: 'komut yaz (örn. touch notlar.txt)',
                    hintStyle: const TextStyle(
                        color: Colors.white24, fontSize: 13),
                    filled: true,
                    fillColor: Colors.white.withOpacity(0.05),
                    border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                        borderSide: BorderSide.none),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              IconButton.filled(
                onPressed: _busy ? null : () => _submit(),
                icon: _busy
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2))
                    : const Icon(Icons.play_arrow),
                tooltip: 'Çalıştır',
              ),
            ]),
          ),
        ),
      ]),
    );
  }
}

class _Hint extends StatelessWidget {
  const _Hint();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Padding(
        padding: EdgeInsets.all(AetherSpacing.xl),
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          Icon(Icons.terminal, size: 52, color: AetherColors.textSubtle),
          SizedBox(height: AetherSpacing.md),
          Text('Tek satır komut',
              style: TextStyle(color: AetherColors.textMuted, fontSize: 16)),
          SizedBox(height: AetherSpacing.xs),
          Text(
            'Bu bir KOMUT satırı, doğal dil değil: ls, touch notlar.txt.\n'
            'Türkçe talimat için Agent ekranını (ya da Chat → Agent ile yap) kullan.\n'
            'Shell yok: &&, ;, | ve > düz argüman sayılır.\n'
            'Riskli komutlar onay ister, yasaklılar çalışmaz.\n'
            'Her çalıştırma Denetim Kayıtları\'na yazılır.',
            textAlign: TextAlign.center,
            style: TextStyle(color: AetherColors.textSubtle, fontSize: 12),
          ),
        ]),
      ),
    );
  }
}

class _EntryView extends StatelessWidget {
  final _Entry entry;
  final void Function(String command) onReuse;
  const _EntryView({required this.entry, required this.onReuse});

  (String, Color) get _badge {
    switch (entry.status) {
      case _Status.running:
        return ('çalışıyor…', AetherColors.textMuted);
      case _Status.done:
        return ('tamam', AetherColors.success);
      case _Status.failed:
        return ('başarısız', AetherColors.danger);
      case _Status.denied:
        return ('yasak', AetherColors.danger);
      case _Status.cancelled:
        return ('iptal', AetherColors.warning);
      case _Status.error:
        return ('hata', AetherColors.danger);
    }
  }

  @override
  Widget build(BuildContext context) {
    final (label, color) = _badge;
    final out = limitLines(entry.output);
    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: AetherColors.backgroundDeep,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white12),
      ),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [
          Expanded(
            child: InkWell(
              onTap: () => onReuse(entry.command),
              child: Text('\$ ${entry.command}',
                  style: const TextStyle(
                      color: AetherColors.text,
                      fontFamily: 'monospace',
                      fontWeight: FontWeight.bold,
                      fontSize: 13)),
            ),
          ),
          Text(label,
              style: TextStyle(
                  color: color, fontSize: 11, fontWeight: FontWeight.bold)),
        ]),
        if (entry.output.isNotEmpty) ...[
          const SizedBox(height: 6),
          SelectableText(out,
              style: TextStyle(
                  color: entry.status == _Status.done
                      ? Colors.white70
                      : color,
                  fontFamily: 'monospace',
                  fontSize: 12,
                  height: 1.35)),
        ] else if (entry.status == _Status.done) ...[
          const SizedBox(height: 6),
          const Text('(çıktı yok — komut sessizce başarılı olabilir)',
              style: TextStyle(color: Colors.white38, fontSize: 11)),
        ],
        if (entry.truncated)
          const Padding(
            padding: EdgeInsets.only(top: 4),
            child: Text('Çıktı 20 000 karaktere kırpıldı.',
                style: TextStyle(color: Colors.white38, fontSize: 11)),
          ),
        if (entry.output.isNotEmpty)
          Align(
            alignment: Alignment.centerRight,
            child: TextButton.icon(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: entry.output));
                ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
                    content: Text('Çıktı kopyalandı'),
                    duration: Duration(seconds: 1)));
              },
              icon: const Icon(Icons.copy, size: 14),
              label: const Text('Kopyala', style: TextStyle(fontSize: 11)),
            ),
          ),
      ]),
    );
  }
}
