import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/application/services/stub_workspace_service.dart';

void main() {
  late StubWorkspaceService ws;

  setUp(() {
    ws = StubWorkspaceService();
  });

  tearDown(() => ws.dispose());

  test('successful search returns hits', () async {
    final hits = await ws.search('AetherOS');
    expect(hits, isNotEmpty);
    expect(hits.first.path, isNotEmpty);
  });

  test('empty query returns empty or all-handled without throw', () async {
    final hits = await ws.search('');
    expect(hits, isA<List>());
  });

  test('no match returns empty list', () async {
    final hits = await ws.search('zzz_no_such_token_xyz');
    expect(hits, isEmpty);
  });

  test('listFiles respects query filter', () async {
    final all = await ws.listFiles();
    final filtered = await ws.listFiles(query: 'main');
    expect(all, isNotEmpty);
    expect(filtered.every((p) => p.toLowerCase().contains('main')), isTrue);
  });
}
