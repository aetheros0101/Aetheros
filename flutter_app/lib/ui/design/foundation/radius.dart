import 'package:flutter/material.dart';

/// AetherOS Foundation — Border Radius Scale
///
/// IDE-oriented: tighter corners. Large radii (20+) reserved for
/// onboarding / marketing surfaces only — never for workbench panels.
abstract final class AetherRadius {
  static const double none = 0;
  static const double xxs = 2;
  static const double xs = 4;
  static const double sm = 6;
  static const double md = 8;
  static const double lg = 10;
  static const double xl = 12;
  static const double xxl = 16;

  /// Full pill — chips, badges, status pills only.
  static const double full = 9999;

  // ─── Semantic ────────────────────────────────────────────────────────────

  /// Workbench panels, sidebars, editor groups — sharp or near-sharp.
  static const double panel = none;

  /// Tabs, toolbar buttons.
  static const double tab = xs;

  /// Inputs, compact controls.
  static const double input = xs;

  /// Buttons in chrome.
  static const double button = xs;

  /// Cards inside panels (settings sections, agent cards).
  static const double card = sm;

  /// Dialogs, command palette, floating panels.
  static const double dialog = md;

  /// Menus, popovers, tooltips.
  static const double menu = sm;

  /// Chips / status pills.
  static const double chip = full;

  // ─── BorderRadius helpers ────────────────────────────────────────────────

  static BorderRadius circular(double r) => BorderRadius.circular(r);

  static final BorderRadius noneR = BorderRadius.zero;
  static final BorderRadius xxsR = BorderRadius.circular(xxs);
  static final BorderRadius xsR = BorderRadius.circular(xs);
  static final BorderRadius smR = BorderRadius.circular(sm);
  static final BorderRadius mdR = BorderRadius.circular(md);
  static final BorderRadius lgR = BorderRadius.circular(lg);
  static final BorderRadius xlR = BorderRadius.circular(xl);
  static final BorderRadius xxlR = BorderRadius.circular(xxl);
  static final BorderRadius fullR = BorderRadius.circular(full);

  static final BorderRadius panelR = noneR;
  static final BorderRadius tabR = xsR;
  static final BorderRadius inputR = xsR;
  static final BorderRadius buttonR = xsR;
  static final BorderRadius cardR = smR;
  static final BorderRadius dialogR = mdR;
  static final BorderRadius menuR = smR;
  static final BorderRadius chipR = fullR;
}
