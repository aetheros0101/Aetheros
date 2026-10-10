import 'package:flutter/material.dart';

import '../foundation/colors.dart';

/// AetherOS Semantic — Surface Hierarchy
///
/// Workbench visual structure is defined by surface roles, not elevation.
/// Each role maps to a neutral step so panels separate via contrast + border.
///
/// ```
/// canvas  → deepest workbench background
/// chrome  → activity bar, title bar
/// sidebar → primary / secondary sidebar
/// panel   → bottom panel, side panels
/// editor  → editor group background
/// tab     → tab strip / inactive tab
/// tabActive
/// input   → text fields, search boxes
/// elevated→ floating panels resting on canvas
/// overlay → modal scrim
/// popup   → menus, tooltips, command palette
/// ```
abstract final class AetherSurfaces {
  /// Deepest workbench background (behind everything).
  static const Color canvas = AetherColorPrimitives.neutral0;

  /// Activity bar, window bar, status bar chrome.
  static const Color chrome = AetherColorPrimitives.neutral50;

  /// Primary / secondary sidebar.
  static const Color sidebar = AetherColorPrimitives.neutral100;

  /// Bottom panel, auxiliary panels.
  static const Color panel = AetherColorPrimitives.neutral100;

  /// Editor group background.
  static const Color editor = AetherColorPrimitives.neutral150;

  /// Inactive tab / tab strip.
  static const Color tab = AetherColorPrimitives.neutral100;

  /// Active tab.
  static const Color tabActive = AetherColorPrimitives.neutral200;

  /// Text inputs, search, comboboxes.
  static const Color input = AetherColorPrimitives.neutral200;

  /// Cards / sections inside panels (slightly raised via contrast).
  static const Color elevated = AetherColorPrimitives.neutral250;

  /// Hover wash target (applied as overlay, not a full surface).
  static const Color hover = AetherColorPrimitives.neutral300;

  /// Selected / active row surface.
  static const Color active = AetherColorPrimitives.neutral350;

  /// Floating UI: menus, popovers, command palette body.
  static const Color popup = AetherColorPrimitives.neutral300;

  /// Modal dialog body.
  static const Color dialog = AetherColorPrimitives.neutral250;

  /// Full-screen / modal scrim.
  static final Color overlay =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral0, 0.72);

  /// Tooltip background.
  static const Color tooltip = AetherColorPrimitives.neutral400;
}

/// Text colors on workbench surfaces.
abstract final class AetherTextColors {
  /// Primary readable text.
  static const Color primary = AetherColorPrimitives.neutral1000;

  /// Secondary / supporting text.
  static final Color secondary =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.70);

  /// Tertiary / placeholder / dim metadata.
  static final Color tertiary =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.45);

  /// Disabled.
  static final Color disabled =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.28);

  /// Accent-colored text (links, active labels).
  static const Color accent = AetherColorPrimitives.accent400;

  /// Inverse (on light accent fills).
  static const Color inverse = AetherColorPrimitives.neutral0;
}

/// Accent usage — intentional emphasis only.
abstract final class AetherAccent {
  /// Primary accent (focus, active item, agent activity, key action).
  static const Color primary = AetherColorPrimitives.accent400;

  /// Hover / brighter accent.
  static const Color hover = AetherColorPrimitives.accent500;

  /// Muted accent fill (selected sidebar item background).
  static const Color muted = AetherColorPrimitives.accent100;

  /// Soft accent container.
  static const Color container = AetherColorPrimitives.accent50;

  /// On-accent text/icon.
  static const Color onAccent = AetherColorPrimitives.neutral1000;
}
