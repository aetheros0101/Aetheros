// ============================================================
// flutter_app/lib/screens/files_screen.dart
//
// Dosyalar — agent'ların ve terminalin çalıştığı çalışma alanı
// ({docDir}/workspace). Gezin, metin dosyası görüntüle/düzenle, oluştur,
// yeniden adlandır, sil; dışarıdan içe, metin dosyasını dışarı aktar.
//
// Güvenlik sınırı Rust'tadır: çalışma alanı dışına çıkılamaz, `.git`
// değiştirilemez, kök silinemez. Silme geri alınamaz → onay sorulur.
// Dosya, sen açtıktan sonra değiştiyse (ör. bir agent yazdıysa) kaydederken
// çakışma uyarısı çıkar; agent'ın yazdığı sessizce ezilmez.
// ============================================================

import 'dart:convert';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../api/aetheros_api.dart';
import '../core/app_error.dart';
import '../core/app_theme.dart';
import '../core/files_format.dart';
import '../core/load_error_banner.dart';
import '../src/rust/api/aetheros.dart' as rust;

class FilesScreen extends StatefulWidget {
  const FilesScreen({super.key});

  @override
  State<FilesScreen> createState() => _FilesScreenState();
}

class _FilesScreenState extends State<FilesScreen> {
  final _banner = LoadErrorBanner();
  String _dir = '';
  List<rust.WsItem> _items = const [];
  bool _loading = true;
  bool _showHidden = false;
  String? _root;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _banner.attach(context);
  }

  @override
  void initState() {
    super.initState();
    _load();
    AetherApi.workspaceRoot().then((r) {
      if (mounted) setState(() => _root = r);
    }).catchError((_) {});
  }

  @override
  void dispose() {
    _banner.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    if (mounted) setState(() => _loading = true);
    try {
      final items = await AetherApi.listDir(_dir, includeHidden: _showHidden);
      if (!mounted) return;
      setState(() {
        _items = items;
        _loading = false;
      });
      _banner.clear();
    } catch (e) {
      if (!mounted) return;
      setState(() => _loading = false);
      _banner.report(e, onRetry: _load);
    }
  }

  void _open(String dir) {
    setState(() => _dir = dir);
    _load();
  }

  void _toast(String msg, {bool error = false}) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(
        content: Text(msg),
        backgroundColor: error ? AetherColors.danger : null,
      ));
  }

  Future<String?> _askName(String title, {String initial = '', String ok = 'Tamam'}) {
    final ctrl = TextEditingController(text: initial);
    String? err;
    return showDialog<String>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setS) => AlertDialog(
          backgroundColor: AetherColors.surface,
          title: Text(title),
          content: TextField(
            controller: ctrl,
            autofocus: true,
            decoration: InputDecoration(errorText: err, hintText: 'ad'),
            onSubmitted: (_) {
              final e = validateEntryName(ctrl.text);
              if (e != null) {
                setS(() => err = e);
              } else {
                Navigator.pop(ctx, ctrl.text.trim());
              }
            },
          ),
          actions: [
            TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Vazgeç')),
            FilledButton(
              onPressed: () {
                final e = validateEntryName(ctrl.text);
                if (e != null) {
                  setS(() => err = e);
                } else {
                  Navigator.pop(ctx, ctrl.text.trim());
                }
              },
              child: Text(ok),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _create({required bool isDir}) async {
    final name = await _askName(isDir ? 'Yeni klasör' : 'Yeni dosya', ok: 'Oluştur');
    if (name == null) return;
    try {
      await AetherApi.createEntry(joinPath(_dir, name), isDir: isDir);
      await _load();
      if (!isDir && mounted) await _openFile(joinPath(_dir, name));
    } catch (e) {
      if (mounted) _toast(userFacingError(e), error: true);
    }
  }

  Future<void> _rename(rust.WsItem item) async {
    final name = await _askName('Yeniden adlandır', initial: item.name, ok: 'Kaydet');
    if (name == null || name == item.name) return;
    try {
      await AetherApi.renameEntry(item.path, joinPath(parentOf(item.path), name));
      await _load();
    } catch (e) {
      if (mounted) _toast(userFacingError(e), error: true);
    }
  }

  Future<void> _delete(rust.WsItem item) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: AetherColors.surface,
        title: Text(item.isDir ? 'Klasör silinsin mi?' : 'Dosya silinsin mi?'),
        content: Text(
          item.isDir
              ? "'${item.name}' ve içindeki her şey silinecek. Bu geri alınamaz."
              : "'${item.name}' silinecek. Bu geri alınamaz.",
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Vazgeç')),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: AetherColors.danger),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Sil'),
          ),
        ],
      ),
    );
    if (ok != true) return;
    try {
      await AetherApi.deleteEntry(item.path);
      await _load();
    } catch (e) {
      if (mounted) _toast(userFacingError(e), error: true);
    }
  }

  Future<void> _import() async {
    try {
      final result = await FilePicker.platform.pickFiles(allowMultiple: true);
      if (result == null) return;
      // Var olan adlar (gizliler dahil) — üzerine yazmamak için.
      final existing = (await AetherApi.listDir(_dir, includeHidden: true))
          .map((i) => i.name)
          .toList();
      var done = 0;
      final failed = <String>[];
      for (final f in result.files) {
        final src = f.path;
        if (src == null) {
          failed.add(f.name);
          continue;
        }
        final name = uniqueName(existing, f.name);
        try {
          await AetherApi.importFile(sourcePath: src, destDir: _dir, fileName: name);
          existing.add(name);
          done++;
        } catch (e) {
          failed.add('${f.name}: ${userFacingError(e)}');
        }
      }
      await _load();
      if (!mounted) return;
      if (failed.isEmpty) {
        _toast('$done dosya içe aktarıldı.');
      } else {
        _toast('$done aktarıldı, ${failed.length} hata: ${failed.first}', error: true);
      }
    } catch (e) {
      if (mounted) _toast(userFacingError(e), error: true);
    }
  }

  Future<void> _openFile(String path) async {
    await Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => FileEditorScreen(path: path)),
    );
    if (mounted) _load();
  }

  void _showAddSheet() {
    showModalBottomSheet<void>(
      context: context,
      backgroundColor: AetherColors.surface,
      builder: (ctx) => SafeArea(
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          ListTile(
            leading: const Icon(Icons.note_add_outlined),
            title: const Text('Yeni dosya'),
            onTap: () {
              Navigator.pop(ctx);
              _create(isDir: false);
            },
          ),
          ListTile(
            leading: const Icon(Icons.create_new_folder_outlined),
            title: const Text('Yeni klasör'),
            onTap: () {
              Navigator.pop(ctx);
              _create(isDir: true);
            },
          ),
          ListTile(
            leading: const Icon(Icons.file_download_outlined),
            title: const Text('Cihazdan içe aktar'),
            subtitle: const Text('Seçtiğin dosyalar bu klasöre kopyalanır'),
            onTap: () {
              Navigator.pop(ctx);
              _import();
            },
          ),
        ]),
      ),
    );
  }

  IconData _icon(rust.WsItem i) => switch (kindOf(i.name, isDir: i.isDir)) {
        FileKind.folder => Icons.folder,
        FileKind.code => Icons.code,
        FileKind.data => Icons.data_object,
        FileKind.image => Icons.image_outlined,
        FileKind.text => Icons.description_outlined,
        FileKind.other => Icons.insert_drive_file_outlined,
      };

  @override
  Widget build(BuildContext context) {
    return PopScope(
      canPop: _dir.isEmpty,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _open(parentOf(_dir));
      },
      child: Scaffold(
        backgroundColor: AetherColors.background,
        appBar: AppBar(
          title: const Text('Dosyalar'),
          actions: [
            IconButton(
              icon: Icon(_showHidden ? Icons.visibility : Icons.visibility_off_outlined),
              tooltip: _showHidden ? 'Gizlileri gizle' : 'Gizlileri göster',
              onPressed: () {
                setState(() => _showHidden = !_showHidden);
                _load();
              },
            ),
            IconButton(
              icon: const Icon(Icons.refresh),
              tooltip: 'Yenile',
              onPressed: _load,
            ),
          ],
        ),
        floatingActionButton: FloatingActionButton.extended(
          onPressed: _showAddSheet,
          icon: const Icon(Icons.add),
          label: const Text('Ekle'),
        ),
        body: Column(children: [
          _Crumbs(path: _dir, onTap: _open),
          const Divider(height: 1),
          Expanded(
            child: _loading && _items.isEmpty
                ? const Center(child: CircularProgressIndicator())
                : RefreshIndicator(
                    onRefresh: _load,
                    child: _items.isEmpty
                        ? ListView(children: [_EmptyHint(root: _root, atRoot: _dir.isEmpty)])
                        : ListView.separated(
                            padding: const EdgeInsets.only(bottom: 88),
                            itemCount: _items.length + (_dir.isEmpty ? 1 : 0),
                            separatorBuilder: (_, __) => const Divider(height: 1),
                            itemBuilder: (_, i) {
                              if (_dir.isEmpty && i == _items.length) {
                                return _EmptyHint(root: _root, atRoot: true, compact: true);
                              }
                              final item = _items[i];
                              return ListTile(
                                leading: Icon(
                                  _icon(item),
                                  color: item.isDir ? AetherColors.warning : AetherColors.textMuted,
                                ),
                                title: Text(
                                  item.name,
                                  style: TextStyle(
                                    color: item.hidden ? AetherColors.textSubtle : AetherColors.text,
                                  ),
                                ),
                                subtitle: item.isDir
                                    ? null
                                    : Text(formatSize(item.size),
                                        style: const TextStyle(color: AetherColors.textSubtle, fontSize: 12)),
                                onTap: () => item.isDir ? _open(item.path) : _openFile(item.path),
                                trailing: PopupMenuButton<String>(
                                  onSelected: (v) {
                                    if (v == 'rename') _rename(item);
                                    if (v == 'delete') _delete(item);
                                  },
                                  itemBuilder: (_) => const [
                                    PopupMenuItem(value: 'rename', child: Text('Yeniden adlandır')),
                                    PopupMenuItem(value: 'delete', child: Text('Sil')),
                                  ],
                                ),
                              );
                            },
                          ),
                  ),
          ),
        ]),
      ),
    );
  }
}

