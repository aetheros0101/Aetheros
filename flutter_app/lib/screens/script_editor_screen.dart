// ============================================================
// flutter_app/lib/screens/script_editor_screen.dart
// Sprint 3 — WAT + Workflow JSON editörü
// ============================================================

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'dart:convert';
import '../api/aetheros_api.dart';

// ── Şablonlar ─────────────────────────────────────────────

class _Template {
  final String name;
  final String code;
  const _Template(this.name, this.code);
}

const _watTemplates = [
  _Template('Merhaba Dünya', r'''(module
  ;; Basit bir sayı döndüren WASM modülü
  (func $run (export "run") (result i32)
    i32.const 42
  )
)'''),
  _Template('Toplama', r'''(module
  ;; İki i32 sayıyı topla
  (func $add (export "add")
    (param $a i32) (param $b i32)
    (result i32)
    local.get $a
    local.get $b
    i32.add
  )

  (func $run (export "run") (result i32)
    i32.const 10
    i32.const 32
    call $add
  )
)'''),
  _Template('Fibonacci', r'''(module
  (func $fib (export "fib")
    (param $n i32) (result i32)
    (if (result i32) (i32.le_s (local.get $n) (i32.const 1))
      (then (local.get $n))
      (else
        (i32.add
          (call $fib (i32.sub (local.get $n) (i32.const 1)))
          (call $fib (i32.sub (local.get $n) (i32.const 2)))
        )
      )
    )
  )

  (func $run (export "run") (result i32)
    ;; fib(10) = 55
    i32.const 10
    call $fib
  )
)'''),
  _Template('Bellek Kullanımı', r'''(module
  (memory (export "memory") 1)
  (data (i32.const 0) "AetherOS\00")

  (func $run (export "run") (result i32)
    ;; Belleğin 0. byte'ını oku ("A" = 65)
    i32.const 0
    i32.load8_u
  )
)'''),
];

const _jsonTemplates = [
  _Template('Basit Görev', '''{
  "name": "basit-gorev",
  "version": "1.0.0",
  "entrypoint": "run",
  "timeout_ms": 30000,
  "steps": [
    {
      "id": "adim1",
      "type": "wasm_call",
      "function": "run",
      "input": {}
    }
  ]
}'''),
  _Template('Zincirleme Görev', '''{
  "name": "zincirleme",
  "version": "1.0.0",
  "entrypoint": "pipeline",
  "steps": [
    {
      "id": "veri-hazirla",
      "type": "wasm_call",
      "function": "prepare",
      "input": { "source": "local" }
    },
    {
      "id": "isle",
      "type": "wasm_call",
      "function": "process",
      "depends_on": ["veri-hazirla"],
      "input": { "mode": "fast" }
    },
    {
      "id": "kaydet",
      "type": "wasm_call",
      "function": "save",
      "depends_on": ["isle"]
    }
  ]
}'''),
  _Template('AI Çıkarım', '''{
  "name": "ai-inference",
  "version": "1.0.0",
  "entrypoint": "infer",
  "model": {
    "backend": "wasmi",
    "precision": "fp32"
  },
  "steps": [
    {
      "id": "on-isle",
      "type": "wasm_call",
      "function": "preprocess"
    },
    {
      "id": "cikarim",
      "type": "wasm_call",
      "function": "infer",
      "depends_on": ["on-isle"]
    }
  ]
}'''),
];

const _watKey  = 'script_editor_wat';
const _jsonKey = 'script_editor_json';

// ── Ekran ─────────────────────────────────────────────────

class ScriptEditorScreen extends StatefulWidget {
  const ScriptEditorScreen({super.key});
  @override
  State<ScriptEditorScreen> createState() => _ScriptEditorScreenState();
}

