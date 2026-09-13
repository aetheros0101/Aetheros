// ============================================================
// flutter_app/lib/screens/log_screen.dart  — Sprint 7
//
// WASM runtime event log'larını canlı olarak gösterir.
//
// ÖZELLİKLER:
//   - 2 saniyelik polling (get_recent_logs bridge call)
//   - Seviye filtresi: Tümü / INFO / WARN / ERROR
//   - Task ID'ye göre arama
//   - Renk kodlaması: INFO=beyaz, WARN=turuncu, ERROR=kırmızı
//   - Task ID'ye dokunarak kopyala
//   - "Temizle" → sadece listeyi gizler, buffer etkilenmez
// ============================================================

import 'dart:async';
import '../core/lifecycle_poller.dart';
import '../core/app_error.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;
import '../core/app_theme.dart';

class LogScreen extends StatefulWidget {
  /// Opsiyonel: belirli bir task için filtreli açılış
  final String? filterTaskId;
  const LogScreen({super.key, this.filterTaskId});

  @override
  State<LogScreen> createState() => _LogScreenState();
}

class _LogScreenState extends State<LogScreen> {
  List<rust.LogRecord> _all     = [];
  String _levelFilter          = 'Tümü'; // Tümü | INFO | WARN | ERROR
  String _taskSearch           = '';
  bool   _autoScroll           = true;
  bool   _loading              = true;
  bool   _hidden               = false; // "Temizle" → geçici gizle

  final _scrollCtrl   = ScrollController();
  final _searchCtrl   = TextEditingController();
  late final LifecyclePoller _poller;
  Timer? _searchDebounce;

  @override
  void initState() {
    super.initState();
    if (widget.filterTaskId != null) {
      _taskSearch = widget.filterTaskId!;
      _searchCtrl.text = widget.filterTaskId!;
    }
    _poller = LifecyclePoller(interval: const Duration(seconds: 2), onTick: _fetch);
    _poller.start();
  }

  @override
  void dispose() {
    _poller.stop();
    _searchDebounce?.cancel();
    _scrollCtrl.dispose();
    _searchCtrl.dispose();
    super.dispose();
  }

