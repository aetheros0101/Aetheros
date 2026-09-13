// ============================================================
// flutter_app/lib/screens/wasm_module_screen.dart
// Sprint 3 — WASM modül yükleyici + modül listesi
// ============================================================

import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_picker/file_picker.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:path_provider/path_provider.dart';
import '../api/aetheros_api.dart';
import 'submit_task_screen.dart';
import '../core/app_error.dart';
import '../core/app_theme.dart';

// ── Model ─────────────────────────────────────────────────

class WasmModule {
  final String hash;
  final int    size;
  final String filename;
  final int    uploadedAt; // ms epoch

  const WasmModule({
    required this.hash,
    required this.size,
    required this.filename,
    required this.uploadedAt,
  });

  Map<String, dynamic> toJson() => {
    'hash': hash, 'size': size,
    'filename': filename, 'uploadedAt': uploadedAt,
  };

  factory WasmModule.fromJson(Map<String, dynamic> j) => WasmModule(
    hash: j['hash'] as String,
    size: j['size'] as int,
    filename: j['filename'] as String,
    uploadedAt: j['uploadedAt'] as int,
  );
}

const _prefsKey = 'aetheros_wasm_modules';

// ── Ekran ─────────────────────────────────────────────────

class WasmModuleScreen extends StatefulWidget {
  const WasmModuleScreen({super.key});
  @override
  State<WasmModuleScreen> createState() => _WasmModuleScreenState();
}

class _WasmModuleScreenState extends State<WasmModuleScreen> {
  List<WasmModule> _modules = [];
  bool _uploading = false;
  String? _uploadError;

  @override
  void initState() {
    super.initState();
    _loadFromPrefs();
  }

  // ── Persistence ──────────────────────────────────────

  Future<void> _loadFromPrefs() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final raw = prefs.getStringList(_prefsKey) ?? [];
      final modules = <WasmModule>[];

      for (final rawEntry in raw) {
        try {
          final module = WasmModule.fromJson(
            jsonDecode(rawEntry) as Map<String, dynamic>,
          );
          if (await AetherApi.checkModuleExists(module.hash)) {
            modules.add(module);
          }
        } catch (_) {
          // Bozuk/eski metadata kaydı tüm ekranı düşürmesin.
        }
      }

