import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/ui/shell/responsive.dart';

void main() {
  for (final width in [360.0, 390.0, 600.0, 768.0, 1024.0, 1280.0, 1440.0, 1920.0]) {
    testWidgets('layout mode stable at ${width.toInt()}px', (tester) async {
      late AetherLayoutMode mode;
      await tester.pumpWidget(
        MediaQuery(
          data: MediaQueryData(size: Size(width, 800)),
          child: Builder(
            builder: (context) {
              mode = layoutModeOf(context);
              return const SizedBox();
            },
          ),
        ),
      );
      expect(mode, layoutModeForWidth(width));
      if (width < 600) {
        expect(mode.useMobileNav, isTrue);
        expect(mode.showActivityBar, isFalse);
      } else {
        expect(mode.showActivityBar, isTrue);
      }
    });
  }
}