  Future<void> _fetch() async {
    try {
      final entries = _taskSearch.isNotEmpty
          ? await AetherApi.getTaskLogs(_taskSearch)
          : await AetherApi.getRecentLogs(limit: 200);

      if (!mounted) return;
      setState(() { _all = entries; _loading = false; });

      if (_autoScroll && _scrollCtrl.hasClients) {
        _scrollCtrl.animateTo(
          0, duration: const Duration(milliseconds: 300),
          curve: Curves.easeOut);
      }
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  List<rust.LogRecord> get _filtered {
    if (_hidden) return [];
    return _all.where((e) {
      if (_levelFilter != 'Tümü' && e.level != _levelFilter) return false;
      return true;
    }).toList();
  }

  @override
  Widget build(BuildContext context) {
    final entries = _filtered;

    return Scaffold(
      backgroundColor: const Color(0xFF0A0A14),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0A0A14),
        title: Row(children: [
          const Icon(Icons.terminal, color: AetherColors.primary, size: 18),
          const SizedBox(width: 8),
          const Text('Runtime Logları',
              style: TextStyle(color: Colors.white, fontSize: 16)),
          const SizedBox(width: 8),
          // Entry sayısı badge
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
            decoration: BoxDecoration(
              color: AetherColors.primary.withOpacity(0.2),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Text(
              '${_all.length}',
              style: const TextStyle(
                  color: AetherColors.primary, fontSize: 11,
                  fontWeight: FontWeight.bold),
            ),
          ),
        ]),
        iconTheme: const IconThemeData(color: Colors.white60),
        actions: [
          // Auto-scroll toggle
          IconButton(
            icon: Icon(
              _autoScroll ? Icons.lock : Icons.lock_open,
              color: _autoScroll
                  ? AetherColors.primary : Colors.white38,
              size: 18,
            ),
            tooltip: _autoScroll ? 'Otomatik kaydır: Açık' : 'Otomatik kaydır: Kapalı',
            onPressed: () => setState(() => _autoScroll = !_autoScroll),
          ),
          // Temizle (geçici)
          IconButton(
            icon: const Icon(Icons.clear_all, color: Colors.white38, size: 20),
            tooltip: 'Görünümü temizle',
            onPressed: () {
              setState(() { _hidden = true; });
              Future.delayed(const Duration(seconds: 10),
                  () { if (mounted) setState(() => _hidden = false); });
            },
          ),
        ],
      ),
      body: Column(children: [

        // ── Filtre bar ──────────────────────────────────
        Container(
          color: AetherColors.background,
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
          child: Column(children: [

            // Seviye filtresi
            Row(children: [
              for (final lvl in ['Tümü', 'INFO', 'WARN', 'ERROR'])
                Padding(
                  padding: const EdgeInsets.only(right: 6),
                  child: _LevelChip(
                    label: lvl,
                    selected: _levelFilter == lvl,
                    onTap: () => setState(() {
                      _levelFilter = lvl;
                      _hidden = false;
                    }),
                  ),
                ),
              const Spacer(),
              // Canlı göstergesi
              _loading
                  ? const SizedBox(width: 10, height: 10,
                      child: CircularProgressIndicator(
                          strokeWidth: 1.5, color: Colors.white24))
                  : Row(children: [
                      Container(width: 6, height: 6,
                          decoration: const BoxDecoration(
                              color: AetherColors.success,
                              shape: BoxShape.circle)),
                      const SizedBox(width: 4),
                      const Text('Canlı',
                          style: TextStyle(
                              color: AetherColors.success, fontSize: 10)),
                    ]),
            ]),
            const SizedBox(height: 8),

            // Task ID arama
            SizedBox(
              height: 34,
              child: TextField(
                controller: _searchCtrl,
                style: const TextStyle(
                    color: Colors.white70, fontSize: 12,
                    fontFamily: 'monospace'),
                decoration: InputDecoration(
                  hintText: 'Task ID ile filtrele...',
                  hintStyle: const TextStyle(
                      color: Colors.white24, fontSize: 12),
                  prefixIcon: const Icon(Icons.search,
                      color: Colors.white24, size: 16),
                  suffixIcon: _taskSearch.isNotEmpty
                      ? InkWell(
                          onTap: () {
                            _searchCtrl.clear();
                            setState(() => _taskSearch = '');
                            _fetch();
                          },
                          child: const Icon(Icons.close,
                              color: Colors.white38, size: 16),
                        )
                      : null,
                  filled: true,
                  fillColor: Colors.white.withOpacity(0.04),
                  contentPadding: const EdgeInsets.symmetric(vertical: 0),
                  border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide.none),
                ),
                onChanged: (v) {
                  final value = v.trim();
                  setState(() => _taskSearch = value);
                  _searchDebounce?.cancel();
                  _searchDebounce = Timer(const Duration(milliseconds: 350), _fetch);
                },
              ),
            ),
          ]),
        ),

        // ── Log listesi ─────────────────────────────────
        Expanded(
          child: entries.isEmpty
              ? _EmptyLog(loading: _loading, hidden: _hidden,
                  onShow: () => setState(() => _hidden = false))
              : ListView.builder(
                  controller: _scrollCtrl,
                  padding: const EdgeInsets.fromLTRB(0, 4, 0, 24),
                  itemCount: entries.length,
                  itemBuilder: (_, i) => _LogRow(
                    entry: entries[i],
                    onCopyTaskId: (id) {
                      Clipboard.setData(ClipboardData(text: id));
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(
                          content: Text('Task ID kopyalandı'),
                          duration: Duration(seconds: 1),
                          backgroundColor: AetherColors.primary,
                        ),
                      );
                    },
                  ),
                ),
        ),
      ]),
    );
  }
}

// ── Seviye chip ───────────────────────────────────────────

