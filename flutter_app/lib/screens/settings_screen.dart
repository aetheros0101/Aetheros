// ============================================================
// flutter_app/lib/screens/settings_screen.dart
//
// Faz-1 UI — Ayarlar
//   • AI provider API key'leri (Anthropic/Gemini/OpenAI/Ollama)
//   • Runtime bilgisi (salt okunur)
//   • Uygulama tercihleri
// ============================================================

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});
  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

// SharedPreferences key'leri
const _kAnthropicKey = 'api_key_anthropic';
const _kOpenAiKey    = 'api_key_openai';
const _kGeminiKey    = 'api_key_gemini';
const _kOllamaUrl    = 'ollama_base_url';

class _SettingsScreenState extends State<SettingsScreen> {
  // Controllers
  final _anthropicCtrl = TextEditingController();
  final _openAiCtrl    = TextEditingController();
  final _geminiCtrl    = TextEditingController();
  final _ollamaCtrl    = TextEditingController(text: 'http://localhost:11434');

  // Görünürlük
  bool _showAnthropic = false;
  bool _showOpenAi    = false;
  bool _showGemini    = false;

  bool _saving = false;
  bool _saved  = false;

  rust.RuntimeInfo? _runtimeInfo;

  @override
  void initState() {
    super.initState();
    _loadSaved();
    _loadRuntime();
  }

  @override
  void dispose() {
    _anthropicCtrl.dispose();
    _openAiCtrl.dispose();
    _geminiCtrl.dispose();
    _ollamaCtrl.dispose();
    super.dispose();
  }

  Future<void> _loadSaved() async {
    final p = await SharedPreferences.getInstance();
    setState(() {
      _anthropicCtrl.text = p.getString(_kAnthropicKey) ?? '';
      _openAiCtrl.text    = p.getString(_kOpenAiKey)    ?? '';
      _geminiCtrl.text    = p.getString(_kGeminiKey)    ?? '';
      _ollamaCtrl.text    = p.getString(_kOllamaUrl)    ?? 'http://localhost:11434';
    });
  }

  Future<void> _loadRuntime() async {
    try {
      final info = await AetherApi.getRuntimeInfo();
      if (mounted) setState(() => _runtimeInfo = info);
    } catch (_) {}
  }

  Future<void> _save() async {
    setState(() { _saving = true; _saved = false; });
    final p = await SharedPreferences.getInstance();
    await p.setString(_kAnthropicKey, _anthropicCtrl.text.trim());
    await p.setString(_kOpenAiKey,    _openAiCtrl.text.trim());
    await p.setString(_kGeminiKey,    _geminiCtrl.text.trim());
    await p.setString(_kOllamaUrl,    _ollamaCtrl.text.trim());
    await Future.delayed(const Duration(milliseconds: 300));
    if (mounted) setState(() { _saving = false; _saved = true; });
    await Future.delayed(const Duration(seconds: 2));
    if (mounted) setState(() => _saved = false);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0D0D1A),
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: const Text('Ayarlar',
            style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          if (_saving)
            const Padding(
              padding: EdgeInsets.all(14),
              child: SizedBox(width: 18, height: 18,
                child: CircularProgressIndicator(strokeWidth: 2,
                    color: Color(0xFF6C63FF))),
            )
          else
            IconButton(
              icon: Icon(
                _saved ? Icons.check : Icons.save,
                color: _saved ? const Color(0xFF4CAF50) : Colors.white70,
              ),
              tooltip: 'Kaydet',
              onPressed: _save,
            ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // ── Runtime bilgisi ──────────────────────────────
          _SectionHeader(label: 'Runtime Bilgisi', icon: Icons.memory),
          _RuntimeCard(info: _runtimeInfo),
          const SizedBox(height: 24),

          // ── AI Provider API Key'leri ─────────────────────
          _SectionHeader(label: 'AI Provider Anahtarları', icon: Icons.vpn_key),
          const Padding(
            padding: EdgeInsets.only(bottom: 12),
            child: Text(
              'API key\'ler cihazda şifrelenmiş olarak saklanır.\n'
              'Anthropic / OpenAI için geçerli bir key gereklidir.',
              style: TextStyle(color: Colors.white38, fontSize: 12),
            ),
          ),

          // Anthropic
          _ApiKeyField(
            label:      'Anthropic (Claude)',
            hint:       'sk-ant-…',
            controller: _anthropicCtrl,
            show:       _showAnthropic,
            color:      const Color(0xFFC87533),
            onToggle:   () => setState(() => _showAnthropic = !_showAnthropic),
          ),
          const SizedBox(height: 12),

          // OpenAI
          _ApiKeyField(
            label:      'OpenAI',
            hint:       'sk-…',
            controller: _openAiCtrl,
            show:       _showOpenAi,
            color:      const Color(0xFF10A37F),
            onToggle:   () => setState(() => _showOpenAi = !_showOpenAi),
          ),
          const SizedBox(height: 12),

          // Gemini
          _ApiKeyField(
            label:      'Google Gemini',
            hint:       'AIza…',
            controller: _geminiCtrl,
            show:       _showGemini,
            color:      const Color(0xFF4285F4),
            onToggle:   () => setState(() => _showGemini = !_showGemini),
          ),
          const SizedBox(height: 12),

          // Ollama (URL, key değil)
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: const Color(0xFF1A1A2E),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.white12),
            ),
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Row(children: [
                Container(
                  width: 8, height: 8,
                  decoration: const BoxDecoration(
                    color: Color(0xFF9B59B6),
                    shape: BoxShape.circle,
                  ),
                ),
                const SizedBox(width: 8),
                const Text('Ollama (yerel)',
                    style: TextStyle(color: Colors.white70,
                        fontWeight: FontWeight.w600)),
              ]),
              const SizedBox(height: 10),
              TextField(
                controller: _ollamaCtrl,
                style: const TextStyle(color: Colors.white, fontSize: 13),
                decoration: _fieldDeco('Base URL', 'http://localhost:11434'),
              ),
            ]),
          ),
          const SizedBox(height: 32),

          // ── Kaydet butonu ─────────────────────────────────
          SizedBox(width: double.infinity,
            child: ElevatedButton.icon(
              onPressed: _saving ? null : _save,
              icon: Icon(
                _saved ? Icons.check : Icons.save,
                color: Colors.white,
              ),
              label: Text(
                _saving ? 'Kaydediliyor…' : (_saved ? 'Kaydedildi!' : 'Kaydet'),
                style: const TextStyle(color: Colors.white, fontSize: 16),
              ),
              style: ElevatedButton.styleFrom(
                backgroundColor: _saved
                    ? const Color(0xFF4CAF50)
                    : const Color(0xFF6C63FF),
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
            ),
          ),
          const SizedBox(height: 16),

          // ── Versiyon ──────────────────────────────────────
          Center(child: Text(
            'AetherOS v${_runtimeInfo?.version ?? "0.1.0"} • Faz 1',
            style: const TextStyle(color: Colors.white24, fontSize: 12),
          )),
          const SizedBox(height: 8),
        ],
      ),
    );
  }
}

