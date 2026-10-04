import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/core/agent_capabilities.dart';
import 'package:aetheros_app/core/chat_to_agent.dart';

void main() {
  test('boşluklar ve satır sonları tek boşluğa iner', () {
    expect(objectiveFromChat('  terminal ile\n\n  ls   çalıştır  '),
        'terminal ile ls çalıştır');
  });

  test('uzun metin kırpılır, kısa metne dokunulmaz', () {
    expect(objectiveFromChat('kısa'), 'kısa');
    final long = 'a' * 800;
    expect(objectiveFromChat(long).length, kMaxObjectiveChars);
    expect(objectiveFromChat('ab cd', maxChars: 3), 'ab');
  });

  test('boş girdi boş döner', () {
    expect(objectiveFromChat('   \n '), '');
  });

  test('önceden seçilen terminal yetkisi katalogdaki gerçek bir id', () {
    expect(kAgentCapabilities.any((c) => c.id == kTerminalCapabilityId), isTrue);
  });
}
