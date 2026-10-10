import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/state/editor_state.dart';

void main() {
  test('openFile activates existing tab', () {
    var s = const EditorState()
        .openFile('a.dart')
        .openFile('b.dart');
    expect(s.activeGroup.tabs.length, 2);
    s = s.openFile('a.dart');
    expect(s.activeGroup.activeTab?.path, 'a.dart');
    expect(s.activeGroup.tabs.length, 2);
  });

  test('closeTab before active shifts index left', () {
    var s = const EditorState()
        .openFile('a.dart')
        .openFile('b.dart')
        .openFile('c.dart');
    // active is c (index 2); close a (index 0)
    s = s.closeTab(0, 0);
    expect(s.activeGroup.tabs.map((t) => t.path).toList(), ['b.dart', 'c.dart']);
    expect(s.activeGroup.activeTab?.path, 'c.dart');
  });

  test('close active middle tab selects next', () {
    var s = const EditorState()
        .openFile('a.dart')
        .openFile('b.dart')
        .openFile('c.dart');
    // active is c; activate b then close b
    s = s.copyWith(
      groups: [
        s.activeGroup.copyWith(activeTabIndex: 1),
      ],
    );
    s = s.closeTab(0, 1);
    expect(s.activeGroup.tabs.map((t) => t.path).toList(), ['a.dart', 'c.dart']);
    expect(s.activeGroup.activeTabIndex, 1);
    expect(s.activeGroup.activeTab?.path, 'c.dart');
  });

  test('close last remaining tab yields empty group', () {
    var s = const EditorState().openFile('only.dart');
    s = s.closeActiveTab();
    expect(s.activeGroup.tabs, isEmpty);
  });

  test('nextTab and previousTab wrap', () {
    var s = const EditorState()
        .openFile('a.dart')
        .openFile('b.dart')
        .openFile('c.dart');
    s = s.nextTab();
    expect(s.activeGroup.activeTab?.path, 'a.dart');
    s = s.previousTab();
    expect(s.activeGroup.activeTab?.path, 'c.dart');
  });

  test('isPathOpen after close', () {
    var s = const EditorState().openFile('a.dart').openFile('b.dart');
    expect(s.isPathOpen('a.dart'), isTrue);
    s = s.closeTab(0, 0);
    expect(s.isPathOpen('a.dart'), isFalse);
    expect(s.isPathOpen('b.dart'), isTrue);
  });

  test('preview tab replaces previous preview', () {
    var s = const EditorState()
        .openFile('a.dart', preview: true)
        .openFile('b.dart', preview: true);
    expect(s.activeGroup.tabs.length, 1);
    expect(s.activeGroup.activeTab?.path, 'b.dart');
  });
}
