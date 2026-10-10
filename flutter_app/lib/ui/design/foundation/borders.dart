import 'package:flutter/material.dart';

import 'colors.dart';
import 'radius.dart';

/// AetherOS Foundation — Border Tokens
///
/// Workbench hierarchy is driven primarily by 1px borders + surface contrast,
/// not shadows. Shadows are reserved for floating UI.
abstract final class AetherBorders {
  static const double thin = 1.0;
  static const double medium = 1.5;
  static const double thick = 2.0;

  // ─── Sides ───────────────────────────────────────────────────────────────

  static const BorderSide none = BorderSide.none;

  /// Default panel / section separator.
  static BorderSide get subtle => BorderSide(
        color: AetherColorPrimitives.withAlpha(
          AetherColorPrimitives.neutral1000,
          0.08,
        ),
        width: thin,
      );

  /// Stronger outline for interactive containers at rest.
  static BorderSide get standard => BorderSide(
        color: AetherColorPrimitives.withAlpha(
          AetherColorPrimitives.neutral1000,
          0.14,
        ),
        width: thin,
      );

  /// Focus ring — accent, never neutral.
  static BorderSide get focus => const BorderSide(
        color: AetherColorPrimitives.accent400,
        width: medium,
      );

  static BorderSide get error => const BorderSide(
        color: AetherColorPrimitives.red400,
        width: medium,
      );

  static BorderSide get success => const BorderSide(
        color: AetherColorPrimitives.green400,
        width: medium,
      );

  // ─── Box borders ─────────────────────────────────────────────────────────

  static Border all(BorderSide side) => Border.fromBorderSide(side);

  /// Vertical split between sidebar ↔ editor ↔ panel.
  static Border get verticalSplit => Border(
        right: subtle,
      );

  /// Horizontal split between editor ↔ bottom panel.
  static Border get horizontalSplit => Border(
        top: subtle,
      );

  static Border get bottomHairline => Border(
        bottom: subtle,
      );

  static Border get topHairline => Border(
        top: subtle,
      );

  // ─── Input borders ───────────────────────────────────────────────────────

  static OutlineInputBorder inputNone({BorderRadius? radius}) =>
      OutlineInputBorder(
        borderRadius: radius ?? AetherRadius.inputR,
        borderSide: BorderSide.none,
      );

  static OutlineInputBorder inputEnabled({BorderRadius? radius}) =>
      OutlineInputBorder(
        borderRadius: radius ?? AetherRadius.inputR,
        borderSide: subtle,
      );

  static OutlineInputBorder inputFocused({BorderRadius? radius}) =>
      OutlineInputBorder(
        borderRadius: radius ?? AetherRadius.inputR,
        borderSide: focus,
      );

  static OutlineInputBorder inputError({BorderRadius? radius}) =>
      OutlineInputBorder(
        borderRadius: radius ?? AetherRadius.inputR,
        borderSide: error,
      );
}
