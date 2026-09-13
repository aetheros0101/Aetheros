import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:archive/archive.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../api/aetheros_api.dart';
import '../core/app_error.dart';
import '../core/app_theme.dart';

const _backupVersion = 2;
const _kWasmModules = 'aetheros_wasm_modules';
const _kWat = 'script_editor_wat';
const _kJson = 'script_editor_json';

/// Portable AetherOS backup. Version 2 is a ZIP container with a JSON manifest
/// and actual cached WASM binaries. API keys are deliberately excluded.
class BackupScreen extends StatefulWidget {
  const BackupScreen({super.key});
  @override
  State<BackupScreen> createState() => _BackupScreenState();
}

class _BackupScreenState extends State<BackupScreen> {
  bool _busy = false;
  String? _message;
  bool _error = false;

  Future<Directory> _wasmDir() async {
    final root = await getApplicationSupportDirectory();
    final dir = Directory('${root.path}/wasm');
    if (!await dir.exists()) await dir.create(recursive: true);
    return dir;
  }

  Future<void> _export() async {
    setState(() { _busy = true; _message = null; });
    try {
      final prefs = await SharedPreferences.getInstance();
      final rawModules = prefs.getStringList(_kWasmModules) ?? const [];
      final dir = await _wasmDir();
      final archive = Archive();
      final modules = <Map<String, dynamic>>[];

      for (final raw in rawModules) {
        final decoded = jsonDecode(raw);
        if (decoded is! Map) continue;
        final hash = decoded['hash']?.toString() ?? '';
        if (hash.isEmpty) continue;
        final binary = File('${dir.path}/$hash.wasm');
        final hasBinary = await binary.exists();
        final path = 'wasm/$hash.wasm';
        if (hasBinary) {
          final bytes = await binary.readAsBytes();
          archive.addFile(ArchiveFile(path, bytes.length, bytes));
        }
        modules.add({
          ...Map<String, dynamic>.from(decoded),
          'binary_path': hasBinary ? path : null,
          'binary_available': hasBinary,
        });
      }

      final manifest = <String, dynamic>{
        'aetheros_backup_version': _backupVersion,
        'format': 'aetheros-zip',
        'exported_at': DateTime.now().toUtc().toIso8601String(),
        'wasm_modules': modules,
        'script_editor': {
          'wat': prefs.getString(_kWat) ?? '',
          'json': prefs.getString(_kJson) ?? '',
        },
      };
      final manifestBytes = utf8.encode(const JsonEncoder.withIndent('  ').convert(manifest));
      archive.addFile(ArchiveFile('manifest.json', manifestBytes.length, manifestBytes));
      final bytes = ZipEncoder().encode(archive);
      if (bytes == null || bytes.isEmpty) throw StateError('Yedek arşivi oluşturulamadı.');

      final ts = DateTime.now();
      final name = 'aetheros_backup_${ts.year}${_p(ts.month)}${_p(ts.day)}_${_p(ts.hour)}${_p(ts.minute)}.aetheros';
      final path = await FilePicker.platform.saveFile(
        dialogTitle: 'AetherOS Yedeğini Kaydet',
        fileName: name,
        bytes: Uint8List.fromList(bytes),
      );
      if (!mounted) return;
      setState(() {
        _message = path == null ? 'Yedek oluşturuldu ancak kaydetme iptal edildi.' : 'Yedek kaydedildi: $name';
        _error = false;
      });
    } catch (e) {
      if (mounted) setState(() { _message = userFacingError(e); _error = true; });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _import() async {
    setState(() { _busy = true; _message = null; });
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['aetheros', 'json'],
        withData: true,
      );
      final bytes = result?.files.single.bytes;
      if (bytes == null) return;

      Map<String, dynamic> manifest;
      final selectedName = result!.files.single.name.toLowerCase();
      if (selectedName.endsWith('.json')) {
        // Backward-compatible import for v1 JSON backups.
        final decoded = jsonDecode(utf8.decode(bytes));
        if (decoded is! Map<String, dynamic>) throw const FormatException('Yedek kökü object olmalı.');
        manifest = decoded;
      } else {
        final archive = ZipDecoder().decodeBytes(bytes);
        final file = _findArchiveFile(archive, 'manifest.json');
        if (file == null) throw const FormatException('manifest.json bulunamadı.');
        final content = file.content;
        manifest = jsonDecode(utf8.decode(content is List<int> ? content : List<int>.from(content))) as Map<String, dynamic>;
      }

      final version = (manifest['aetheros_backup_version'] as num?)?.toInt();
      if (version == null || version < 1 || version > _backupVersion) {
        throw FormatException('Desteklenmeyen yedek sürümü: $version');
      }
      if (!mounted) return;
      final ok = await _confirmImport(manifest);
      if (ok != true) return;

      final prefs = await SharedPreferences.getInstance();
      final modules = (manifest['wasm_modules'] as List?)?.whereType<Map>().toList() ?? const <Map>[];
      final savedModules = <String>[];
      final wasmDir = await _wasmDir();
      int restoredBinaryCount = 0;

      // For ZIP v2, re-upload every available binary into the Rust ModuleStore.
      // JSON v1 retains metadata-only behavior.
      Archive? decodedArchive;
      if (!selectedName.endsWith('.json')) decodedArchive = ZipDecoder().decodeBytes(bytes);
      for (final module in modules) {
        final map = Map<String, dynamic>.from(module);
        final path = map['binary_path']?.toString();
        if (decodedArchive != null && path != null) {
          final file = _findArchiveFile(decodedArchive, path);
          if (file != null) {
            final content = file.content;
            final wasm = content is List<int> ? content : List<int>.from(content);
            final uploaded = await AetherApi.uploadWasmModule(bytes: wasm);
            map['hash'] = uploaded.hash;
            map['size'] = uploaded.size;
            await File('${wasmDir.path}/${uploaded.hash}.wasm').writeAsBytes(wasm, flush: true);
            restoredBinaryCount++;
          }
        }
        map.remove('binary_path');
        map.remove('binary_available');
        savedModules.add(jsonEncode(map));
      }
      await prefs.setStringList(_kWasmModules, savedModules);

      final editor = manifest['script_editor'];
      if (editor is Map) {
        await prefs.setString(_kWat, editor['wat']?.toString() ?? '');
        await prefs.setString(_kJson, editor['json']?.toString() ?? '');
      }

      if (!mounted) return;
      setState(() {
        _message = 'Geri yükleme tamamlandı. ${savedModules.length} modül kaydı, $restoredBinaryCount WASM binary ve editör verileri geri yüklendi. API anahtarları güvenlik nedeniyle aktarılmadı.';
        _error = false;
      });
    } on FormatException catch (e) {
      if (mounted) setState(() { _message = e.message; _error = true; });
    } catch (e) {
      if (mounted) setState(() { _message = userFacingError(e); _error = true; });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<bool?> _confirmImport(Map<String, dynamic> data) {
    final modules = data['wasm_modules'] as List? ?? const [];
    return showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        title: const Text('Geri Yükleme Onayı'),
        content: Text('${modules.length} WASM kaydı ve editör verileri mevcut verilerin üzerine yazılacak. Devam edilsin mi?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('İptal')),
          FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Geri Yükle')),
        ],
      ),
    );
  }

