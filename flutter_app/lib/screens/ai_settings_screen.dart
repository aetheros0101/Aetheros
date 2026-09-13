// ============================================================
// flutter_app/lib/screens/ai_settings_screen.dart
// Haziran 2026 — Çoklu AI provider Ayarları
//
// Kullanıcı her provider için ayrı ayrı API key (Anthropic/OpenAI/
// Gemini) ya da host (Ollama) girip aktif eder. Birden fazla
// provider yapılandırılmışsa hangisinin kullanılacağı ("Aktif
// Model") kullanıcı tarafından seçilir — otomatik fallback yok.
// ============================================================

import 'package:flutter/material.dart';

import '../services/ai_provider_service.dart';
import '../core/app_error.dart';
import '../core/app_theme.dart';

const _bg = AetherColors.background;
const _card = AetherColors.surface;
const _accent = AetherColors.primary;
const _green = AetherColors.success;
const _red = AetherColors.danger;

class AiSettingsScreen extends StatefulWidget {
  const AiSettingsScreen({super.key});
  @override
  State<AiSettingsScreen> createState() => _AiSettingsScreenState();
}

class _AiSettingsScreenState extends State<AiSettingsScreen> {
  List<String> _enabledIds = [];
  String? _activeId;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    final ids = await AiProviderService.enabledProviderIds();
    final active = await AiProviderService.getActiveProviderId();
    if (!mounted) return;
    setState(() {
      _enabledIds = ids;
      _activeId = active;
      _loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: _bg,
      appBar: AppBar(
        backgroundColor: _bg,
        title: const Text('AI Ayarları', style: TextStyle(color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white70),
      ),
      body: _loading
          ? const Center(
              child: CircularProgressIndicator(color: _accent),
            )
          : ListView(
              padding: const EdgeInsets.all(20),
              children: [
                const Text(
                  'Bir modeli aktif etmek için API anahtarını gir (Gemini '
                  'ücretsiz) ya da kendi cihazında/ağında çalışan bir '
                  'Ollama sunucusuna bağlan. Birden fazla model '
                  'bağlarsan hangisinin kullanılacağını sen seçersin.',
                  style: TextStyle(color: Colors.white60, fontSize: 12, height: 1.5),
                ),
                const SizedBox(height: 20),
                if (_enabledIds.length > 1) ...[
                  _ActiveModelSelector(
                    enabledIds: _enabledIds,
                    activeId: _activeId,
                    onChanged: _refresh,
                  ),
                  const SizedBox(height: 20),
                ],
                ...aiProviders.map((info) => Padding(
                      padding: const EdgeInsets.only(bottom: 16),
                      child: _ProviderCard(
                        info: info,
                        isEnabled: _enabledIds.contains(info.id),
                        isActive: _activeId == info.id,
                        onChanged: _refresh,
                      ),
                    )),
              ],
            ),
    );
  }
}

// ── Aktif Model seçici (2+ provider yapılandırılmışsa görünür) ──

class _ActiveModelSelector extends StatelessWidget {
  final List<String> enabledIds;
  final String? activeId;
  final VoidCallback onChanged;

  const _ActiveModelSelector({
    required this.enabledIds,
    required this.activeId,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: _accent.withOpacity(0.08),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: _accent.withOpacity(0.25)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Row(children: [
            Icon(Icons.bolt, color: _accent, size: 16),
            SizedBox(width: 8),
            Text('Aktif Model',
                style: TextStyle(
                    color: _accent, fontWeight: FontWeight.bold, fontSize: 13)),
          ]),
          const SizedBox(height: 6),
          const Text(
            'Birden fazla model bağladın — hangisinin kullanılacağını seç.',
            style: TextStyle(color: Colors.white60, fontSize: 12),
          ),
          ...enabledIds.map((id) {
            final info = aiProviderById(id);
            return RadioListTile<String>(
              value: id,
              groupValue: activeId,
              activeColor: _accent,
              dense: true,
              contentPadding: EdgeInsets.zero,
              title: Text(info.displayName,
                  style: const TextStyle(color: Colors.white, fontSize: 13)),
              onChanged: (v) async {
                if (v == null) return;
                try {
                  await AiProviderService.setActive(v);
                  onChanged();
                } catch (e) {
                  if (!context.mounted) return;
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                    content: Text('Aktif model değiştirilemedi: $e'),
                    backgroundColor: _red,
                  ));
                }
              },
            );
          }),
        ],
      ),
    );
  }
}

// ── Tek bir provider kartı ───────────────────────────────────

class _ProviderCard extends StatefulWidget {
  final AiProviderInfo info;
  final bool isEnabled;
  final bool isActive;
  final VoidCallback onChanged;