      modules.sort((a, b) => b.uploadedAt.compareTo(a.uploadedAt));
      if (!mounted) return;
      setState(() => _modules = modules);
      await _saveToPrefs();
    } catch (e) {
      if (!mounted) return;
      setState(() => _uploadError = 'Modül listesi yüklenemedi: $e');
    }
  }

  Future<Directory> _binaryDir() async {
    final root = await getApplicationSupportDirectory();
    final dir = Directory('${root.path}/wasm');
    if (!await dir.exists()) await dir.create(recursive: true);
    return dir;
  }

  Future<File> _binaryFile(String hash) async {
    final dir = await _binaryDir();
    return File('${dir.path}/$hash.wasm');
  }

  Future<void> _saveToPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setStringList(
      _prefsKey,
      _modules.map((m) => jsonEncode(m.toJson())).toList(),
    );
  }

  // ── Yükleme ───────────────────────────────────────────

  Future<void> _pickAndUpload() async {
    setState(() { _uploading = true; _uploadError = null; });
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['wasm'],
        withData: true,
      );

      if (result == null || result.files.single.bytes == null) {
        if (mounted) setState(() => _uploading = false);
        return;
      }

      final file  = result.files.single;
      final bytes = file.bytes!.toList();

      final response = await AetherApi.uploadWasmModule(bytes: bytes);

      final module = WasmModule(
        hash:       response.hash,
        size:       response.size,
        filename:   file.name,
        uploadedAt: DateTime.now().millisecondsSinceEpoch,
      );
      // Keep a local binary cache so a backup can restore the actual WASM.
      await (await _binaryFile(module.hash)).writeAsBytes(bytes, flush: true);

      if (!mounted) return;
      setState(() {
        // Aynı hash varsa üzerine yaz
        _modules.removeWhere((m) => m.hash == module.hash);
        _modules.insert(0, module);
      });
      await _saveToPrefs();

      if (mounted) _showSuccess(module);
    } catch (e) {
      if (mounted) setState(() => _uploadError = userFacingError(e));
    } finally {
      if (mounted) setState(() => _uploading = false);
    }
  }

  Future<void> _deleteModule(WasmModule m) async {
    if (!mounted) return;
    setState(() => _modules.remove(m));
    try {
      final binary = await _binaryFile(m.hash);
      if (await binary.exists()) await binary.delete();
      await _saveToPrefs();
    } catch (e) {
      if (!mounted) return;
      setState(() => _uploadError = 'Modül listesi kaydedilemedi: $e');
    }
  }

  void _copyHash(String hash) {
    Clipboard.setData(ClipboardData(text: hash));
    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
      content: Text('Hash kopyalandı'),
      backgroundColor: AetherColors.primary,
      duration: Duration(seconds: 1),
    ));
  }

  void _showSuccess(WasmModule m) {
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(
      content: Text('✅ ${m.filename} yüklendi (${_kb(m.size)})'),
      backgroundColor: AetherColors.success,
      duration: const Duration(seconds: 3),
    ));
  }

  // ── Build ─────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AetherColors.background,
      appBar: AppBar(
        backgroundColor: AetherColors.background,
        title: const Text('WASM Modüller',
            style: TextStyle(color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          IconButton(
            icon: const Icon(Icons.help_outline, color: Colors.white38, size: 20),
            onPressed: _showHelp,
          ),
        ],
      ),
      body: Column(children: [
        // ── Yükle butonu ────────────────────────────
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          child: SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: AetherColors.primary,
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: _uploading ? null : _pickAndUpload,
              icon: _uploading
                  ? const SizedBox(width: 18, height: 18,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: Colors.white))
                  : const Icon(Icons.upload_file),
              label: Text(
                _uploading ? 'Yükleniyor...' : '.wasm Dosyası Seç',
                style: const TextStyle(
                    fontSize: 15, fontWeight: FontWeight.bold),
              ),
            ),
          ),
        ),

        // ── Hata ────────────────────────────────────
        if (_uploadError != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 10, 16, 0),
            child: Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: AetherColors.danger.withOpacity(0.08),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                    color: AetherColors.danger.withOpacity(0.3)),
              ),
              child: Text(_uploadError!,
                  style: const TextStyle(
                      color: AetherColors.danger, fontSize: 12)),
            ),
          ),

        const SizedBox(height: 12),

        // ── Modül listesi ────────────────────────────
        Expanded(
          child: _modules.isEmpty
              ? _EmptyState(onUpload: _pickAndUpload)
              : ListView.separated(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
                  itemCount: _modules.length,
                  separatorBuilder: (_, __) => const SizedBox(height: 8),
                  itemBuilder: (_, i) => _ModuleCard(
                    module: _modules[i],
                    onCopyHash: () => _copyHash(_modules[i].hash),
                    onDelete: () => _confirmDelete(_modules[i]),
                    onSubmitTask: () => _goSubmit(_modules[i]),
                  ),
                ),
        ),
      ]),
    );
  }

  void _goSubmit(WasmModule m) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => SubmitTaskScreen(prefilledHash: m.hash),
      ),
    );
  }

  Future<void> _confirmDelete(WasmModule m) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: AetherColors.surface,
        title: const Text('Modülü Sil',
            style: TextStyle(color: Colors.white)),
        content: Text(
          '${m.filename} silinsin mi? Bu işlem geri alınamaz.',
          style: const TextStyle(color: Colors.white60),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('İptal',
                  style: TextStyle(color: Colors.white54))),
          TextButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Sil',
                  style: TextStyle(color: AetherColors.danger))),
        ],
      ),
    );
    if (ok == true) _deleteModule(m);
  }

  void _showHelp() {
    showModalBottomSheet(
      context: context,
      backgroundColor: AetherColors.surface,
      shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(20))),
      builder: (_) => Padding(
        padding: const EdgeInsets.all(24),
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          Container(width: 40, height: 4,
              decoration: BoxDecoration(
                  color: Colors.white24,
                  borderRadius: BorderRadius.circular(2))),
          const SizedBox(height: 20),
          const Text('WASM Modül Kullanımı',
              style: TextStyle(color: Colors.white,
                  fontSize: 16, fontWeight: FontWeight.bold)),
          const SizedBox(height: 16),
          ...[
            ('1', 'Bir .wasm dosyası seç ve yükle'),
            ('2', 'Modül hash\'ini kopyala veya "Task Gönder" butonuna bas'),
            ('3', 'Task Gönder ekranında hash otomatik dolar'),
            ('4', 'Entrypoint olarak WASM modülündeki export adını yaz'),
          ].map((item) => Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Container(
                width: 22, height: 22,
                decoration: BoxDecoration(
                  color: AetherColors.primary.withOpacity(0.2),
                  shape: BoxShape.circle,
                ),
                child: Center(child: Text(item.$1,
                    style: const TextStyle(
                        color: AetherColors.primary, fontSize: 11,
                        fontWeight: FontWeight.bold))),
              ),
              const SizedBox(width: 10),
              Expanded(child: Text(item.$2,
                  style: const TextStyle(color: Colors.white70, fontSize: 13))),
            ]),
          )),
          const SizedBox(height: 8),
        ]),
      ),
    );
  }

  String _kb(int bytes) {
    if (bytes < 1024) return '${bytes}B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)}KB';
    return '${(bytes / 1024 / 1024).toStringAsFixed(2)}MB';
  }
}

// ── Modül kartı ───────────────────────────────────────────