class _LevelChip extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _LevelChip({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  static Color _color(String lvl) {
    switch (lvl) {
      case 'ERROR': return AetherColors.danger;
      case 'WARN':  return const Color(0xFFFF9800);
      case 'INFO':  return const Color(0xFF4FC3F7);
      default:      return AetherColors.primary;
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = _color(label);
    return InkWell(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 150),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
        decoration: BoxDecoration(
          color: selected ? c.withOpacity(0.18) : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
              color: selected ? c.withOpacity(0.6) : Colors.white12),
        ),
        child: Text(label,
            style: TextStyle(
                color: selected ? c : Colors.white38,
                fontSize: 11,
                fontWeight: selected
                    ? FontWeight.bold : FontWeight.normal)),
      ),
    );
  }
}

// ── Tek log satırı ────────────────────────────────────────

class _LogRow extends StatelessWidget {
  final rust.LogRecord entry;
  final void Function(String) onCopyTaskId;
  const _LogRow({required this.entry, required this.onCopyTaskId});

  Color get _levelColor {
    switch (entry.level) {
      case 'ERROR': return AetherColors.danger;
      case 'WARN':  return const Color(0xFFFF9800);
      default:      return const Color(0xFF4FC3F7);
    }
  }

  String get _timeStr {
    final dt = DateTime.fromMillisecondsSinceEpoch(
        entry.timestampMs.toInt()).toLocal();
    final h  = dt.hour.toString().padLeft(2, '0');
    final m  = dt.minute.toString().padLeft(2, '0');
    final s  = dt.second.toString().padLeft(2, '0');
    final ms = dt.millisecond.toString().padLeft(3, '0');
    return '$h:$m:$s.$ms';
  }

  @override
  Widget build(BuildContext context) {
    final taskId = entry.taskId;
    final short  = taskId != null
        ? taskId.substring(0, 8).toUpperCase()
        : null;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
      decoration: BoxDecoration(
        border: Border(
          left: BorderSide(color: _levelColor.withOpacity(0.4), width: 2),
        ),
      ),
      child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [

        // Zaman damgası
        Text(_timeStr,
            style: const TextStyle(
                color: Colors.white24, fontSize: 10,
                fontFamily: 'monospace')),
        const SizedBox(width: 8),

        // Seviye badge
        SizedBox(
          width: 38,
          child: Text(
            entry.level,
            style: TextStyle(
                color: _levelColor, fontSize: 10,
                fontWeight: FontWeight.bold),
          ),
        ),

        // Mesaj + task ID
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(entry.message,
                  style: const TextStyle(
                      color: Colors.white70, fontSize: 12)),
              if (short != null) ...[
                const SizedBox(height: 2),
                InkWell(
                  onTap: () => onCopyTaskId(taskId!),
                  child: Text(
                    short,
                    style: const TextStyle(
                        color: Color(0xFF80CBC4),
                        fontFamily: 'monospace',
                        fontSize: 10),
                  ),
                ),
              ],
            ],
          ),
        ),
      ]),
    );
  }
}

// ── Boş durum ─────────────────────────────────────────────

class _EmptyLog extends StatelessWidget {
  final bool loading;
  final bool hidden;
  final VoidCallback onShow;
  const _EmptyLog({
    required this.loading,
    required this.hidden,
    required this.onShow,
  });

  @override
  Widget build(BuildContext context) => Center(
    child: Column(mainAxisSize: MainAxisSize.min, children: [
      Icon(
        loading ? Icons.hourglass_empty : Icons.receipt_long_outlined,
        color: Colors.white12, size: 48,
      ),
      const SizedBox(height: 12),
      Text(
        hidden ? 'Görünüm temizlendi' : (loading ? 'Yükleniyor...' : 'Henüz log yok'),
        style: const TextStyle(color: Colors.white38, fontSize: 13),
      ),
      if (hidden) ...[
        const SizedBox(height: 12),
        TextButton.icon(
          onPressed: onShow,
          icon: const Icon(Icons.visibility_outlined, size: 16),
          label: const Text('Göster'),
          style: TextButton.styleFrom(
              foregroundColor: AetherColors.primary),
        ),
      ],
    ]),
  );
}
