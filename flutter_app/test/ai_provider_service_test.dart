// ============================================================
// test/ai_provider_service_test.dart
//
// Önceki durum: flutter_app'te tek bir test dosyası yoktu (widget veya
// unit). Bu dosya, Rust bridge'e veya bir widget ağacına ihtiyaç
// duymayan saf Dart mantığını kapsayan ilk test setini ekler:
// `aiProviders` sabit listesi ve `aiProviderById` lookup fonksiyonu.
//
// Servisin geri kalanı (save/load/rehydrateFromStorage vb.)
// flutter_secure_storage + shared_preferences + Rust FFI'a bağımlı;
// bunlar için sahte (fake) storage/bridge implementasyonları
// eklenmeden anlamlı bir şekilde test edilemezler — bir sonraki adım
// olarak bırakıldı (bkz. CHANGES.md).
// ============================================================

import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/services/ai_provider_service.dart';

void main() {
  group('aiProviders (static metadata)', () {
    test('listede en az bir provider var', () {
      expect(aiProviders, isNotEmpty);
    });

    test('her provider benzersiz bir id kullanıyor', () {
      final ids = aiProviders.map((p) => p.id).toList();
      expect(ids.toSet().length, ids.length,
          reason: 'Tekrarlayan provider id bulundu: $ids');
    });

    test('her provider için id, displayName ve defaultModel boş değil', () {
      for (final p in aiProviders) {
        expect(p.id.isNotEmpty, isTrue, reason: 'boş id');
        expect(p.displayName.isNotEmpty, isTrue,
            reason: '${p.id}: boş displayName');
        expect(p.defaultModel.isNotEmpty, isTrue,
            reason: '${p.id}: boş defaultModel');
      }
    });

    test('needsBaseUrl true olan provider için defaultBaseUrl boş olmamalı',
        () {
      for (final p in aiProviders) {
        if (p.needsBaseUrl) {
          expect(p.defaultBaseUrl.isNotEmpty, isTrue,
              reason: '${p.id}: needsBaseUrl=true ama defaultBaseUrl boş');
        }
      }
    });

    test('anthropic provider tanımlı ve API key gerektiriyor', () {
      final anthropic = aiProviders.firstWhere((p) => p.id == 'anthropic');
      expect(anthropic.needsApiKey, isTrue);
    });

    test('ollama provider tanımlı ve API key gerektirmiyor', () {
      final ollama = aiProviders.firstWhere((p) => p.id == 'ollama');
      expect(ollama.needsApiKey, isFalse);
    });
  });

  group('aiProviderById', () {
    test('bilinen bir id için doğru provider döner', () {
      for (final expected in aiProviders) {
        final found = aiProviderById(expected.id);
        expect(found.id, expected.id);
      }
    });

    test('bilinmeyen bir id için listedeki ilk provider\'a düşer (throw etmez)',
        () {
      expect(() => aiProviderById('bilinmeyen-provider-xyz'),
          returnsNormally);
      expect(aiProviderById('bilinmeyen-provider-xyz').id,
          aiProviders.first.id);
    });

    test('boş string id için de throw etmez', () {
      expect(() => aiProviderById(''), returnsNormally);
    });
  });
}