class _ModuleCard extends StatelessWidget {
  final WasmModule module;
  final VoidCallback onCopyHash;
  final VoidCallback onDelete;
  final VoidCallback onSubmitTask;

  const _ModuleCard({
    required this.module,
    required this.onCopyHash,
    required this.onDelete,
    required this.onSubmitTask,
  });

  @override
  Widget build(BuildContext context) {
    final dt =
        DateTime.fromMillisecondsSinceEpoch(module.uploadedAt).toLocal();
    final dateStr =
        '${dt.day.toString().padLeft(2,'0')}.${dt.month.toString().padLeft(2,'0')}.${dt.year}  '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}'

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: AetherColors.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
            color: AetherColors.primary.withOpacity(0.2)),
      ),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        // Başlık satırı
        Row(children: [
          Container(
            width: 36, height: 36,
            decoration: BoxDecoration(
              color: AetherColors.primary.withOpacity(0.12),
              borderRadius: BorderRadius.circular(8),
            ),
            child: const Icon(Icons.memory,
                color: AetherColors.primary, size: 18),
          ),
          const SizedBox(width: 10),
          Expanded(child: Column(
              crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(module.filename,
                style: const TextStyle(
                    color: Colors.white, fontWeight: FontWeight.bold,
                    fontSize: 13),
                maxLines: 1, overflow: TextOverflow.ellipsis),
            Text(dateStr,
                style: const TextStyle(color: Colors.white38, fontSize: 11)),
          ])),
          // Sil
          InkWell(
            onTap: onDelete,
            child: const Padding(
              padding: EdgeInsets.all(4),
              child: Icon(Icons.delete_outline,
                  color: Colors.white24, size: 18),
            ),
          ),
        ]),
        const SizedBox(height: 10),

        // Hash kutusu
        InkWell(
          onTap: onCopyHash,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            decoration: BoxDecoration(
              color: Colors.black26,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.white10),
            ),
            child: Row(children: [
              Expanded(
                child: Text(
                  module.hash,
                  style: const TextStyle(
                      color: Color(0xFF80CBC4),
                      fontFamily: 'monospace',
                      fontSize: 11,
                      letterSpacing: 0.3),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 6),
              const Icon(Icons.copy,
                  color: AetherColors.primary, size: 14),
            ]),
          ),
        ),
        const SizedBox(height: 10),

        // Alt bilgi + task butonu
        Row(children: [
          _Chip(label: _sizeStr(module.size), icon: Icons.data_usage),
          const Spacer(),
          InkWell(
            onTap: onSubmitTask,
            child: Container(
              padding: const EdgeInsets.symmetric(
                  horizontal: 12, vertical: 6),
              decoration: BoxDecoration(
                color: AetherColors.primary.withOpacity(0.15),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                    color: AetherColors.primary.withOpacity(0.4)),
              ),
              child: const Row(children: [
                Icon(Icons.send, color: AetherColors.primary, size: 13),
                SizedBox(width: 5),
                Text('Task Gönder',
                    style: TextStyle(
                        color: AetherColors.primary,
                        fontSize: 11,
                        fontWeight: FontWeight.bold)),
              ]),
            ),
          ),
        ]),
      ]),
    );
  }

  String _sizeStr(int b) {
    if (b < 1024) return '${b}B';
    if (b < 1024 * 1024) return '${(b / 1024).toStringAsFixed(1)}KB';
    return '${(b / 1024 / 1024).toStringAsFixed(2)}MB';
  }
}

class _Chip extends StatelessWidget {
  final String label;
  final IconData icon;
  const _Chip({required this.label, required this.icon});
  @override
  Widget build(BuildContext context) => Row(children: [
    Icon(icon, color: Colors.white38, size: 12),
    const SizedBox(width: 4),
    Text(label,
        style: const TextStyle(color: Colors.white38, fontSize: 11)),
  ]);
}

// ── Boş durum ─────────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  final VoidCallback onUpload;
  const _EmptyState({required this.onUpload});
  @override
  Widget build(BuildContext context) => Center(
    child: Column(mainAxisSize: MainAxisSize.min, children: [
      const Icon(Icons.cloud_upload,
          color: Colors.white12, size: 56),
      const SizedBox(height: 14),
      const Text('WASM modül yüklenmedi',
          style: TextStyle(color: Colors.white38, fontSize: 14)),
      const SizedBox(height: 6),
      const Text('.wasm dosyasını yükleyerek başla',
          style: TextStyle(color: Colors.white24, fontSize: 12)),
      const SizedBox(height: 20),
      OutlinedButton.icon(
        style: OutlinedButton.styleFrom(
          foregroundColor: AetherColors.primary,
          side: const BorderSide(color: AetherColors.primary),
          shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(10)),
        ),
        onPressed: onUpload,
        icon: const Icon(Icons.upload_file, size: 16),
        label: const Text('Dosya Seç'),
      ),
    ]),
  );
}