class _ScriptEditorScreenState extends State<ScriptEditorScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs;
  final _watCtrl  = TextEditingController();
  final _jsonCtrl = TextEditingController();
  bool   _saved      = false;
  bool   _compiling  = false;
  String? _compileError;

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 2, vsync: this);
    _tabs.addListener(() => setState(() {}));
    _loadSaved();
  }

  @override
  void dispose() {
    _tabs.dispose();
    _watCtrl.dispose();
    _jsonCtrl.dispose();
    super.dispose();
  }

  Future<void> _loadSaved() async {
    final p = await SharedPreferences.getInstance();
    final wat  = p.getString(_watKey);
    final json = p.getString(_jsonKey);
    if (wat  != null) _watCtrl.text  = wat;
    if (json != null) _jsonCtrl.text = json;
    if (wat == null && json == null) {
      _watCtrl.text  = _watTemplates.first.code;
      _jsonCtrl.text = _jsonTemplates.first.code;
    }
  }

  /// WAT sekmesi: derle + yükle.
  /// JSON sekmesi: SharedPreferences'a kaydet.
  Future<void> _save() async {
    if (_tabs.index == 0) {
      await _compileAndUpload();
    } else {
      await _saveJson();
    }
  }

  Future<void> _saveJson() async {
    final p = await SharedPreferences.getInstance();
    await p.setString(_watKey,  _watCtrl.text);
    await p.setString(_jsonKey, _jsonCtrl.text);
    setState(() { _saved = true; _compileError = null; });
    await Future.delayed(const Duration(seconds: 2));
    if (mounted) setState(() => _saved = false);
  }

  Future<void> _compileAndUpload() async {
    final watSource = _watCtrl.text.trim();
    if (watSource.isEmpty) {
      setState(() => _compileError = 'WAT kodu boş.');
      return;
    }

    setState(() { _compiling = true; _compileError = null; });

    try {
      final result = await AetherApi.compileWatToWasm(
        name:       'script-${DateTime.now().millisecondsSinceEpoch}',
        watSource:  watSource,
        entrypoint: 'run',
        timeoutMs:  30000,
      );

      // SharedPreferences'a da kaydet (editör içeriği)
      final p = await SharedPreferences.getInstance();
      await p.setString(_watKey, watSource);

      // WasmModuleScreen listesine de ekle
      const modulesKey = 'aetheros_wasm_modules';
      final rawList = p.getStringList(modulesKey) ?? [];
      final alreadyExists = rawList.any((s) {
        try { return (jsonDecode(s) as Map)['hash'] == result.hash; }
        catch (_) { return false; }
      });
      if (!alreadyExists) {
        rawList.insert(0, jsonEncode({
          'hash':       result.hash,
          'size':       result.size,
          'filename':   'script-${DateTime.now().millisecondsSinceEpoch}.wasm',
          'uploadedAt': DateTime.now().millisecondsSinceEpoch,
        }));
        await p.setStringList(modulesKey, rawList);
      }

      if (!mounted) return;
      setState(() { _compiling = false; _saved = true; _compileError = null; });

      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text(
          '✓ Derlendi & yüklendi\n'
          'Hash: ${result.hash.substring(0, 16)}…\n'
          'Boyut: ${result.size} bayt',
        ),
        backgroundColor: const Color(0xFF4CAF50),
        duration: const Duration(seconds: 3),
      ));

      await Future.delayed(const Duration(seconds: 2));
      if (mounted) setState(() => _saved = false);

    } catch (e) {
      if (!mounted) return;
      setState(() {
        _compiling    = false;
        _compileError = e.toString().replaceFirst('Exception: ', '');
      });
    }
  }

  void _applyTemplate(_Template t) {
    final ctrl = _tabs.index == 0 ? _watCtrl : _jsonCtrl;
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF1A1A2E),
        title: Text('${t.name} şablonu',
            style: const TextStyle(color: Colors.white, fontSize: 15)),
        content: const Text('Mevcut içerik değiştirilecek.',
            style: TextStyle(color: Colors.white60)),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('İptal',
                  style: TextStyle(color: Colors.white38))),
          TextButton(
              onPressed: () {
                ctrl.text = t.code;
                Navigator.pop(context);
              },
              child: const Text('Uygula',
                  style: TextStyle(color: Color(0xFF6C63FF)))),
        ],
      ),
    );
  }

  void _copy() {
    final text = _tabs.index == 0 ? _watCtrl.text : _jsonCtrl.text;
    Clipboard.setData(ClipboardData(text: text));
    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
      content: Text('Kod kopyalandı'),
      backgroundColor: Color(0xFF6C63FF),
      duration: Duration(seconds: 1),
    ));
  }

  void _clear() {
    final ctrl = _tabs.index == 0 ? _watCtrl : _jsonCtrl;
    ctrl.clear();
  }

  @override
  Widget build(BuildContext context) {
    final isWat  = _tabs.index == 0;
    final templates = isWat ? _watTemplates : _jsonTemplates;

    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: const Text('Script Editör',
            style: TextStyle(color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          // WAT sekmesi → Derle & Yükle | JSON sekmesi → Kaydet
          if (_compiling)
            const Padding(
              padding: EdgeInsets.all(14),
              child: SizedBox(
                width: 18, height: 18,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: Color(0xFF6C63FF),
                ),
              ),
            )
          else
            IconButton(
              icon: Icon(
                _saved
                    ? Icons.check
                    : (_tabs.index == 0 ? Icons.play_arrow : Icons.save),
                color: _saved
                    ? const Color(0xFF4CAF50)
                    : (_tabs.index == 0
                        ? const Color(0xFF6C63FF)
                        : Colors.white70),
              ),
              tooltip: _tabs.index == 0 ? 'Derle & Yükle' : 'Kaydet',
              onPressed: _save,
            ),
          // Kopyala
          IconButton(
            icon: const Icon(Icons.copy, color: Colors.white70),
            tooltip: 'Kopyala',
            onPressed: _copy,
          ),
        ],
        bottom: TabBar(
          controller: _tabs,
          indicatorColor: const Color(0xFF6C63FF),
          labelColor: const Color(0xFF6C63FF),
          unselectedLabelColor: Colors.white38,
          tabs: const [
            Tab(text: 'WAT  (WebAssembly Text)'),
            Tab(text: 'Workflow JSON'),
          ],
        ),
      ),
      body: Column(children: [
        // ── Derleme hatası banner ─────────────────────
        if (_compileError != null)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            color: const Color(0xFF3D1515),
            child: Row(children: [
              const Icon(Icons.error_outline,
                  color: Color(0xFFFF5252), size: 14),
              const SizedBox(width: 8),
              Expanded(child: Text(
                _compileError!,
                style: const TextStyle(
                    color: Color(0xFFFF8A80), fontSize: 11),
              )),
              GestureDetector(
                onTap: () => setState(() => _compileError = null),
                child: const Icon(Icons.close,
                    color: Colors.white38, size: 14),
              ),
            ]),
          ),
        // ── Şablon seçici ────────────────────────────
        _TemplateBar(
          templates: templates,
          onSelect: _applyTemplate,
          onClear: _clear,
        ),
        // ── Editör ───────────────────────────────────
        Expanded(
          child: TabBarView(
            controller: _tabs,
            children: [
              _Editor(controller: _watCtrl,  lang: 'wat'),
              _Editor(controller: _jsonCtrl, lang: 'json'),
            ],
          ),
        ),
        // ── Alt not ──────────────────────────────────
        _CompilerNotice(isWat: isWat),
      ]),
    );
  }
}

