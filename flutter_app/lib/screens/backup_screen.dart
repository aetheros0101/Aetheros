// ============================================================
// flutter_app/lib/screens/backup_screen.dart
// Sprint 5 — Yedekleme / Geri Yükleme (Export/Import JSON)
//
// Kapsam: WASM modül listesi, Script Editör içeriği (WAT+JSON),
// AI ayarları (model seçimi — API key HARİÇ, güvenlik).
// ============================================================

import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_picker/file_picker.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

const _backupVersion = 1;

// SharedPreferences anahtarları (diğer ekranlarla aynı olmalı)
const _kWasmModules = 'aetheros_wasm_modules';
const _kWat         = 'script_editor_wat';
const _kJson        = 'script_editor_json';
const _kAiModel     = 'gemini_model';
// _kAiKey HARİÇ — API key yedeğe dahil edilmez (güvenlik)

class BackupScreen extends StatefulWidget {
  const BackupScreen({super.key});
  @override
  State<BackupScreen> createState() => _BackupScreenState();
}

class _BackupScreenState extends State<BackupScreen> {
  bool _exporting = false;
  bool _importing = false;
  String? _lastExportPath;
  String? _message;
  bool _messageIsError = false;

  // ── Export ────────────────────────────────────────────

  Future<void> _export() async {
    setState(() { _exporting = true; _message = null; });
    try {
      final p = await SharedPreferences.getInstance();

      final data = <String, dynamic>{
        'aetheros_backup_version': _backupVersion,
        'exported_at': DateTime.now().toIso8601String(),
        'wasm_modules': p.getStringList(_kWasmModules) ?? [],
        'script_editor': {
          'wat':  p.getString(_kWat)  ?? '',
          'json': p.getString(_kJson) ?? '',
        },
        'ai_settings': {
          'model': p.getString(_kAiModel) ?? '',
          // api_key bilerek dahil edilmedi
        },
      };

      final jsonStr = const JsonEncoder.withIndent('  ').convert(data);

      final dir = await getTemporaryDirectory();
      final ts  = DateTime.now();
      final fname =
          'aetheros_backup_${ts.year}${_p(ts.month)}${_p(ts.day)}_'
          '${_p(ts.hour)}${_p(ts.minute)}.json';
      final file = File('${dir.path}/$fname');
      await file.writeAsString(jsonStr);

      // Kullanıcının seçtiği konuma kaydet
      final savePath = await FilePicker.platform.saveFile(
        dialogTitle: 'Yedeği Kaydet',
        fileName: fname,
        bytes: utf8.encode(jsonStr),
      );

      setState(() {
        _lastExportPath = savePath ?? file.path;
        _message = '✅ Yedek oluşturuldu: $fname';
        _messageIsError = false;
      });
    } catch (e) {
      setState(() {
        _message = '❌ Export hatası: $e';
        _messageIsError = true;
      });
    } finally {
      setState(() => _exporting = false);
    }
  }

  // ── Import ────────────────────────────────────────────

  Future<void> _import() async {
    setState(() { _importing = true; _message = null; });
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['json'],
        withData: true,
      );

      if (result == null || result.files.single.bytes == null) {
        setState(() => _importing = false);
        return;
      }

      final content = utf8.decode(result.files.single.bytes!);
      final data = jsonDecode(content) as Map<String, dynamic>;

      final version = data['aetheros_backup_version'] as int?;
      if (version == null) {
        throw const FormatException('Geçersiz yedek dosyası');
      }

      final confirmed = await _confirmImport(data);
      if (confirmed != true) {
        setState(() => _importing = false);
        return;
      }

      final p = await SharedPreferences.getInstance();

      // WASM modülleri
      final modules = (data['wasm_modules'] as List?)
          ?.map((e) => e.toString())
          .toList();
      if (modules != null) {
        await p.setStringList(_kWasmModules, modules);
      }

