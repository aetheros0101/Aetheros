// ============================================================
// flutter_app/lib/screens/ai_settings_screen.dart
// Sprint 4 — Gemini API ayarları
// ============================================================

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../services/gemini_service.dart';

const _keyApiKey = 'gemini_api_key';
const _keyModel  = 'gemini_model';

class AiSettingsScreen extends StatefulWidget {
  const AiSettingsScreen({super.key});
  @override
  State<AiSettingsScreen> createState() => _AiSettingsScreenState();
}

class _AiSettingsScreenState extends State<AiSettingsScreen> {
  final _keyCtrl   = TextEditingController();
  bool  _obscure   = true;
  String _model    = 'gemini-2.0-flash';
  bool  _testing   = false;
  bool? _testOk;
  String _testMsg  = '';
  bool  _saved     = false;

  static const _models = [
    ('gemini-2.0-flash',      'Gemini 2.0 Flash   (Ücretsiz, Hızlı)'),
    ('gemini-1.5-flash',      'Gemini 1.5 Flash   (Ücretsiz)'),
    ('gemini-1.5-flash-8b',   'Gemini 1.5 Flash 8B (Ücretsiz, Hafif)'),
    ('gemini-1.5-pro',        'Gemini 1.5 Pro      (Sınırlı Ücretsiz)'),
  ];

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() { _keyCtrl.dispose(); super.dispose(); }

  Future<void> _load() async {
    final p = await SharedPreferences.getInstance();
    setState(() {
      _keyCtrl.text = p.getString(_keyApiKey) ?? '';
      _model        = p.getString(_keyModel)  ?? 'gemini-2.0-flash';
    });
  }

  Future<void> _save() async {
    final p = await SharedPreferences.getInstance();
    await p.setString(_keyApiKey, _keyCtrl.text.trim());
    await p.setString(_keyModel,  _model);
    setState(() => _saved = true);
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
        content: Text('✅ Ayarlar kaydedildi'),
        backgroundColor: Color(0xFF4CAF50),
        duration: Duration(seconds: 2),
      ));
    }
    await Future.delayed(const Duration(seconds: 2));
    if (mounted) setState(() => _saved = false);
  }

  Future<void> _test() async {
    final key = _keyCtrl.text.trim();
    if (key.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
        content: Text('API anahtarı gir'),
        backgroundColor: Color(0xFFEF5350),
      ));
      return;
    }
    setState(() { _testing = true; _testOk = null; _testMsg = ''; });
    final result = await GeminiService.testApiKey(key, model: _model);
    setState(() {
      _testing = false;
      _testOk  = result.ok;
      _testMsg = result.message;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: const Text('AI Ayarları', style: TextStyle(color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          IconButton(
            icon: Icon(_saved ? Icons.check : Icons.save,
                color: _saved ? const Color(0xFF4CAF50) : Colors.white70),
            onPressed: _save,
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          // ── Bilgi kartı ──────────────────────────────
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
                Icon(Icons.info_outline,
                    color: Color(0xFF6C63FF), size: 16),
                SizedBox(width: 8),
                Text('Gemini API Kurulumu',
                    style: TextStyle(
                        color: Color(0xFF6C63FF),
                        fontWeight: FontWeight.bold,
                        fontSize: 13)),
              ]),
              const SizedBox(height: 8),
              const Text(
                '1. aistudio.google.com adresine git\n'
                '2. "Get API Key" → "Create API Key"\n'
                '3. Ücretsiz tier: dakikada 15 istek, günde 1500 istek',
                style: TextStyle(
                    color: Colors.white60, fontSize: 12, height: 1.6),
              ),
            ]),
          ),
          const SizedBox(height: 24),

          // ── API Key ──────────────────────────────────
          _label('Google Gemini API Anahtarı'),
          const SizedBox(height: 6),
          TextField(
            controller: _keyCtrl,
            obscureText: _obscure,
            style: const TextStyle(
                color: Colors.white, fontFamily: 'monospace', fontSize: 13),
            decoration: InputDecoration(
              hintText: 'AIza...',
              hintStyle: const TextStyle(color: Colors.white24),
              filled: true,
              fillColor: const Color(0xFF1A1A2E),
              border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(10),
                  borderSide: BorderSide.none),
              focusedBorder: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(10),
                  borderSide: const BorderSide(color: Color(0xFF6C63FF))),
              suffixIcon: IconButton(
                icon: Icon(_obscure ? Icons.visibility_off : Icons.visibility,
                    color: Colors.white38, size: 18),
                onPressed: () => setState(() => _obscure = !_obscure),
              ),
            ),
          ),
          const SizedBox(height: 20),

          // ── Model seçimi ─────────────────────────────
          _label('Model'),
          const SizedBox(height: 6),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14),
            decoration: BoxDecoration(
              color: const Color(0xFF1A1A2E),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: Colors.white12),
            ),
            child: DropdownButtonHideUnderline(
              child: DropdownButton<String>(
                value: _model,
                dropdownColor: const Color(0xFF1A1A2E),
                style: const TextStyle(color: Colors.white, fontSize: 13),
                isExpanded: true,
                items: _models.map((m) => DropdownMenuItem(
                  value: m.$1,
                  child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start, children: [
                    Text(m.$2,
                        style: const TextStyle(
                            color: Colors.white, fontSize: 13)),
                  ]),
                )).toList(),
                onChanged: (v) { if (v != null) setState(() => _model = v); },
              ),
            ),
          ),
          const SizedBox(height: 24),

          // ── Test butonu ──────────────────────────────
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
              onPressed: _testing ? null : _test,
              icon: _testing
                  ? const SizedBox(width: 16, height: 16,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: Color(0xFF6C63FF)))
                  : const Icon(Icons.wifi_tethering, size: 16),
              label: Text(_testing ? 'Test ediliyor...' : 'Bağlantıyı Test Et'),
            ),
          ),

          // ── Test sonucu ──────────────────────────────
          if (_testOk != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: (_testOk! ? const Color(0xFF4CAF50) : const Color(0xFFEF5350))
                    .withOpacity(0.08),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                  color: (_testOk! ? const Color(0xFF4CAF50) : const Color(0xFFEF5350))
                      .withOpacity(0.3),
                ),
              ),
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text(
                  _testOk! ? '✅ Bağlantı başarılı!' : '❌ Bağlantı başarısız',
                  style: TextStyle(
                    color: _testOk! ? const Color(0xFF4CAF50) : const Color(0xFFEF5350),
                    fontWeight: FontWeight.bold,
                    fontSize: 12,
                  ),
                ),
                if (_testMsg.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  Text(_testMsg,
                      style: const TextStyle(
                          color: Colors.white60, fontSize: 11)),
                ],
              ]),
            ),
          ],
          const SizedBox(height: 24),

          // ── Kaydet ───────────────────────────────────
          SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: const Color(0xFF6C63FF),
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: _save,
              icon: const Icon(Icons.save, size: 16),
              label: const Text('Kaydet',
                  style: TextStyle(fontSize: 15, fontWeight: FontWeight.bold)),
            ),
          ),
        ],
      ),
    );
  }

  Widget _label(String t) => Text(t,
      style: const TextStyle(
          color: Colors.white54, fontSize: 12, letterSpacing: 0.8));
}

// ── Yardımcı: API anahtarını oku ─────────────────────────

Future<({String apiKey, String model})> loadAiSettings() async {
  final p = await SharedPreferences.getInstance();
  return (
    apiKey: p.getString(_keyApiKey) ?? '',
    model:  p.getString(_keyModel)  ?? 'gemini-2.0-flash',
  );
}