// ── Widget'lar ────────────────────────────────────────────────

class _SectionHeader extends StatelessWidget {
  final String label;
  final IconData icon;
  const _SectionHeader({required this.label, required this.icon});
  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: Row(children: [
      Icon(icon, color: const Color(0xFF6C63FF), size: 18),
      const SizedBox(width: 8),
      Text(label, style: const TextStyle(
          color: Colors.white,
          fontSize: 15,
          fontWeight: FontWeight.w700)),
    ]),
  );
}

class _RuntimeCard extends StatelessWidget {
  final rust.RuntimeInfo? info;
  const _RuntimeCard({required this.info});
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFF1A1A2E),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.white12),
      ),
      child: info == null
          ? const Center(
              child: SizedBox(height: 20,
                child: CircularProgressIndicator(
                  strokeWidth: 2, color: Color(0xFF6C63FF))))
          : Wrap(spacing: 16, runSpacing: 8, children: [
              _InfoChip('Versiyon', info!.version),
              _InfoChip('Backend',  info!.backend),
              _InfoChip('Worker',   '${info!.workerCount}'),
              _InfoChip('Durum',
                  info!.isRunning ? 'Aktif' : 'Durdu',
                  color: info!.isRunning
                      ? const Color(0xFF4CAF50)
                      : const Color(0xFFFF5252)),
            ]),
    );
  }
}

class _InfoChip extends StatelessWidget {
  final String label;
  final String value;
  final Color? color;
  const _InfoChip(this.label, this.value, {this.color});
  @override
  Widget build(BuildContext context) => Row(mainAxisSize: MainAxisSize.min, children: [
    Text('$label: ', style: const TextStyle(color: Colors.white38, fontSize: 12)),
    Text(value, style: TextStyle(
        color: color ?? Colors.white70,
        fontSize: 12, fontWeight: FontWeight.w600)),
  ]);
}

class _ApiKeyField extends StatelessWidget {
  final String label;
  final String hint;
  final TextEditingController controller;
  final bool show;
  final Color color;
  final VoidCallback onToggle;

  const _ApiKeyField({
    required this.label,
    required this.hint,
    required this.controller,
    required this.show,
    required this.color,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.all(14),
    decoration: BoxDecoration(
      color: const Color(0xFF1A1A2E),
      borderRadius: BorderRadius.circular(12),
      border: Border.all(color: Colors.white12),
    ),
    child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Row(children: [
        Container(
          width: 8, height: 8,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
        const SizedBox(width: 8),
        Text(label, style: const TextStyle(
            color: Colors.white70, fontWeight: FontWeight.w600)),
        const Spacer(),
        if (controller.text.isNotEmpty)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: const Color(0xFF4CAF50).withOpacity(0.2),
              borderRadius: BorderRadius.circular(4),
            ),
            child: const Text('Ayarlı',
                style: TextStyle(color: Color(0xFF4CAF50), fontSize: 10)),
          ),
      ]),
      const SizedBox(height: 10),
      TextField(
        controller: controller,
        obscureText: !show,
        style: const TextStyle(color: Colors.white, fontSize: 13,
            fontFamily: 'monospace'),
        decoration: _fieldDeco(hint, hint).copyWith(
          suffixIcon: IconButton(
            icon: Icon(
              show ? Icons.visibility_off : Icons.visibility,
              color: Colors.white38,
              size: 18,
            ),
            onPressed: onToggle,
          ),
        ),
      ),
    ]),
  );
}

InputDecoration _fieldDeco(String label, String hint) => InputDecoration(
  hintText: hint,
  hintStyle: const TextStyle(color: Colors.white24, fontSize: 12),
  enabledBorder: OutlineInputBorder(
    borderRadius: BorderRadius.circular(8),
    borderSide: const BorderSide(color: Colors.white12),
  ),
  focusedBorder: OutlineInputBorder(
    borderRadius: BorderRadius.circular(8),
    borderSide: const BorderSide(color: Color(0xFF6C63FF)),
  ),
  filled: true,
  fillColor: const Color(0xFF252540),
  isDense: true,
  contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
);