// ── Şablon seçici bar ─────────────────────────────────────

class _TemplateBar extends StatelessWidget {
  final List<_Template> templates;
  final ValueChanged<_Template> onSelect;
  final VoidCallback onClear;

  const _TemplateBar({
    required this.templates,
    required this.onSelect,
    required this.onClear,
  });

  @override
  Widget build(BuildContext context) => Container(
    height: 40,
    color: const Color(0xFF0F0F1A),
    child: Row(children: [
      Expanded(
        child: ListView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          children: templates.map((t) => GestureDetector(
            onTap: () => onSelect(t),
            child: Container(
              margin: const EdgeInsets.only(right: 6),
              padding: const EdgeInsets.symmetric(horizontal: 10),
              decoration: BoxDecoration(
                color: const Color(0xFF1A1A2E),
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: Colors.white10),
              ),
              child: Center(child: Text(t.name,
                  style: const TextStyle(
                      color: Colors.white54, fontSize: 11))),
            ),
          )).toList(),
        ),
      ),
      // Temizle
      GestureDetector(
        onTap: onClear,
        child: const Padding(
          padding: EdgeInsets.symmetric(horizontal: 12),
          child: Icon(Icons.delete_sweep,
              color: Colors.white24, size: 18),
        ),
      ),
    ]),
  );
}