class _Crumbs extends StatelessWidget {
  final String path;
  final ValueChanged<String> onTap;
  const _Crumbs({required this.path, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final crumbs = breadcrumbs(path);
    return SizedBox(
      height: 44,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.md),
        children: [
          for (var i = 0; i < crumbs.length; i++) ...[
            if (i > 0) const Icon(Icons.chevron_right, size: 18, color: AetherColors.textSubtle),
            TextButton(
              onPressed: i == crumbs.length - 1 ? null : () => onTap(crumbs[i].path),
              child: Text(
                crumbs[i].label,
                style: TextStyle(
                  color: i == crumbs.length - 1 ? AetherColors.text : AetherColors.primary,
                  fontWeight: i == crumbs.length - 1 ? FontWeight.bold : FontWeight.normal,
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _EmptyHint extends StatelessWidget {
  final String? root;
  final bool atRoot;
  final bool compact;
  const _EmptyHint({required this.root, required this.atRoot, this.compact = false});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.all(compact ? AetherSpacing.lg : AetherSpacing.xxl),
      child: Column(children: [
        if (!compact) ...[
          const Icon(Icons.folder_open, size: 48, color: AetherColors.textSubtle),
          const SizedBox(height: AetherSpacing.md),
          const Text('Bu klasör boş.', style: TextStyle(color: AetherColors.textMuted)),
          const SizedBox(height: AetherSpacing.sm),
        ],
        if (atRoot)
          Text(
            "Agent'lar ve Terminal bu klasörde çalışır. Agent'a dosya vermek için "
            "'Ekle → Cihazdan içe aktar' ile kopyala, sonra hedefte göreli yolla anlat: "
            "\"notlar.txt dosyasını oku\".${root == null ? '' : '\n\n$root'}",
            textAlign: TextAlign.center,
            style: const TextStyle(color: AetherColors.textSubtle, fontSize: 12),
          ),
      ]),
    );
  }
}

// ── Metin dosyası görüntüleyici / düzenleyici ─────────────────

class FileEditorScreen extends StatefulWidget {
  final String path;
  const FileEditorScreen({super.key, required this.path});

  @override
  State<FileEditorScreen> createState() => _FileEditorScreenState();
}

class _FileEditorScreenState extends State<FileEditorScreen> {
  final _ctrl = TextEditingController();
  rust.WsFile? _file;
  String? _loadError;
  String _saved = '';
  bool _saving = false;

  bool get _dirty => _file != null && _ctrl.text != _saved;

  @override
  void initState() {
    super.initState();
    _ctrl.addListener(() => setState(() {}));
    _load();
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final f = await AetherApi.readFile(widget.path);
      if (!mounted) return;
      _ctrl.text = f.content;
      setState(() {
        _file = f;
        _saved = f.content;
        _loadError = null;
      });
    } catch (e) {
      if (mounted) setState(() => _loadError = userFacingError(e));
    }
  }

  Future<void> _save({bool overwrite = false}) async {
    final f = _file;
    if (f == null || _saving) return;
    setState(() => _saving = true);
    try {
      final w = await AetherApi.writeFile(
        widget.path,
        _ctrl.text,
        expectedVersion: overwrite ? null : f.version,
      );
      if (!mounted) return;
      setState(() {
        _file = w;
        _saved = w.content;
      });
      ScaffoldMessenger.of(context)
        ..hideCurrentSnackBar()
        ..showSnackBar(const SnackBar(content: Text('Kaydedildi.')));
    } catch (e) {
      if (!mounted) return;
      if (isWriteConflict(e)) {
        await _resolveConflict();
      } else {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(
          content: Text(userFacingError(e)),
          backgroundColor: AetherColors.danger,
        ));
      }
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _resolveConflict() async {
    final choice = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: AetherColors.surface,
        title: const Text('Dosya değişmiş'),
        content: const Text(
          'Sen açtıktan sonra bu dosya değişti (bir agent yazmış olabilir). '
          'Ne yapalım?',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, 'cancel'), child: const Text('İptal')),
          TextButton(
            onPressed: () => Navigator.pop(ctx, 'reload'),
            child: const Text('Yeniden yükle (değişikliklerim gider)'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, 'overwrite'),
            child: const Text('Üzerine yaz'),
          ),
        ],
      ),
    );
    if (!mounted) return;
    if (choice == 'reload') await _load();
    if (choice == 'overwrite') await _save(overwrite: true);
  }

  Future<void> _export() async {
    try {
      final bytes = Uint8List.fromList(utf8.encode(_ctrl.text));
      final out = await FilePicker.platform.saveFile(
        dialogTitle: 'Dosyayı kaydet',
        fileName: baseName(widget.path),
        bytes: bytes,
      );
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text(out == null ? 'Dışa aktarma iptal edildi.' : 'Dışa aktarıldı.'),
      ));
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(
          content: Text(userFacingError(e)),
          backgroundColor: AetherColors.danger,
        ));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final f = _file;
    return PopScope(
      canPop: !_dirty,
      onPopInvokedWithResult: (didPop, _) async {
        if (didPop) return;
        final discard = await showDialog<bool>(
          context: context,
          builder: (ctx) => AlertDialog(
            backgroundColor: AetherColors.surface,
            title: const Text('Kaydedilmemiş değişiklik var'),
            content: const Text('Çıkarsan değişiklikler kaybolur.'),
            actions: [
              TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Kal')),
              FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Çık')),
            ],
          ),
        );
        if (discard == true && context.mounted) Navigator.pop(context);
      },
      child: Scaffold(
        backgroundColor: AetherColors.background,
        appBar: AppBar(
          title: Text(baseName(widget.path), overflow: TextOverflow.ellipsis),
          actions: [
            if (f != null) ...[
              IconButton(
                icon: const Icon(Icons.copy),
                tooltip: 'Kopyala',
                onPressed: () async {
                  await Clipboard.setData(ClipboardData(text: _ctrl.text));
                  if (context.mounted) {
                    ScaffoldMessenger.of(context)
                        .showSnackBar(const SnackBar(content: Text('Panoya kopyalandı.')));
                  }
                },
              ),
              IconButton(
                icon: const Icon(Icons.file_upload_outlined),
                tooltip: 'Dışa aktar',
                onPressed: _export,
              ),
              if (!f.readonly)
                IconButton(
                  icon: _saving
                      ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                      : Icon(Icons.save, color: _dirty ? AetherColors.primary : null),
                  tooltip: 'Kaydet',
                  onPressed: _dirty && !_saving ? _save : null,
                ),
            ],
          ],
        ),
        body: _loadError != null
            ? Center(
                child: Padding(
                  padding: const EdgeInsets.all(AetherSpacing.xl),
                  child: Text(_loadError!,
                      textAlign: TextAlign.center,
                      style: const TextStyle(color: AetherColors.textMuted)),
                ),
              )
            : f == null
                ? const Center(child: CircularProgressIndicator())
                : Column(children: [
                    if (f.readonly)
                      const MaterialBanner(
                        content: Text('Dosya salt okunur.'),
                        actions: [SizedBox.shrink()],
                      ),
                    Expanded(
                      child: TextField(
                        controller: _ctrl,
                        readOnly: f.readonly,
                        maxLines: null,
                        expands: true,
                        keyboardType: TextInputType.multiline,
                        textAlignVertical: TextAlignVertical.top,
                        style: const TextStyle(fontFamily: 'monospace', fontSize: 13, color: AetherColors.text),
                        decoration: const InputDecoration(
                          border: InputBorder.none,
                          contentPadding: EdgeInsets.all(AetherSpacing.lg),
                          hintText: 'Dosya boş. Yazmaya başla…',
                        ),
                      ),
                    ),
                    Container(
                      width: double.infinity,
                      color: AetherColors.surface,
                      padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.lg, vertical: 6),
                      child: Text(
                        '${widget.path} · ${formatSize(utf8.encode(_ctrl.text).length)}${_dirty ? ' · kaydedilmedi' : ''}',
                        style: const TextStyle(color: AetherColors.textSubtle, fontSize: 11),
                      ),
                    ),
                  ]),
      ),
    );
  }
}
