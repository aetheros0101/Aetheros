// ============================================================
// AetherOS — AI provider configuration
//
// Model selection is runtime-driven:
//   - OpenAI / Anthropic / Gemini: official provider model catalog is queried
//     after the user supplies an API key.
//   - Ollama: models are queried from the configured Ollama host.
//   - No cloud model ID is treated as a permanent application constant.
//
// API keys are stored only in flutter_secure_storage. Model/base-url/active
// provider metadata is stored in SharedPreferences.
// ============================================================

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'ai_model.dart';
import 'ai_model_catalog.dart';

import '../src/rust/api/aetheros.dart' as aether;

// ── Provider metadata ──────────────────────────────────────

class AiProviderInfo {
  final String id;
  final String displayName;
  final bool needsApiKey;
  final bool needsBaseUrl;
  final String defaultModel;
  final String defaultBaseUrl;
  final List<AiModelOption> modelOptions;
  final String helpText;
  final String helpUrl;

  const AiProviderInfo({
    required this.id,
    required this.displayName,
    required this.needsApiKey,
    required this.needsBaseUrl,
    required this.defaultModel,
    this.defaultBaseUrl = '',
    this.modelOptions = const [],
    required this.helpText,
    this.helpUrl = '',
  });
}

const List<AiProviderInfo> aiProviders = [
  AiProviderInfo(
    id: 'anthropic',
    displayName: 'Anthropic Claude',
    needsApiKey: true,
    needsBaseUrl: false,
    defaultModel: '',
    modelOptions: const [],
    helpText: 'API anahtarını gir ve modelleri sağlayıcıdan dinamik olarak getir.',
    helpUrl: 'https://console.anthropic.com',
  ),
  AiProviderInfo(
    id: 'openai',
    displayName: 'OpenAI',
    needsApiKey: true,
    needsBaseUrl: false,
    defaultModel: '',
    modelOptions: const [],
    helpText: 'API anahtarını gir ve modelleri OpenAI hesabından dinamik olarak getir.',
    helpUrl: 'https://platform.openai.com/api-keys',
  ),
  AiProviderInfo(
    id: 'gemini',
    displayName: 'Google Gemini',
    needsApiKey: true,
    needsBaseUrl: false,
    defaultModel: '',
    modelOptions: const [],
    helpText: 'API anahtarını gir ve modelleri Google hesabından dinamik olarak getir.',
    helpUrl: 'https://aistudio.google.com',
  ),
  AiProviderInfo(
    id: 'ollama',
    displayName: 'Ollama (Cihazında/Ağında Local)',
    needsApiKey: false,
    needsBaseUrl: true,
    defaultModel: 'llama3.2',
    defaultBaseUrl: 'http://127.0.0.1:11434',
    modelOptions: [], // sunucudan dinamik çekilir — bkz. listOllamaModels
    helpText: '1. Cihazında veya ağındaki bir makinede `ollama serve` çalıştır\n'
        '2. İstediğin modeli indir: `ollama pull llama3.2`\n'
        '3. Aynı ağdaki başka bir cihazdaysan host\'u o cihazın IP\'siyle gir\n'
        '4. API key gerekmez — tamamen ücretsiz, internet gerektirmez',
    helpUrl: 'https://ollama.com',
  ),
];

AiProviderInfo aiProviderById(String id) =>
    aiProviders.firstWhere((p) => p.id == id, orElse: () => aiProviders.first);

// ── Kayıtlı yapılandırma ────────────────────────────────────

class ConfiguredProvider {
  final String providerId;
  final String? apiKey;
  final String? baseUrl;
  final String model;
  const ConfiguredProvider({
    required this.providerId,
    this.apiKey,
    this.baseUrl,
    required this.model,
  });
}

// ── Servis ───────────────────────────────────────────────────

class AiProviderService {
  static const _secureStorage = FlutterSecureStorage();

  static const _prefActive = 'ai_active_provider';
  static String _secureKeyFor(String id) => 'ai_apikey_$id';
  static String _prefBaseUrl(String id) => 'ai_baseurl_$id';
  static String _prefModel(String id) => 'ai_model_$id';
  static String _prefEnabled(String id) => 'ai_enabled_$id';

  /// Provider'ı yapılandır, kalıcı olarak kaydet ve Rust router'a kaydet.
  /// İlk yapılandırılan provider otomatik aktif olur.
  static Future<void> save({
    required String providerId,
    String? apiKey,
    String? baseUrl,
    String? model,
  }) async {
    final info = aiProviderById(providerId);
    final resolvedModel = (model == null || model.trim().isEmpty)
        ? info.defaultModel
        : model.trim();
    final resolvedBaseUrl = info.needsBaseUrl
        ? ((baseUrl == null || baseUrl.trim().isEmpty)
            ? info.defaultBaseUrl
            : baseUrl.trim())
        : null;
    final resolvedKey = info.needsApiKey ? apiKey?.trim() : null;

    if (info.needsApiKey && (resolvedKey == null || resolvedKey.isEmpty)) {
      throw ArgumentError('${info.displayName} için API anahtarı gerekli');
    }

    // Önce runtime'a uygula. Böylece bridge başarısız olursa cihazda
    // yarım/kullanılamaz bir provider kaydı bırakmayız.
    await aether.configureAiProvider(
      providerId: providerId,
      apiKey: resolvedKey,
      baseUrl: resolvedBaseUrl,
      model: resolvedModel,
    );

    final prefs = await SharedPreferences.getInstance();
    if (info.needsApiKey) {
      await _secureStorage.write(
        key: _secureKeyFor(providerId),
        value: resolvedKey,
      );
    }
    if (resolvedBaseUrl != null) {
      await prefs.setString(_prefBaseUrl(providerId), resolvedBaseUrl);
    }
    await prefs.setString(_prefModel(providerId), resolvedModel);
    await prefs.setBool(_prefEnabled(providerId), true);

    // İlk yapılandırılan provider otomatik aktif olsun.
    final currentActive = await getActiveProviderId();
    if (currentActive == null) {
      await setActive(providerId);
    }
  }

