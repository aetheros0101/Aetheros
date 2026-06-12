// ============================================================
// flutter_app/lib/services/gemini_service.dart
// Sprint 4 — Gemini API Dart servisi
// Doğrudan HTTP (Rust bridge gerektirmez, sadece build-apk)
// ============================================================

import 'dart:convert';
import 'package:http/http.dart' as http;

// ── Mesaj modeli ──────────────────────────────────────────

enum MessageRole { user, model }

class ChatMessage {
  final MessageRole role;
  final String text;
  final DateTime timestamp;

  const ChatMessage({
    required this.role,
    required this.text,
    required this.timestamp,
  });
}

// ── Gemini servisi ────────────────────────────────────────

class GeminiService {
  static const _base =
      'https://generativelanguage.googleapis.com/v1beta/models';

  // AetherOS varsayılan sistem promptu
  static const systemPrompt =
      'Sen AetherOS\'un yapay zeka asistanısın. '
      'AetherOS, WASM tabanlı bir mobil otomasyon runtime platformudur. '
      'WASM modülleri, WebAssembly Text (WAT) formatı, workflow JSON tanımları, '
      'görev yönetimi ve otomasyon pipeline\'ları konularında uzman yardım sağla. '
      'Yanıtlarını Türkçe ver. Kod örneklerinde WAT veya JSON formatını kullan.';

  /// Tek mesaj gönder.
  static Future<String> chat({
    required String prompt,
    required String apiKey,
    String model = 'gemini-2.0-flash',
    String? system,
    List<ChatMessage> history = const [],
  }) async {
    final url = Uri.parse('$_base/$model:generateContent?key=$apiKey');

    final contents = <Map<String, dynamic>>[];

    // Geçmiş mesajları ekle
    for (final msg in history) {
      contents.add({
        'role': msg.role == MessageRole.user ? 'user' : 'model',
        'parts': [{'text': msg.text}],
      });
    }

    // Yeni mesajı ekle
    contents.add({
      'role': 'user',
      'parts': [{'text': prompt}],
    });

    final body = <String, dynamic>{
      'contents': contents,
      'generationConfig': {
        'temperature': 0.7,
        'maxOutputTokens': 2048,
        'topP': 0.9,
      },
    };

    // Sistem promptu
    final sys = system ?? systemPrompt;
    if (sys.isNotEmpty) {
      body['system_instruction'] = {
        'parts': [{'text': sys}],
      };
    }

    final response = await http.post(
      url,
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(body),
    ).timeout(const Duration(seconds: 30));

    return _parseResponse(response);
  }

  /// Workflow JSON üret.
  static Future<String> generateWorkflow({
    required String description,
    required String apiKey,
    String model = 'gemini-2.0-flash',
  }) async {
    const system =
        'Sen bir AetherOS workflow uzmanısın. '
        'Verilen açıklamaya göre geçerli AetherOS workflow JSON üret. '
        'SADECE JSON döndür, açıklama veya markdown ekleme. '
        'Şema: {"name":string, "version":"1.0.0", "entrypoint":string, '
        '"steps":[{"id":string, "type":"wasm_call", "function":string, '
        '"depends_on"?:string[], "input"?:{}}]}';

    return chat(
      prompt: description,
      apiKey: apiKey,
      model: model,
      system: system,
    );
  }

  /// WAT kodu üret.
  static Future<String> generateWat({
    required String description,
    required String apiKey,
    String model = 'gemini-2.0-flash',
  }) async {
    const system =
        'Sen bir WebAssembly Text (WAT) uzmanısın. '
        'Verilen açıklamaya göre geçerli WAT kodu üret. '
        'SADECE WAT kodu döndür, açıklama veya markdown ekleme. '
        'Başlangıç: (module ... ) yapısını kullan. '
        'Her zaman "run" adlı bir export fonksiyonu ekle.';

    return chat(
      prompt: description,
      apiKey: apiKey,
      model: model,
      system: system,
    );
  }

  /// API anahtarını test et.
  static Future<({bool ok, String message})> testApiKey(
    String apiKey, {
    String model = 'gemini-2.0-flash',
  }) async {
    try {
      final result = await chat(
        prompt: 'Merhaba! AetherOS asistanı mısın?',
        apiKey: apiKey,
        model: model,
        history: [],
      );
      return (ok: true, message: result.substring(0, result.length.clamp(0, 80)));
    } on GeminiException catch (e) {
      return (ok: false, message: e.message);
    } catch (e) {
      return (ok: false, message: e.toString());
    }
  }

  // ── İç yardımcılar ──────────────────────────────────────

  static String _parseResponse(http.Response response) {
    final decoded = jsonDecode(response.body) as Map<String, dynamic>;

    // Hata kontrolü
    if (decoded.containsKey('error')) {
      final err = decoded['error'] as Map<String, dynamic>;
      final code    = err['code'] as int? ?? 0;
      final message = err['message'] as String? ?? 'Bilinmeyen hata';
      throw GeminiException(code: code, message: _tr(code, message));
    }

    final candidates = decoded['candidates'] as List<dynamic>?;
    if (candidates == null || candidates.isEmpty) {
      throw const GeminiException(code: 0, message: 'Yanıt boş geldi');
    }

    final content  = (candidates.first as Map)['content'] as Map?;
    final parts    = content?['parts'] as List?;
    final text     = (parts?.first as Map?)?['text'] as String?;

    if (text == null || text.isEmpty) {
      throw const GeminiException(
          code: 0, message: 'Model yanıt üretemedi');
    }

    return text;
  }

  static String _tr(int code, String msg) => switch (code) {
    400 => 'Geçersiz istek. API anahtarı veya model adını kontrol et.',
    401 => 'Kimlik doğrulama hatası. API anahtarı geçersiz.',
    403 => 'Erişim reddedildi. API anahtarı bu modeli desteklemiyor olabilir.',
    429 => 'İstek limiti aşıldı. Biraz bekleyip tekrar dene.',
    500 => 'Gemini sunucu hatası. Daha sonra tekrar dene.',
    503 => 'Gemini servisi geçici olarak kullanılamıyor.',
    _   => msg,
  };
}

// ── İstisna sınıfı ────────────────────────────────────────

class GeminiException implements Exception {
  final int    code;
  final String message;
  const GeminiException({required this.code, required this.message});
  @override
  String toString() => 'GeminiException($code): $message';
}
