import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/core/agent_capabilities.dart';

void main() {
  test('workbench default capabilities are all known to Rust', () {
    final known = kAgentCapabilities.map((c) => c.id).toSet();
    for (final id in kDefaultWorkbenchCapabilities) {
      expect(known, contains(id), reason: '$id Rust parse_capability bilmiyor');
    }
  });

  test('defaults survive normalization unchanged (nothing silently dropped)', () {
    expect(
      normalizeCapabilities(kDefaultWorkbenchCapabilities).toSet(),
      kDefaultWorkbenchCapabilities.toSet(),
    );
  });

  test('old invalid names are not used', () {
    expect(kDefaultWorkbenchCapabilities, isNot(contains('file_read')));
    expect(kDefaultWorkbenchCapabilities, isNot(contains('file_write')));
  });
}