  const _ProviderCard({
    required this.info,
    required this.isEnabled,
    required this.isActive,
    required this.onChanged,
  });

  @override
  State<_ProviderCard> createState() => _ProviderCardState();
}

class _ProviderCardState extends State<_ProviderCard> {
  final _keyCtrl = TextEditingController();
  final _baseUrlCtrl = TextEditingController();
  final _modelCtrl = TextEditingController();

  bool _obscure = true;
  late String _model;
  List<AiModelOption> _modelOptions = const [];

  bool _expanded = false;
  bool _saving = false;
  bool _fetchingModels = false;

  bool _testing = false;
  bool? _testOk;
  String _testMsg = '';

  @override
  void initState() {
    super.initState();
    _model = widget.info.defaultModel;
    _modelOptions = widget.info.modelOptions;
    _modelCtrl.text = _model;
    _baseUrlCtrl.text = widget.info.defaultBaseUrl;
    _expanded = !widget.isEnabled; // henüz kurulmamışsa açık başlasın
    _loadExisting();
  }

  @override
  void dispose() {
    _keyCtrl.dispose();
    _baseUrlCtrl.dispose();
    _modelCtrl.dispose();
    super.dispose();
  }

  Future<void> _loadExisting() async {
    final cfg = await AiProviderService.loadConfig(widget.info.id);
    if (!mounted) return;
    if (cfg != null) {
      setState(() {
        _keyCtrl.text = cfg.apiKey ?? '';
        _baseUrlCtrl.text = cfg.baseUrl ?? widget.info.defaultBaseUrl;
        _model = cfg.model;
        _modelCtrl.text = cfg.model;
      });
    }
    if (widget.info.id == 'ollama') {
      _fetchOllamaModels(silent: true);
    } else if (widget.info.needsApiKey && (cfg?.apiKey?.isNotEmpty ?? false)) {
      _fetchCloudModels(silent: true);
    }
  }

  Future<void> _fetchCloudModels({bool silent = false}) async {
    final key = _keyCtrl.text.trim();
    if (key.isEmpty) {
      if (!silent) _showSnack('Önce API anahtarını gir', isError: true);
      return;
    }

    setState(() => _fetchingModels = true);
    try {
      final models = await AiProviderService.discoverModels(
        providerId: widget.info.id,
        apiKey: key,
      );
      if (!mounted) return;

      setState(() {
        _modelOptions = models;
        if (models.isNotEmpty &&
            (_model.isEmpty || !models.any((m) => m.value == _model))) {
          _model = models.first.value;
          _modelCtrl.text = _model;
        }
        _fetchingModels = false;
      });

      if (!silent && models.isEmpty) {
        _showSnack('Bu hesap için kullanılabilir sohbet modeli bulunamadı',
            isError: true);
      }
    } catch (e) {
      if (!mounted) return;
      setState(() => _fetchingModels = false);
      if (!silent) {
        _showSnack('Model listesi alınamadı: ${userFacingError(e)}', isError: true);
      }
    }
  }