// ── Kod editörü ───────────────────────────────────────────

class _Editor extends StatelessWidget {
  final TextEditingController controller;
  final String lang;
  const _Editor({required this.controller, required this.lang});

  @override
  Widget build(BuildContext context) => Container(
    color: const Color(0xFF0D0D18),
    child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
      // Satır numaraları
      _LineNumbers(controller: controller),
      // Kod alanı
      Expanded(
        child: TextField(
          controller: controller,
          maxLines: null,
          expands: true,
          style: const TextStyle(
            color: Color(0xFFE0E0E0),
            fontFamily: 'monospace',
            fontSize: 12.5,
            height: 1.6,
          ),
          decoration: const InputDecoration(
            border: InputBorder.none,
            contentPadding: EdgeInsets.fromLTRB(4, 12, 12, 12),
          ),
          keyboardType: TextInputType.multiline,
          textInputAction: TextInputAction.newline,
          autocorrect: false,
          enableSuggestions: false,
        ),
      ),
    ]),
  );
}

// ── Satır numaraları ──────────────────────────────────────

class _LineNumbers extends StatefulWidget {
  final TextEditingController controller;
  const _LineNumbers({required this.controller});
  @override
  State<_LineNumbers> createState() => _LineNumbersState();
}

class _LineNumbersState extends State<_LineNumbers> {
  int _lines = 1;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_update);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_update);
    super.dispose();
  }

  void _update() {
    final n = '\n'.allMatches(widget.controller.text).length + 1;
    if (n != _lines) setState(() => _lines = n);
  }

  @override
  Widget build(BuildContext context) => Container(
    width: 38,
    color: const Color(0xFF12121F),
    padding: const EdgeInsets.only(top: 12),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: List.generate(_lines, (i) => Padding(
        padding: const EdgeInsets.only(right: 6),
        child: Text(
          '${i + 1}',
          style: const TextStyle(
            color: Colors.white24,
            fontFamily: 'monospace',
            fontSize: 12.5,
            height: 1.6,
          ),
        ),
      )),
    ),
  );
}

// ── Sprint 4 derleyici notu ───────────────────────────────

class _CompilerNotice extends StatelessWidget {
  final bool isWat;
  const _CompilerNotice({required this.isWat});

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
    color: const Color(0xFF12121F),
    child: Row(children: [
      Icon(
        isWat ? Icons.rocket_launch : Icons.info_outline,
        color: const Color(0xFF6C63FF),
        size: 14,
      ),
      const SizedBox(width: 8),
      Expanded(child: Text(
        isWat
            ? '▶ butonu: WAT → WASM derle + ModuleStore\'a yükle. '
              'Ardından WASM Modüller ekranından task gönder.'
            : 'JSON workflow kopyalayıp kaydet. '
              'Workflow çalıştırma ayrı aşamada gelecek.',
        style: const TextStyle(color: Colors.white38, fontSize: 11),
      )),
    ]),
  );
}
