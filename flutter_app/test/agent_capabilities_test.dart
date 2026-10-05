import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/core/agent_capabilities.dart';

void main() {
  test('catalog ids are unique and match the Rust names', () {
    final ids = kAgentCapabilities.map((c) => c.id).toList();
    expect(ids.toSet().length, ids.length);
    // Rust: bridge::api::parse_capability
    expect(
      ids.toSet(),
      {
        'terminal_execution',
        'workspace_read',
        'workspace_write',
        'workspace_mutate',
        'wasm_execution',
        'workflow_execution',
        'remote_execution',
        'ai_reasoning',
      },
    );
  });

  test('terminal is flagged sensitive', () {
    final terminal =
        kAgentCapabilities.firstWhere((c) => c.id == 'terminal_execution');
    expect(terminal.sensitive, isTrue);
  });

  test('workspace write/mutate are sensitive, read is not', () {
    bool sensitive(String id) =>
        kAgentCapabilities.firstWhere((c) => c.id == id).sensitive;
    expect(sensitive('workspace_read'), isFalse);
    expect(sensitive('workspace_write'), isTrue);
    expect(sensitive('workspace_mutate'), isTrue);
  });

  test('workspace ids survive normalize in catalog order', () {
    expect(
      normalizeCapabilities(['workspace_mutate', 'workspace_read', 'workspace_write']),
      ['workspace_read', 'workspace_write', 'workspace_mutate'],
    );
  });

  test('normalize drops unknown ids, dedups and keeps catalog order', () {
    expect(
      normalizeCapabilities(['wasm_execution', 'uçan_halı', 'terminal_execution', 'wasm_execution']),
      ['terminal_execution', 'wasm_execution'],
    );
    expect(normalizeCapabilities(const []), isEmpty);
  });

  test('summary', () {
    expect(capabilitySummary(const []), 'Yetki yok');
    expect(capabilitySummary(['wasm_execution', 'terminal_execution']), 'Terminal, WASM');
    expect(capabilitySummary(['workspace_read']), 'Dosya okuma');
  });
}