  /// Birden fazla provider yapılandırılmışsa kullanılacak olanı seç.
  static Future<void> setActive(String providerId) async {
    final info = aiProviders.where((p) => p.id == providerId);
    if (info.isEmpty) {
      throw ArgumentError('Bilinmeyen AI provider: $providerId');
    }

    final prefs = await SharedPreferences.getInstance();
    final enabled = prefs.getBool(_prefEnabled(providerId)) ?? false;
    if (!enabled) {
      throw StateError('Provider yapılandırılmadan aktif edilemez: $providerId');
    }

    // Önce Rust router'ı değiştir; başarıdan sonra kalıcı seçimi yaz.
    await aether.setActiveAiProvider(providerId: providerId);
    await prefs.setString(_prefActive, providerId);
  }

  static Future<String?> getActiveProviderId() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_prefActive);
  }

  /// Provider'ı tamamen kaldır (key'i sil, devre dışı bırak).
  static Future<void> remove(String providerId) async {
    await _secureStorage.delete(key: _secureKeyFor(providerId));

    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_prefBaseUrl(providerId));
    await prefs.remove(_prefModel(providerId));
    await prefs.remove(_prefEnabled(providerId));
    if (prefs.getString(_prefActive) == providerId) {
      await prefs.remove(_prefActive);
    }

    try {
      await aether.removeAiProvider(providerId: providerId);
    } catch (e) {
      debugPrint('removeAiProvider hatası ($providerId): $e');
    }
  }

  /// Kullanıcının yapılandırdığı (enabled) provider id'leri.
  static Future<List<String>> enabledProviderIds() async {
    final prefs = await SharedPreferences.getInstance();
    return aiProviders
        .map((p) => p.id)
        .where((id) => prefs.getBool(_prefEnabled(id)) ?? false)
        .toList();
  }

  /// Bir provider'ın kayıtlı yapılandırmasını oku (Ayarlar ekranını
  /// doldurmak ve rehydrate için).
  static Future<ConfiguredProvider?> loadConfig(String providerId) async {
    final prefs = await SharedPreferences.getInstance();
    final enabled = prefs.getBool(_prefEnabled(providerId)) ?? false;
    if (!enabled) return null;

    final info = aiProviderById(providerId);
    final apiKey = info.needsApiKey
        ? await _secureStorage.read(key: _secureKeyFor(providerId))
        : null;
    final baseUrl = prefs.getString(_prefBaseUrl(providerId));
    final model = prefs.getString(_prefModel(providerId)) ?? info.defaultModel;

    return ConfiguredProvider(
      providerId: providerId,
      apiKey: apiKey,
      baseUrl: baseUrl,
      model: model,
    );
  }

  /// Uygulama açılışında initializeRuntime() BAŞARILI olduktan sonra
  /// çağrılır (bkz. main.dart). Flutter'da kayıtlı tüm provider'ları
  /// Rust router'a yeniden kaydeder — Rust router in-memory olduğu
  /// için her runtime başlangıcında boştur.
  static Future<void> rehydrateFromStorage() async {
    final ids = await enabledProviderIds();

    for (final id in ids) {
      final cfg = await loadConfig(id);
      if (cfg == null) continue;
      try {
        await aether.configureAiProvider(
          providerId: cfg.providerId,
          apiKey: cfg.apiKey,
          baseUrl: cfg.baseUrl,
          model: cfg.model,
        );
      } catch (e) {
        debugPrint('AI provider rehydrate hatası ($id): $e');
      }
    }

    final active = await getActiveProviderId();
    if (active != null && ids.contains(active)) {
      try {
        await aether.setActiveAiProvider(providerId: active);
      } catch (e) {
        debugPrint('AI aktif provider rehydrate hatası: $e');
      }
    }
  }

  /// Cloud provider modellerini sağlayıcının resmi model kataloğundan getir.
  ///
  /// API anahtarı yalnızca bu istek için kullanılır; kalıcı kayıt yine
  /// flutter_secure_storage üzerinden yapılır.
  static Future<List<AiModelOption>> discoverModels({
    required String providerId,
    required String apiKey,
  }) {
    return AiModelCatalog.list(providerId: providerId, apiKey: apiKey);
  }

  /// Ollama sunucusunda yüklü modelleri listele (Ayarlar dropdown'ı için).
  static Future<List<String>> listOllamaModels(String baseUrl) {
    return aether.listOllamaModels(baseUrl: baseUrl);
  }

  /// "Bağlantıyı Test Et" — kayıtlı provider'a küçük bir istek gönderir.
  static Future<String> testProvider(String providerId) {
    return aether.testAiProvider(providerId: providerId);
  }

  /// Genel amaçlı sohbet isteği — kullanıcının aktif ettiği provider
  /// üzerinden çalışır (AI Chat ekranı tarafından kullanılır).
  static Future<String> chat({
    required String prompt,
    String? systemPrompt,
    int maxTokens = 1024,
  }) {
    return aether.aiChat(
      prompt: prompt,
      systemPrompt: systemPrompt,
      maxTokens: maxTokens,
    );
  }
}