  Future<void> _fetchOllamaModels({bool silent = false}) async {
    final host = _baseUrlCtrl.text.trim();
    if (host.isEmpty) return;
    setState(() => _fetchingModels = true);
    try {
      final models = await AiProviderService.listOllamaModels(host);
      if (!mounted) return;
      setState(() {
        _modelOptions = models
            .map((m) => AiModelOption(m, m, providerId: 'ollama'))
            .toList(growable: false);
        if (models.isNotEmpty && !models.contains(_model)) {
          _model = models.first;
        }
        _fetchingModels = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _fetchingModels = false);
      if (!silent) {
        _showSnack('Ollama sunucusuna ulaşılamadı: $host', isError: true);
      }
    }
  }

  Future<void> _save() async {
    final info = widget.info;
    if (info.needsApiKey && _keyCtrl.text.trim().isEmpty) {
      _showSnack('API anahtarı gir', isError: true);
      return;
    }

    if (_model.trim().isEmpty) {
      if (_modelOptions.isNotEmpty) {
        _model = _modelOptions.first.value;
      } else {
        _showSnack(
          info.needsApiKey
              ? 'Önce “Modelleri getir” ile kullanılabilir bir model seç'
              : 'Model adı gir',
          isError: true,
        );
        return;
      }
    }

    setState(() => _saving = true);
    try {
      await AiProviderService.save(
        providerId: info.id,
        apiKey: info.needsApiKey ? _keyCtrl.text.trim() : null,
        baseUrl: info.needsBaseUrl ? _baseUrlCtrl.text.trim() : null,
        model: _model,
      );
      if (!mounted) return;
      _showSnack('✅ ${info.displayName} kaydedildi');
      widget.onChanged();
      setState(() => _expanded = false);
    } catch (e) {
      _showSnack('Kaydedilemedi: ${userFacingError(e)}', isError: true);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _remove() async {
    await AiProviderService.remove(widget.info.id);
    if (!mounted) return;
    setState(() {
      _keyCtrl.clear();
      _testOk = null;
      _testMsg = '';
      _expanded = true;
    });
    widget.onChanged();
  }

  Future<void> _test() async {
    setState(() {
      _testing = true;
      _testOk = null;
      _testMsg = '';
    });
    try {
      final result = await AiProviderService.testProvider(widget.info.id);
      if (!mounted) return;
      setState(() {
        _testing = false;
        _testOk = true;
        _testMsg = result.substring(0, result.length > 80 ? 80 : result.length);
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _testing = false;
        _testOk = false;
        _testMsg = userFacingError(e);
      });
    }
  }

  void _showSnack(String text, {bool isError = false}) {
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(
      content: Text(text),
      backgroundColor: isError ? _red : _green,
      duration: const Duration(seconds: 2),
    ));
  }

  @override
  Widget build(BuildContext context) {
    final info = widget.info;

    return Container(
      decoration: BoxDecoration(
        color: _card,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(
          color: widget.isActive ? _accent.withOpacity(0.5) : Colors.white12,
        ),
      ),
      child: Column(
        children: [
          InkWell(
            borderRadius: BorderRadius.circular(14),
            onTap: () => setState(() => _expanded = !_expanded),
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  Expanded(
                    child: Row(children: [
                      Text(info.displayName,
                          style: const TextStyle(
                              color: Colors.white,
                              fontWeight: FontWeight.bold,
                              fontSize: 14)),
                      if (widget.isActive) ...[
                        const SizedBox(width: 8),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 2),
                          decoration: BoxDecoration(
                            color: _accent.withOpacity(0.2),
                            borderRadius: BorderRadius.circular(20),
                          ),
                          child: const Text('AKTİF',
                              style: TextStyle(
                                  color: _accent,
                                  fontSize: 10,
                                  fontWeight: FontWeight.bold)),
                        ),
                      ] else if (widget.isEnabled) ...[
                        const SizedBox(width: 8),
                        const Icon(Icons.check_circle,
                            color: _green, size: 14),
                      ],
                    ]),
                  ),
                  Icon(
                    _expanded ? Icons.expand_less : Icons.expand_more,
                    color: Colors.white38,
                  ),
                ],
              ),
            ),
          ),
          if (_expanded)
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
              child: _buildForm(info),
            ),
        ],
      ),
    );
  }

  Widget _buildForm(AiProviderInfo info) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Divider(color: Colors.white12, height: 1),
        const SizedBox(height: 14),

        // ── Yardım kartı ─────────────────────────────────
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Colors.white.withOpacity(0.04),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Text(
            info.helpText,
            style: const TextStyle(color: Colors.white54, fontSize: 11.5, height: 1.5),
          ),
        ),
        const SizedBox(height: 16),

        // ── API key (cloud) ya da Host (Ollama) ──────────
        if (info.needsApiKey) ...[
          _label('API Anahtarı'),
          const SizedBox(height: 6),
          TextField(
            controller: _keyCtrl,
            obscureText: _obscure,
            style: const TextStyle(
                color: Colors.white, fontFamily: 'monospace', fontSize: 13),
            decoration: _inputDecoration(
              hint: 'sk-... / AIza...',
              suffixIcon: IconButton(
                icon: Icon(_obscure ? Icons.visibility_off : Icons.visibility,
                    color: Colors.white38, size: 18),
                onPressed: () => setState(() => _obscure = !_obscure),
              ),
            ),
          ),
          const SizedBox(height: 16),
        ],
        if (info.needsBaseUrl) ...[
          _label('Sunucu Adresi'),
          const SizedBox(height: 6),
          Row(children: [
            Expanded(
              child: TextField(
                controller: _baseUrlCtrl,
                style: const TextStyle(color: Colors.white, fontSize: 13),
                decoration: _inputDecoration(hint: 'http://127.0.0.1:11434'),
              ),
            ),
            const SizedBox(width: 8),
            IconButton(
              tooltip: 'Modelleri listele',
              onPressed: _fetchingModels ? null : () => _fetchOllamaModels(),
              icon: _fetchingModels
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2, color: _accent),
                    )
                  : const Icon(Icons.refresh, color: _accent, size: 20),
            ),
          ]),
          const SizedBox(height: 16),
        ],

        // ── Model seçimi ──────────────────────────────────
        Row(
          children: [
            Expanded(child: _label('Model')),
            if (info.needsApiKey)
              TextButton.icon(
                onPressed: _fetchingModels ? null : () => _fetchCloudModels(),
                style: TextButton.styleFrom(
                  foregroundColor: _accent,
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  minimumSize: Size.zero,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
                icon: _fetchingModels
                    ? const SizedBox(
                        width: 14,
                        height: 14,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: _accent,
                        ),
                      )
                    : const Icon(Icons.sync, size: 15),
                label: Text(_fetchingModels ? 'Getiriliyor...' : 'Modelleri getir'),
              ),
          ],
        ),
        const SizedBox(height: 6),
        _modelOptions.isEmpty
            ? TextField(
                controller: _modelCtrl,
                style: const TextStyle(color: Colors.white, fontSize: 13),
                decoration: _inputDecoration(hint: info.needsApiKey ? 'Model ID (isteğe bağlı)' : info.defaultModel),
                onChanged: (v) =>
                    _model = v.trim().isEmpty ? widget.info.defaultModel : v.trim(),
              )
            : Container(
                padding: const EdgeInsets.symmetric(horizontal: 14),
                decoration: BoxDecoration(
                  color: AetherColors.background,
                  borderRadius: BorderRadius.circular(10),
                  border: Border.all(color: Colors.white12),
                ),
                child: DropdownButtonHideUnderline(
                  child: DropdownButton<String>(
                    value: _modelOptions.any((m) => m.value == _model)
                        ? _model
                        : _modelOptions.first.value,
                    dropdownColor: _card,
                    isExpanded: true,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    items: _modelOptions
                        .map((m) => DropdownMenuItem(
                              value: m.value,
                              child: Text(m.label,
                                  style: const TextStyle(color: Colors.white, fontSize: 13)),
                            ))
                        .toList(),
                    onChanged: (v) {
                      if (v != null) setState(() => _model = v);
                    },
                  ),
                ),
              ),
        const SizedBox(height: 18),

        // ── Aksiyon butonları ─────────────────────────────
        Row(children: [
          Expanded(
            child: FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: _accent,
                padding: const EdgeInsets.symmetric(vertical: 12),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
              ),
              onPressed: _saving ? null : _save,
              icon: _saving
                  ? const SizedBox(
                      width: 14, height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                  : const Icon(Icons.save, size: 15),
              label: Text(_saving ? 'Kaydediliyor...' : 'Kaydet'),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: OutlinedButton.icon(
              style: OutlinedButton.styleFrom(
                foregroundColor: _accent,
                side: const BorderSide(color: _accent),
                padding: const EdgeInsets.symmetric(vertical: 12),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
              ),
              onPressed: (widget.isEnabled && !_testing) ? _test : null,
              icon: _testing
                  ? const SizedBox(
                      width: 14, height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2, color: _accent))
                  : const Icon(Icons.wifi_tethering, size: 15),
              label: const Text('Test Et'),
            ),
          ),
        ]),

        if (widget.isEnabled) ...[
          const SizedBox(height: 10),
          SizedBox(
            width: double.infinity,
            child: TextButton.icon(
              onPressed: _remove,
              style: TextButton.styleFrom(foregroundColor: _red),
              icon: const Icon(Icons.delete_outline, size: 16),
              label: const Text('Kaldır'),
            ),
          ),
        ],

        if (_testOk != null) ...[
          const SizedBox(height: 12),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: (_testOk! ? _green : _red).withOpacity(0.08),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: (_testOk! ? _green : _red).withOpacity(0.3)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _testOk! ? '✅ Bağlantı başarılı!' : '❌ Bağlantı başarısız',
                  style: TextStyle(
                    color: _testOk! ? _green : _red,
                    fontWeight: FontWeight.bold,
                    fontSize: 12,
                  ),
                ),
                if (_testMsg.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  Text(_testMsg,
                      style: const TextStyle(color: Colors.white60, fontSize: 11)),
                ],
              ],
            ),
          ),
        ],
      ],
    );
  }

  Widget _label(String t) => Text(t,
      style: const TextStyle(color: Colors.white54, fontSize: 12, letterSpacing: 0.8));

  InputDecoration _inputDecoration({required String hint, Widget? suffixIcon}) {
    return InputDecoration(
      hintText: hint,
      hintStyle: const TextStyle(color: Colors.white24),
      filled: true,
      fillColor: AetherColors.background,
      border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10), borderSide: BorderSide.none),
      focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: const BorderSide(color: _accent)),
      suffixIcon: suffixIcon,
    );
  }
}