  ArchiveFile? _findArchiveFile(Archive archive, String name) {
    for (final file in archive.files) {
      if (file.name == name) return file;
    }
    return null;
  }

  String _p(int n) => n.toString().padLeft(2, '0');

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Yedekleme')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          Card(child: Padding(padding: const EdgeInsets.all(16), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: const [
            Row(children: [Icon(Icons.shield_outlined), SizedBox(width: 8), Text('Güvenli ve taşınabilir yedek')]),
            SizedBox(height: 10),
            Text('AetherOS v2 yedeği gerçek WASM binary dosyalarını ZIP içine alır. API anahtarları hiçbir zaman yedeğe girmez.'),
          ]))),
          const SizedBox(height: 20),
          FilledButton.icon(onPressed: _busy ? null : _export, icon: const Icon(Icons.upload_file), label: Text(_busy ? 'İşleniyor...' : 'Yedek Oluştur (.aetheros)')),
          const SizedBox(height: 12),
          OutlinedButton.icon(onPressed: _busy ? null : _import, icon: const Icon(Icons.download), label: const Text('Yedekten Geri Yükle')),
          if (_message != null) ...[
            const SizedBox(height: 16),
            SelectableText(_message!, style: TextStyle(color: _error ? Theme.of(context).colorScheme.error : Colors.greenAccent)),
          ],
        ],
      ),
    );
  }
}
