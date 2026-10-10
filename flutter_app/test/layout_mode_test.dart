import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/ui/shell/responsive.dart';
import 'package:aetheros_app/ui/design/foundation/breakpoints.dart'
    show AetherBreakpoints;

void main() {
  test('layoutModeForWidth breakpoints', () {
    expect(layoutModeForWidth(360), AetherLayoutMode.mobile);
    expect(layoutModeForWidth(390), AetherLayoutMode.mobile);
    expect(layoutModeForWidth(599), AetherLayoutMode.mobile);
    expect(layoutModeForWidth(600), AetherLayoutMode.tablet);
    expect(layoutModeForWidth(768), AetherLayoutMode.tablet);
    expect(layoutModeForWidth(899), AetherLayoutMode.tablet);
    expect(layoutModeForWidth(900), AetherLayoutMode.workbench);
    expect(layoutModeForWidth(1024), AetherLayoutMode.workbench);
    expect(layoutModeForWidth(1199), AetherLayoutMode.workbench);
    expect(layoutModeForWidth(1200), AetherLayoutMode.wideWorkbench);
    expect(layoutModeForWidth(1440), AetherLayoutMode.wideWorkbench);
    expect(layoutModeForWidth(1920), AetherLayoutMode.wideWorkbench);
  });

  test('chrome visibility flags', () {
    expect(AetherLayoutMode.mobile.showActivityBar, isFalse);
    expect(AetherLayoutMode.mobile.showPrimarySidebar, isFalse);
    expect(AetherLayoutMode.mobile.useMobileNav, isTrue);

    expect(AetherLayoutMode.tablet.showActivityBar, isTrue);
    expect(AetherLayoutMode.tablet.showPrimarySidebar, isTrue);
    expect(AetherLayoutMode.tablet.showSecondarySidebar, isFalse);

    expect(AetherLayoutMode.workbench.showSecondarySidebar, isTrue);
    expect(AetherLayoutMode.wideWorkbench.showSecondarySidebar, isTrue);
  });

  test('AetherBreakpoints thresholds match shell helper', () {
    expect(AetherBreakpoints.compactMax, 600);
    expect(AetherBreakpoints.mediumMax, 900);
    expect(AetherBreakpoints.expandedMax, 1200);
  });
}
