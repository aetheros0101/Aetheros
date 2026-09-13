import 'dart:convert';

import 'package:http/http.dart' as http;

import 'ai_model.dart';

/// Runtime model discovery for cloud providers.
///
/// Model names are intentionally NOT hard-coded here. Providers expose their
/// own model catalogs and may add/deprecate models independently of the app
/// release cycle.
class AiModelCatalog {
  AiModelCatalog._();

  static Future<List<AiModelOption>> list({
    required String providerId,
    required String apiKey,
    Duration timeout = const Duration(seconds: 12),
  }) async {
    if (apiKey.trim().isEmpty) {
      throw ArgumentError('API anahtarı gerekli');
    }

    switch (providerId) {
      case 'openai':
        return _openAi(apiKey.trim(), timeout);
      case 'anthropic':
        return _anthropic(apiKey.trim(), timeout);
      case 'gemini':
        return _gemini(apiKey.trim(), timeout);
      default:
        throw UnsupportedError('Dinamik model keşfi desteklenmiyor: $providerId');
    }
  }

  static Future<List<AiModelOption>> _openAi(
    String key,
    Duration timeout,
  ) async {
    final result = <AiModelOption>[];
    String? after;

    for (var page = 0; page < 10; page++) {
      final query = <String, String>{'limit': '100'};
      if (after != null) query['after'] = after!;

      final response = await http
          .get(
            Uri.https('api.openai.com', '/v1/models', query),
            headers: {'Authorization': 'Bearer $key'},
          )
          .timeout(timeout);

      _check(response, 'OpenAI');
      final body = _jsonObject(response);
      final data = (body['data'] as List?) ?? const [];

      for (final item in data.whereType<Map>()) {
        final id = item['id']?.toString() ?? '';
        if (_isOpenAiChatCandidate(id)) {
          result.add(AiModelOption(id, id, providerId: 'openai', capabilities: _openAiCapabilities(id)));
        }
      }

      final hasMore = body['has_more'] == true;
      final lastId = data.isNotEmpty
          ? data.last is Map
              ? (data.last as Map)['id']?.toString()
              : null
          : null;
      if (!hasMore || lastId == null || lastId.isEmpty) break;
      after = lastId;
    }

    return _dedupeAndSort(result);
  }

  static Set<AiCapability> _openAiCapabilities(String id) {
    final lower = id.toLowerCase();
    final result = <AiCapability>{AiCapability.chat};
    if (lower.contains('vision') || lower.contains('gpt-4') || lower.contains('gpt-5')) {
      result.add(AiCapability.vision);
    }
    result.add(AiCapability.tools);
    return result;
  }

  static bool _isOpenAiChatCandidate(String id) {
    if (id.isEmpty) return false;
    final lower = id.toLowerCase();
    // No provider-specific positive allowlist: newly introduced model IDs
    // should surface automatically. Only known non-chat families are hidden.
    const excluded = [
      'embedding', 'moderation', 'whisper', 'tts', 'dall-e', 'image',
      'search', 'transcribe', 'audio', 'realtime', 'instruct',
    ];
    return !excluded.any(lower.contains);
  }

  static Future<List<AiModelOption>> _anthropic(
    String key,
    Duration timeout,
  ) async {
    final result = <AiModelOption>[];
    String? afterId;

    for (var page = 0; page < 10; page++) {
      final query = <String, String>{'limit': '1000'};
      if (afterId != null) query['after_id'] = afterId!;

      final response = await http
          .get(
            Uri.https('api.anthropic.com', '/v1/models', query),
            headers: {
              'x-api-key': key,
              'anthropic-version': '2023-06-01',
            },
          )
          .timeout(timeout);

      _check(response, 'Anthropic');
      final body = _jsonObject(response);
      final data = (body['data'] as List?) ?? const [];

      for (final item in data.whereType<Map>()) {
        final id = item['id']?.toString() ?? '';
        if (id.isEmpty) continue;
        final label = item['display_name']?.toString();
        result.add(AiModelOption(id, label?.trim().isNotEmpty == true ? label! : id, providerId: 'anthropic', capabilities: const {AiCapability.chat, AiCapability.tools}));
      }

      final hasMore = body['has_more'] == true;
      final lastId = body['last_id']?.toString();
      if (!hasMore || lastId == null || lastId.isEmpty) break;
      afterId = lastId;
    }

    return _dedupeAndSort(result);
  }

  static Future<List<AiModelOption>> _gemini(
    String key,
    Duration timeout,
  ) async {
    final result = <AiModelOption>[];
    String? pageToken;

    for (var page = 0; page < 10; page++) {
      final query = <String, String>{
        'pageSize': '1000',
      };
      if (pageToken != null) query['pageToken'] = pageToken!;

      final response = await http
          .get(
            Uri.https(
              'generativelanguage.googleapis.com',
              '/v1beta/models',
              query,
            ),
            headers: {'x-goog-api-key': key},
          )
          .timeout(timeout);

      _check(response, 'Gemini');
      final body = _jsonObject(response);
      final data = (body['models'] as List?) ?? const [];

      for (final item in data.whereType<Map>()) {
        final supported = (item['supportedGenerationMethods'] as List?)
                ?.map((e) => e.toString())
                .toList() ??
            const <String>[];
        if (!supported.contains('generateContent')) continue;

        final rawName = item['name']?.toString() ?? '';
        final id = rawName.startsWith('models/')
            ? rawName.substring('models/'.length)
            : rawName;
        if (id.isEmpty) continue;

        final label = item['displayName']?.toString();
        final inputLimit = (item['inputTokenLimit'] as num?)?.toInt();
        final outputLimit = (item['outputTokenLimit'] as num?)?.toInt();
        result.add(AiModelOption(
          id,
          label?.trim().isNotEmpty == true ? label! : id,
          providerId: 'gemini',
          contextWindow: inputLimit,
          maxOutputTokens: outputLimit,
          capabilities: const {AiCapability.chat},
        ));
      }

      final next = body['nextPageToken']?.toString();
      if (next == null || next.isEmpty) break;
      pageToken = next;
    }

    return _dedupeAndSort(result);
  }

  static List<AiModelOption> _dedupeAndSort(List<AiModelOption> values) {
    final byId = <String, AiModelOption>{};
    for (final value in values) {
      byId[value.value] = value;
    }

    final result = byId.values.toList()
      ..sort((a, b) => a.label.toLowerCase().compareTo(b.label.toLowerCase()));
    return result;
  }

  static Map<String, dynamic> _jsonObject(http.Response response) {
    final decoded = jsonDecode(response.body);
    if (decoded is! Map<String, dynamic>) {
      throw const FormatException('Model API beklenmeyen JSON döndürdü');
    }
    return decoded;
  }

  static void _check(http.Response response, String provider) {
    if (response.statusCode >= 200 && response.statusCode < 300) return;

    String detail = '';
    try {
      final decoded = jsonDecode(response.body);
      if (decoded is Map) {
        final error = decoded['error'];
        if (error is Map) {
          detail = error['message']?.toString() ?? '';
        } else {
          detail = error?.toString() ?? '';
        }
      }
    } catch (_) {
      // Keep the user-facing error generic if the provider did not return JSON.
    }

    final suffix = detail.isEmpty ? '' : ': $detail';
    throw StateError('$provider model listesi alınamadı (${response.statusCode})$suffix');
  }
}
