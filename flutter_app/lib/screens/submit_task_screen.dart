// ============================================================
// flutter_app/lib/screens/submit_task_screen.dart
// Sprint 2: ikon fix, başarı sonrası "Task Listesine Git"
// ============================================================

import 'package:flutter/material.dart';
import '../api/aetheros_api.dart';
import 'task_list_screen.dart';

class SubmitTaskScreen extends StatefulWidget {
  final String? prefilledHash;
  const SubmitTaskScreen({super.key, this.prefilledHash});
  @override
  State<SubmitTaskScreen> createState() => _SubmitTaskScreenState();
}

class _SubmitTaskScreenState extends State<SubmitTaskScreen> {
  final _entrypointCtrl = TextEditingController(text: 'run');
  final _hashCtrl       = TextEditingController();
  String _priority      = 'normal';
  int    _timeoutMs     = 30000;
  int    _maxRetries    = 3;
  bool   _loading       = false;
  String? _result;
  String? _error;

  @override
  void dispose() {
    _entrypointCtrl.dispose();
    _hashCtrl.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    setState(() { _loading = true; _result = null; _error = null; });
    try {
      final taskId = await AetherApi.submitTask(
        entrypoint:     _entrypointCtrl.text.trim(),
        wasmModuleHash: _hashCtrl.text.trim(),
        priority:       _priority,
        timeoutMs:      _timeoutMs,
        maxRetries:     _maxRetries,
      );
      setState(() => _result = taskId);
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: const Text('Task Gönder', style: TextStyle(color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white70),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(20),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [

          _label('Entrypoint fonksiyonu'),
          _field(controller: _entrypointCtrl, hint: 'run'),
          const SizedBox(height: 16),

          _label('WASM Modül Hash (opsiyonel)'),
          _field(
            controller: _hashCtrl,
            hint: 'SHA-256 hex (64 karakter) — boş bırakılabilir',
          ),
          const SizedBox(height: 16),

          _label('Öncelik'),
          _PrioritySelector(
            value: _priority,
            onChanged: (v) => setState(() => _priority = v),
          ),
          const SizedBox(height: 16),

          Row(children: [
            Expanded(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              _label('Timeout (ms)'),
              _picker(value: _timeoutMs, options: [5000,15000,30000,60000],
                  onChanged: (v) => setState(() => _timeoutMs = v)),
            ])),
            const SizedBox(width: 12),
            Expanded(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              _label('Max Retry'),
              _picker(value: _maxRetries, options: [0,1,3,5],
                  onChanged: (v) => setState(() => _maxRetries = v)),
            ])),
          ]),
          const SizedBox(height: 28),

          SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: const Color(0xFF6C63FF),
                padding: const EdgeInsets.symmetric(vertical: 16),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: _loading ? null : _submit,
              icon: _loading
                  ? const SizedBox(width: 18, height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                  : const Icon(Icons.send),   // ← ikon fix
              label: Text(
                _loading ? 'Gönderiliyor...' : 'Gönder',
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
              ),
            ),
          ),
          const SizedBox(height: 20),

          // ── Başarı ──────────────────────────────────
          if (_result != null) ...[
            _ResultBox(
              label: '✓ Task kabul edildi',
              content: _result!,
              color: const Color(0xFF4CAF50),
            ),
            const SizedBox(height: 12),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                style: OutlinedButton.styleFrom(
                  foregroundColor: const Color(0xFF6C63FF),
                  side: const BorderSide(color: Color(0xFF6C63FF)),
                  padding: const EdgeInsets.symmetric(vertical: 14),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                ),
                onPressed: () => Navigator.pushReplacement(
                  context,
                  MaterialPageRoute(builder: (_) => const TaskListScreen()),
                ),
                icon: const Icon(Icons.list_alt, size: 18),
                label: const Text('Task Listesine Git'),
              ),
            ),
          ],

          // ── Hata ────────────────────────────────────
          if (_error != null)
            _ResultBox(
              label: '✗ Hata',
              content: _error!,
              color: const Color(0xFFEF5350),
            ),
        ]),
      ),
    );
  }

  Widget _label(String t) => Padding(
    padding: const EdgeInsets.only(bottom: 6),
    child: Text(t, style: const TextStyle(color: Colors.white54, fontSize: 12, letterSpacing: 0.8)),
  );

  Widget _field({required TextEditingController controller, required String hint}) =>
      TextField(
        controller: controller,
        style: const TextStyle(color: Colors.white),
        decoration: InputDecoration(
          hintText: hint,
          hintStyle: const TextStyle(color: Colors.white24),
          filled: true,
          fillColor: const Color(0xFF1A1A2E),
          border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(10),
              borderSide: const BorderSide(color: Color(0xFF6C63FF), width: 0.5)),
          enabledBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(10),
              borderSide: const BorderSide(color: Colors.white12)),
          focusedBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(10),
              borderSide: const BorderSide(color: Color(0xFF6C63FF))),
        ),
      );

  Widget _picker({
    required int value,
    required List<int> options,
    required ValueChanged<int> onChanged,
  }) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 12),
    decoration: BoxDecoration(
      color: const Color(0xFF1A1A2E),
      borderRadius: BorderRadius.circular(10),
      border: Border.all(color: Colors.white12),
    ),
    child: DropdownButtonHideUnderline(
      child: DropdownButton<int>(
        value: options.contains(value) ? value : options.first,
        dropdownColor: const Color(0xFF1A1A2E),
        style: const TextStyle(color: Colors.white),
        items: options.map((o) => DropdownMenuItem(value: o, child: Text('$o'))).toList(),
        onChanged: (v) => v != null ? onChanged(v) : null,
      ),
    ),
  );
}

// ── Priority seçici ───────────────────────────────────────

class _PrioritySelector extends StatelessWidget {
  final String value;
  final ValueChanged<String> onChanged;
  const _PrioritySelector({required this.value, required this.onChanged});

  static const _opts = [
    ('critical', '🔴 Critical', Color(0xFFEF5350)),
    ('high',     '🟠 High',     Color(0xFFFF9800)),
    ('normal',   '🟢 Normal',   Color(0xFF4CAF50)),
    ('low',      '🔵 Low',      Color(0xFF42A5F5)),
  ];

  @override
  Widget build(BuildContext context) => Row(
    children: _opts.map((o) {
      final (key, label, color) = o;
      final sel = value == key;
      return Expanded(
        child: GestureDetector(
          onTap: () => onChanged(key),
          child: Container(
            margin: const EdgeInsets.only(right: 6),
            padding: const EdgeInsets.symmetric(vertical: 8),
            decoration: BoxDecoration(
              color: sel ? color.withOpacity(0.15) : const Color(0xFF1A1A2E),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: sel ? color : Colors.white12),
            ),
            child: Text(label,
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: sel ? color : Colors.white38,
                  fontSize: 10,
                  fontWeight: sel ? FontWeight.bold : FontWeight.normal,
                )),
          ),
        ),
      );
    }).toList(),
  );
}

// ── Sonuç kutusu ──────────────────────────────────────────

class _ResultBox extends StatelessWidget {
  final String label, content;
  final Color color;
  const _ResultBox({required this.label, required this.content, required this.color});

  @override
  Widget build(BuildContext context) => Container(
    width: double.infinity,
    padding: const EdgeInsets.all(14),
    decoration: BoxDecoration(
      color: color.withOpacity(0.08),
      borderRadius: BorderRadius.circular(10),
      border: Border.all(color: color.withOpacity(0.3)),
    ),
    child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text(label,
          style: TextStyle(color: color, fontWeight: FontWeight.bold, fontSize: 12)),
      const SizedBox(height: 6),
      SelectableText(content,
          style: const TextStyle(
              color: Colors.white70, fontFamily: 'monospace', fontSize: 12)),
    ]),
  );
}
