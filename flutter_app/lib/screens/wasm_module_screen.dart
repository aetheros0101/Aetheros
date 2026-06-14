// ============================================================
// flutter_app/lib/screens/wasm_module_screen.dart
// Fix #2 — Health check + modül durum badge'leri
//
// YENİ:
//   - Ekran açılışında her modül için checkModuleExists() çağrılır
//   - Runtime'da hazır olmayan modüller kırmızı "Hazır Değil"
//     badge'i ile işaretlenir
//   - Eksik modüller için "Yeniden Yükle" aksiyonu
//   - "Task Gönder" butonu hazır olmayan modülde devre dışı +
//     açıklayıcı tooltip
// ============================================================

import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_picker/file_picker.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../api/aetheros_api.dart';
import 'submit_task_screen.dart';

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
    hash:       j['hash']       as String,
    size:       j['size']       as int,
    filename:   j['filename']   as String,
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
  List<WasmModule> _modules    = [];
  Set<String>      _missing    = {}; // runtime'da hazır olmayan hash'ler
  bool  _uploading             = false;
  bool  _checking              = false; // health check devam ediyor
  String? _uploadError;

  @override
  void initState() {
    super.initState();
    _loadFromPrefs().then((_) => _checkHealth());
  }

  // ── SharedPreferences ─────────────────────────────────

  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final raw   = prefs.getStringList(_prefsKey) ?? [];
    setState(() {
      _modules = raw
          .map((s) => WasmModule.fromJson(jsonDecode(s) as Map<String, dynamic>))
          .toList()
        ..sort((a, b) => b.uploadedAt.compareTo(a.uploadedAt));
    });
  }

  Future<void> _saveToPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setStringList(
      _prefsKey,
      _modules.map((m) => jsonEncode(m.toJson())).toList(),
    );
  }

  // ── Health check ──────────────────────────────────────
  //
  // Her modül için Rust runtime'a "bu hash bellekte var mı?" sorusu.
  // Fix #1 ile startup'ta sled'den yükleniyor; bu check,
  // Fix #1 öncesi yüklenmiş eski modülleri ya da edge case'leri yakalar.

  Future<void> _checkHealth() async {
    if (_modules.isEmpty) return;
    setState(() => _checking = true);

    final missing = <String>{};
    for (final m in _modules) {
      try {
        final ok = await AetherApi.checkModuleExists(m.hash);
        if (!ok) missing.add(m.hash);
      } catch (_) {
        // Runtime hazır değilse check atla
      }
    }

    if (mounted) setState(() { _missing = missing; _checking = false; });
  }

  // ── Yükleme ───────────────────────────────────────────

  Future<void> _pickAndUpload({String? replaceHash}) async {
    setState(() { _uploading = true; _uploadError = null; });
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['wasm'],
        withData: true,
      );

      if (result == null || result.files.single.bytes == null) {
        setState(() => _uploading = false);
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

      setState(() {
        // Eski kaydı (hash veya replaceHash) temizle
        _modules.removeWhere(
            (m) => m.hash == module.hash || m.hash == replaceHash);
        _modules.insert(0, module);
        _missing.remove(module.hash);
        _missing.remove(replaceHash);
      });
      await _saveToPrefs();

      if (mounted) _showSuccess(module);
    } catch (e) {
      setState(() => _uploadError = e.toString());
    } finally {
      setState(() => _uploading = false);
    }
  }

  Future<void> _deleteModule(WasmModule m) async {
    setState(() {
      _modules.remove(m);
      _missing.remove(m.hash);
    });
    await _saveToPrefs();
  }

  void _copyHash(String hash) {
    Clipboard.setData(ClipboardData(text: hash));
    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
      content: Text('Hash kopyalandı'),
      backgroundColor: Color(0xFF6C63FF),
      duration: Duration(seconds: 1),
    ));
  }

  void _showSuccess(WasmModule m) {
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(
      content: Text('✅ ${m.filename} yüklendi (${_kb(m.size)})'),
      backgroundColor: const Color(0xFF4CAF50),
      duration: const Duration(seconds: 3),
    ));
  }

  // ── Build ──────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    final missingCount = _missing.length;

    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: Row(children: [
          const Text('WASM Modüller',
              style: TextStyle(color: Colors.white)),
          if (_checking) ...[ // health check döner animasyonu
            const SizedBox(width: 8),
            const SizedBox(width: 12, height: 12,
              child: CircularProgressIndicator(
                  strokeWidth: 1.5, color: Colors.white38)),
          ],
        ]),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          // Health check yenile
          IconButton(
            icon: const Icon(Icons.health_and_safety_outlined,
                color: Colors.white38, size: 20),
            tooltip: 'Modül durumlarını kontrol et',
            onPressed: _checking ? null : _checkHealth,
          ),
          IconButton(
            icon: const Icon(Icons.help_outline,
                color: Colors.white38, size: 20),
            onPressed: _showHelp,
          ),
        ],
      ),
      body: Column(children: [

        // ── Hazır-değil uyarı banner'ı ──────────────────
        if (missingCount > 0)
          _MissingBanner(
            count: missingCount,
            onDismiss: () => setState(() => _missing.clear()),
          ),

        // ── Yükle butonu ────────────────────────────────
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          child: SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: const Color(0xFF6C63FF),
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: _uploading ? null : () => _pickAndUpload(),
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

        // ── Upload hatası ────────────────────────────────
        if (_uploadError != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 10, 16, 0),
            child: Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: const Color(0xFFEF5350).withOpacity(0.08),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                    color: const Color(0xFFEF5350).withOpacity(0.3)),
              ),
              child: Text(_uploadError!,
                  style: const TextStyle(
                      color: Color(0xFFEF5350), fontSize: 12)),
            ),
          ),

        const SizedBox(height: 12),

        // ── Modül listesi ────────────────────────────────
        Expanded(
          child: _modules.isEmpty
              ? _EmptyState(onUpload: _pickAndUpload)
              : ListView.separated(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
                  itemCount: _modules.length,
                  separatorBuilder: (_, __) => const SizedBox(height: 8),
                  itemBuilder: (_, i) {
                    final m = _modules[i];
                    final ready = !_missing.contains(m.hash);
                    return _ModuleCard(
                      module:      m,
                      isReady:     ready,
                      onCopyHash:  () => _copyHash(m.hash),
                      onDelete:    () => _confirmDelete(m),
                      onSubmitTask: ready
                          ? () => _goSubmit(m)
                          : null, // devre dışı
                      onReupload:  ready
                          ? null
                          : () => _pickAndUpload(replaceHash: m.hash),
                    );
                  },
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
        backgroundColor: const Color(0xFF1A1A2E),
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
                  style: TextStyle(color: Color(0xFFEF5350)))),
        ],
      ),
    );
    if (ok == true) _deleteModule(m);
  }

  void _showHelp() {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1A1A2E),
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
            ('⚠', '"Hazır Değil" badge\'i görürsen dosyayı tekrar yükle'),
          ].map((item) => Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Container(
                width: 22, height: 22,
                decoration: BoxDecoration(
                  color: const Color(0xFF6C63FF).withOpacity(0.2),
                  shape: BoxShape.circle,
                ),
                child: Center(child: Text(item.$1,
                    style: const TextStyle(
                        color: Color(0xFF6C63FF), fontSize: 11,
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

// ── Eksik modül banner'ı ──────────────────────────────────

class _MissingBanner extends StatelessWidget {
  final int count;
  final VoidCallback onDismiss;
  const _MissingBanner({required this.count, required this.onDismiss});

  @override
  Widget build(BuildContext context) => Container(
    margin: const EdgeInsets.fromLTRB(16, 12, 16, 0),
    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
    decoration: BoxDecoration(
      color: const Color(0xFFFF6F00).withOpacity(0.1),
      borderRadius: BorderRadius.circular(10),
      border: Border.all(color: const Color(0xFFFF6F00).withOpacity(0.35)),
    ),
    child: Row(children: [
      const Icon(Icons.warning_amber_rounded,
          color: Color(0xFFFF6F00), size: 18),
      const SizedBox(width: 10),
      Expanded(child: Text(
        '$count modül runtime\'da hazır değil. '
        'Dosyayı tekrar yüklemek için "Yeniden Yükle" butonuna bas.',
        style: const TextStyle(color: Color(0xFFFF6F00), fontSize: 12),
      )),
      GestureDetector(
        onTap: onDismiss,
        child: const Padding(
          padding: EdgeInsets.only(left: 8),
          child: Icon(Icons.close, color: Color(0xFFFF6F00), size: 16),
        ),
      ),
    ]),
  );
}

// ── Modül kartı ───────────────────────────────────────────

class _ModuleCard extends StatelessWidget {
  final WasmModule   module;
  final bool         isReady;
  final VoidCallback onCopyHash;
  final VoidCallback onDelete;
  final VoidCallback? onSubmitTask; // null → hazır değil
  final VoidCallback? onReupload;   // null → hazır

  const _ModuleCard({
    required this.module,
    required this.isReady,
    required this.onCopyHash,
    required this.onDelete,
    this.onSubmitTask,
    this.onReupload,
  });

  @override
  Widget build(BuildContext context) {
    final dt = DateTime.fromMillisecondsSinceEpoch(module.uploadedAt).toLocal();
    final dateStr =
        '${dt.day.toString().padLeft(2,'0')}.${dt.month.toString().padLeft(2,'0')}.${dt.year}  '
        '${dt.hour.toString().padLeft(2,'0')}:${dt.minute.toString().padLeft(2,'0')}';

    final borderColor = isReady
        ? const Color(0xFF6C63FF).withOpacity(0.2)
        : const Color(0xFFEF5350).withOpacity(0.35);

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFF1A1A2E),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: borderColor),
      ),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [

        // ── Başlık ────────────────────────────────────────
        Row(children: [
          Container(
            width: 36, height: 36,
            decoration: BoxDecoration(
              color: (isReady
                  ? const Color(0xFF6C63FF)
                  : const Color(0xFFEF5350)).withOpacity(0.12),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(
              isReady ? Icons.memory : Icons.memory_outlined,
              color: isReady
                  ? const Color(0xFF6C63FF)
                  : const Color(0xFFEF5350),
              size: 18,
            ),
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

          // Durum badge'i
          if (!isReady)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
              decoration: BoxDecoration(
                color: const Color(0xFFEF5350).withOpacity(0.12),
                borderRadius: BorderRadius.circular(6),
                border: Border.all(
                    color: const Color(0xFFEF5350).withOpacity(0.3)),
              ),
              child: const Text('Hazır Değil',
                  style: TextStyle(
                      color: Color(0xFFEF5350),
                      fontSize: 10, fontWeight: FontWeight.bold)),
            ),
          if (isReady)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
              decoration: BoxDecoration(
                color: const Color(0xFF4CAF50).withOpacity(0.1),
                borderRadius: BorderRadius.circular(6),
                border: Border.all(
                    color: const Color(0xFF4CAF50).withOpacity(0.25)),
              ),
              child: const Text('Hazır',
                  style: TextStyle(
                      color: Color(0xFF4CAF50),
                      fontSize: 10, fontWeight: FontWeight.bold)),
            ),

          const SizedBox(width: 6),
          GestureDetector(
            onTap: onDelete,
            child: const Padding(
              padding: EdgeInsets.all(4),
              child: Icon(Icons.delete_outline,
                  color: Colors.white24, size: 18),
            ),
          ),
        ]),
        const SizedBox(height: 10),

        // ── Hash kutusu ────────────────────────────────────
        GestureDetector(
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
                      fontSize: 11, letterSpacing: 0.3),
                  maxLines: 1, overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 6),
              const Icon(Icons.copy, color: Color(0xFF6C63FF), size: 14),
            ]),
          ),
        ),
        const SizedBox(height: 10),

        // ── Alt bilgi + aksiyonlar ─────────────────────────
        Row(children: [
          _Chip(label: _sizeStr(module.size), icon: Icons.data_usage),
          const Spacer(),

          // Hazır değilse "Yeniden Yükle", hazırsa "Task Gönder"
          if (!isReady && onReupload != null)
            _ActionButton(
              label: 'Yeniden Yükle',
              icon: Icons.upload_file,
              color: const Color(0xFFFF6F00),
              onTap: onReupload!,
            )
          else if (isReady && onSubmitTask != null)
            _ActionButton(
              label: 'Task Gönder',
              icon: Icons.send,
              color: const Color(0xFF6C63FF),
              onTap: onSubmitTask!,
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

class _ActionButton extends StatelessWidget {
  final String label;
  final IconData icon;
  final Color color;
  final VoidCallback onTap;
  const _ActionButton({
    required this.label,
    required this.icon,
    required this.color,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) => GestureDetector(
    onTap: onTap,
    child: Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: color.withOpacity(0.4)),
      ),
      child: Row(children: [
        Icon(icon, color: color, size: 13),
        const SizedBox(width: 5),
        Text(label,
            style: TextStyle(
                color: color, fontSize: 11, fontWeight: FontWeight.bold)),
      ]),
    ),
  );
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
  final Function() onUpload;
  const _EmptyState({required this.onUpload});
  @override
  Widget build(BuildContext context) => Center(
    child: Column(mainAxisSize: MainAxisSize.min, children: [
      const Icon(Icons.cloud_upload, color: Colors.white12, size: 56),
      const SizedBox(height: 14),
      const Text('WASM modül yüklenmedi',
          style: TextStyle(color: Colors.white38, fontSize: 14)),
      const SizedBox(height: 6),
      const Text('.wasm dosyasını yükleyerek başla',
          style: TextStyle(color: Colors.white24, fontSize: 12)),
      const SizedBox(height: 20),
      OutlinedButton.icon(
        style: OutlinedButton.styleFrom(
          foregroundColor: const Color(0xFF6C63FF),
          side: const BorderSide(color: Color(0xFF6C63FF)),
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
