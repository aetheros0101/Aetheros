import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/state/app_state.dart';
import 'package:aetheros_app/state/workbench_state.dart';

void main() {
  test('status bar toggle via workbench copyWith', () {
    var s = const AppState();
    expect(s.workbench.statusBarVisible, isTrue);
    s = s.copyWith(
      workbench: s.workbench.copyWith(statusBarVisible: false),
    );
    expect(s.workbench.statusBarVisible, isFalse);
  });

  test('density values persist in toJson/fromJson', () {
    final wb = const WorkbenchLayoutState(density: 'compact');
    final restored = WorkbenchLayoutState.fromJson(wb.toJson());
    expect(restored.density, 'compact');
  });
}
