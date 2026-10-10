import 'package:flutter/material.dart';

/// AetherOS Foundation — Spacing Scale
///
/// 4-pt base grid. Workbench panels use tighter tokens than typical mobile apps.
abstract final class AetherSpacing {
  static const double none = 0;
  static const double xxs = 2;
  static const double xs = 4;
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
  static const double xl = 20;
  static const double xxl = 24;
  static const double xxxl = 32;
  static const double huge = 48;

  // ─── Workbench semantic ──────────────────────────────────────────────────

  /// Gap between panel chrome elements (toolbar icons, tab close buttons).
  static const double chromeGap = xs;

  /// Internal padding of toolbars / activity bar items.
  static const double toolbarPadding = sm;

  /// Panel content inset.
  static const double panelInset = sm;

  /// Editor group gap (split editors).
  static const double editorGap = xxs;

  /// Tree / explorer row indent step.
  static const double treeIndent = lg;

  /// Form field vertical gap.
  static const double formGap = md;

  /// Dialog content padding.
  static const double dialogPadding = lg;

  // ─── EdgeInsets helpers ──────────────────────────────────────────────────

  static const EdgeInsets zero = EdgeInsets.zero;

  static EdgeInsets all(double v) => EdgeInsets.all(v);

  static EdgeInsets symmetric({double h = 0, double v = 0}) =>
      EdgeInsets.symmetric(horizontal: h, vertical: v);

  static EdgeInsets only({
    double left = 0,
    double top = 0,
    double right = 0,
    double bottom = 0,
  }) =>
      EdgeInsets.only(left: left, top: top, right: right, bottom: bottom);

  static const EdgeInsets toolbar = EdgeInsets.symmetric(
    horizontal: toolbarPadding,
    vertical: xs,
  );

  static const EdgeInsets panel = EdgeInsets.all(panelInset);

  static const EdgeInsets dialog = EdgeInsets.all(dialogPadding);

  static const EdgeInsets treeRow = EdgeInsets.symmetric(
    horizontal: sm,
    vertical: xxs,
  );
}
