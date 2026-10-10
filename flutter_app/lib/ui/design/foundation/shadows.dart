import 'package:flutter/material.dart';

/// AetherOS Foundation — Shadow Tokens
///
/// Workbench panels use surface contrast + 1px borders for hierarchy.
/// Shadows are reserved for floating UI only:
/// dialogs, menus, command palette, tooltips, hover cards.
abstract final class AetherShadows {
  static const List<BoxShadow> none = [];

  /// Tooltip / small popover.
  static const List<BoxShadow> sm = [
    BoxShadow(
      color: Color(0x40000000),
      offset: Offset(0, 2),
      blurRadius: 6,
    ),
  ];

  /// Dropdown menus, context menus.
  static const List<BoxShadow> md = [
    BoxShadow(
      color: Color(0x52000000),
      offset: Offset(0, 4),
      blurRadius: 12,
    ),
    BoxShadow(
      color: Color(0x24000000),
      offset: Offset(0, 1),
      blurRadius: 3,
    ),
  ];

  /// Command palette, dialogs.
  static const List<BoxShadow> lg = [
    BoxShadow(
      color: Color(0x66000000),
      offset: Offset(0, 8),
      blurRadius: 24,
    ),
    BoxShadow(
      color: Color(0x33000000),
      offset: Offset(0, 2),
      blurRadius: 6,
    ),
  ];

  /// Modal overlays with strong separation.
  static const List<BoxShadow> xl = [
    BoxShadow(
      color: Color(0x80000000),
      offset: Offset(0, 12),
      blurRadius: 40,
    ),
    BoxShadow(
      color: Color(0x40000000),
      offset: Offset(0, 4),
      blurRadius: 12,
    ),
  ];

  // ─── Semantic aliases ────────────────────────────────────────────────────

  static const List<BoxShadow> tooltip = sm;
  static const List<BoxShadow> menu = md;
  static const List<BoxShadow> dropdown = md;
  static const List<BoxShadow> commandPalette = lg;
  static const List<BoxShadow> dialog = lg;
  static const List<BoxShadow> modal = xl;

  /// Soft accent glow for focused floating elements (use sparingly).
  static const List<BoxShadow> focusGlow = [
    BoxShadow(
      color: Color(0x406C63FF),
      offset: Offset(0, 0),
      blurRadius: 8,
      spreadRadius: 0,
    ),
  ];
}