      // Script editör
      final scriptEditor = data['script_editor'] as Map<String, dynamic>?;
      if (scriptEditor != null) {
        final wat  = scriptEditor['wat']  as String?;
        final json = scriptEditor['json'] as String?;
        if (wat  != null && wat.isNotEmpty)  await p.setString(_kWat,  wat);
        if (json != null && json.isNotEmpty) await p.setString(_kJson, json);
      }

      // AI ayarları (sadece model, key hariç)
      final aiSettings = data['ai_settings'] as Map<String, dynamic>?;
      if (aiSettings != null) {
        final model = aiSettings['model'] as String?;
        if (model != null && model.isNotEmpty) {
          await p.setString(_kAiModel, model);
        }
      }

      setState(() {
        _message = '✅ Geri yükleme tamamlandı. '
            '${modules?.length ?? 0} WASM modülü, script ve AI ayarları geri yüklendi.';
        _messageIsError = false;
      });
    } on FormatException catch (e) {
      setState(() {
        _message = '❌ Geçersiz dosya: ${e.message}';
        _messageIsError = true;
      });
    } catch (e) {
      setState(() {
        _message = '❌ Import hatası: $e';
        _messageIsError = true;
      });
    } finally {
      setState(() => _importing = false);
    }
  }

  Future<bool?> _confirmImport(Map<String, dynamic> data) {
    final exportedAt = data['exported_at'] as String?;
    final moduleCount = (data['wasm_modules'] as List?)?.length ?? 0;
    final hasWat  = ((data['script_editor']?['wat']  as String?) ?? '').isNotEmpty;
    final hasJson = ((data['script_editor']?['json'] as String?) ?? '').isNotEmpty;

    return showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF1A1A2E),
        title: const Text('Geri Yükleme Onayı',
            style: TextStyle(color: Colors.white, fontSize: 16)),
        content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
          if (exportedAt != null)
            Text('Yedek tarihi: ${_fmtDate(exportedAt)}',
                style: const TextStyle(color: Colors.white60, fontSize: 12)),
          const SizedBox(height: 10),
          Text('• $moduleCount WASM modülü',
              style: const TextStyle(color: Colors.white70, fontSize: 13)),
          Text('• WAT kodu: ${hasWat ? "Var" : "Yok"}',
              style: const TextStyle(color: Colors.white70, fontSize: 13)),
          Text('• Workflow JSON: ${hasJson ? "Var" : "Yok"}',
              style: const TextStyle(color: Colors.white70, fontSize: 13)),
          const SizedBox(height: 12),
          const Text(
            'Mevcut veriler bu yedekle değiştirilecek. Devam edilsin mi?',
            style: TextStyle(color: Color(0xFFFFB74D), fontSize: 12),
          ),
        ]),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('İptal',
                style: TextStyle(color: Colors.white54)),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Geri Yükle',
                style: TextStyle(color: Color(0xFF6C63FF))),
          ),
        ],
      ),
    );
  }

  String _fmtDate(String iso) {
    try {
      final d = DateTime.parse(iso).toLocal();
      return '${_p(d.day)}.${_p(d.month)}.${d.year}  ${_p(d.hour)}:${_p(d.minute)}';
    } catch (_) {
      return iso;
    }
  }

  String _p(int n) => n.toString().padLeft(2, '0');

  // ── UI ────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: const Text('Yedekleme', style: TextStyle(color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white70),
      ),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          // ── Bilgi ────────────────────────────────────
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: const Color(0xFF6C63FF).withOpacity(0.08),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                  color: const Color(0xFF6C63FF).withOpacity(0.25)),
            ),
            child: Column(
                crossAxisAlignment: CrossAxisAlignment.start, children: [
              const Row(children: [
                Icon(Icons.shield_outlined,
                    color: Color(0xFF6C63FF), size: 16),
                SizedBox(width: 8),
                Text('Yedek İçeriği',
                    style: TextStyle(
                        color: Color(0xFF6C63FF),
                        fontWeight: FontWeight.bold, fontSize: 13)),
              ]),
              const SizedBox(height: 8),
              const Text(
                '• WASM modül kayıtları (hash + isim)\n'
                '• Script Editör (WAT + Workflow JSON)\n'
                '• AI model tercihi\n\n'
                '⚠️ Gemini API anahtarı GÜVENLİK nedeniyle '
                'yedeğe dahil edilmez — paylaşılan yedek dosyasında '
                'API anahtarınız bulunmaz.',
                style: TextStyle(color: Colors.white60, fontSize: 12, height: 1.6),
              ),
            ]),
          ),
          const SizedBox(height: 28),

          // ── Export ───────────────────────────────────
          _label('Yedek Oluştur'),
          const SizedBox(height: 6),
          const Text(
            'Tüm verilerini bir JSON dosyasına aktar. '
            'Bu dosyayı başka bir cihaza taşıyabilir veya yedek olarak saklayabilirsin.',
            style: TextStyle(color: Colors.white38, fontSize: 12),
          ),
          const SizedBox(height: 12),
          SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: const Color(0xFF6C63FF),
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: _exporting ? null : _export,
              icon: _exporting
                  ? const SizedBox(width: 16, height: 16,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: Colors.white))
                  : const Icon(Icons.upload, size: 18),
              label: Text(_exporting ? 'Oluşturuluyor...' : 'Yedek Oluştur ve Kaydet'),
            ),
          ),
          const SizedBox(height: 32),

          Divider(color: Colors.white12),
          const SizedBox(height: 24),

          // ── Import ───────────────────────────────────
          _label('Yedekten Geri Yükle'),
          const SizedBox(height: 6),
          const Text(
            'Önceden oluşturduğun bir .json yedek dosyasını seç. '
            'Mevcut WASM modülleri, script ve AI tercihleri değiştirilecek.',
            style: TextStyle(color: Colors.white38, fontSize: 12),
          ),
          const SizedBox(height: 12),
          SizedBox(
            width: double.infinity,
            child: OutlinedButton.icon(
              style: OutlinedButton.styleFrom(
                foregroundColor: const Color(0xFF6C63FF),
                side: const BorderSide(color: Color(0xFF6C63FF)),
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: _importing ? null : _import,
              icon: _importing
                  ? const SizedBox(width: 16, height: 16,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: Color(0xFF6C63FF)))
                  : const Icon(Icons.download, size: 18),
              label: Text(_importing ? 'Yükleniyor...' : 'Yedek Dosyası Seç'),
            ),
          ),

          // ── Sonuç mesajı ─────────────────────────────
          if (_message != null) ...[
            const SizedBox(height: 16),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: (_messageIsError
                        ? const Color(0xFFEF5350)
                        : const Color(0xFF4CAF50))
                    .withOpacity(0.08),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                  color: (_messageIsError
                          ? const Color(0xFFEF5350)
                          : const Color(0xFF4CAF50))
                      .withOpacity(0.3),
                ),
              ),
              child: Row(children: [
                Expanded(
                  child: Text(_message!,
                      style: TextStyle(
                        color: _messageIsError
                            ? const Color(0xFFEF5350)
                            : const Color(0xFF4CAF50),
                        fontSize: 12,
                      )),
                ),
                if (_lastExportPath != null && !_messageIsError)
                  GestureDetector(
                    onTap: () {
                      Clipboard.setData(
                          ClipboardData(text: _lastExportPath!));
                      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
                        content: Text('Yol kopyalandı'),
                        backgroundColor: Color(0xFF6C63FF),
                        duration: Duration(seconds: 1),
                      ));
                    },
                    child: const Icon(Icons.copy,
                        color: Color(0xFF4CAF50), size: 16),
                  ),
              ]),
            ),
          ],
        ],
      ),
    );
  }

  Widget _label(String t) => Text(t,
      style: const TextStyle(
          color: Colors.white,
          fontSize: 14,
          fontWeight: FontWeight.bold,
          letterSpacing: 0.5));
}
