import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/services/ai_model.dart';

void main() {
  test('model identity is provider plus id', () {
    const a = AiModelOption('model-x', 'X', providerId: 'openai');
    const b = AiModelOption('model-x', 'Another label', providerId: 'openai');
    const c = AiModelOption('model-x', 'X', providerId: 'gemini');
    expect(a, equals(b));
    expect(a, isNot(equals(c)));
  });

  test('capabilities are queryable', () {
    const model = AiModelOption('vision-x', 'Vision X', capabilities: {AiCapability.chat, AiCapability.vision});
    expect(model.supports(AiCapability.chat), isTrue);
    expect(model.supports(AiCapability.vision), isTrue);
    expect(model.supports(AiCapability.audioInput), isFalse);
  });
}
