import 'package:flutter/material.dart';

import 'fonts.dart';

/// AetherOS Foundation — Typography Scale
///
/// Two layers:
/// 1. Base UI scale (ui*) for chrome / panels / dialogs
/// 2. Domain scales for workbench surfaces (editor, code, terminal, agent)
///
/// Font families resolve through [AetherFonts] so Settings can override
/// editor/terminal fonts without touching call sites.
abstract final class AetherTypography {
  // ═══════════════════════════════════════════════════════════════════════════
  // UI SCALE
  // ═══════════════════════════════════════════════════════════════════════════

  static const TextStyle uiDisplay = TextStyle(
    fontSize: 28,
    height: 36 / 28,
    fontWeight: FontWeight.w600,
    letterSpacing: -0.25,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle uiTitle = TextStyle(
    fontSize: 18,
    height: 24 / 18,
    fontWeight: FontWeight.w600,
    letterSpacing: 0.0,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle uiSubtitle = TextStyle(
    fontSize: 14,
    height: 20 / 14,
    fontWeight: FontWeight.w600,
    letterSpacing: 0.1,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle uiBody = TextStyle(
    fontSize: 13,
    height: 20 / 13,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.15,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle uiBodyStrong = TextStyle(
    fontSize: 13,
    height: 20 / 13,
    fontWeight: FontWeight.w600,
    letterSpacing: 0.15,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle uiCaption = TextStyle(
    fontSize: 12,
    height: 16 / 12,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.2,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle uiLabel = TextStyle(
    fontSize: 11,
    height: 16 / 11,
    fontWeight: FontWeight.w500,
    letterSpacing: 0.3,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle uiMicro = TextStyle(
    fontSize: 10,
    height: 14 / 10,
    fontWeight: FontWeight.w500,
    letterSpacing: 0.4,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // EDITOR / CODE
  // ═══════════════════════════════════════════════════════════════════════════

  static const TextStyle code = TextStyle(
    fontSize: 13,
    height: 20 / 13,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamily: AetherFonts.code,
    fontFamilyFallback: AetherFonts.monoFallback,
  );

  static const TextStyle codeSmall = TextStyle(
    fontSize: 12,
    height: 18 / 12,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamily: AetherFonts.code,
    fontFamilyFallback: AetherFonts.monoFallback,
  );

  static const TextStyle codeDense = TextStyle(
    fontSize: 11,
    height: 16 / 11,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamily: AetherFonts.code,
    fontFamilyFallback: AetherFonts.monoFallback,
  );

  static const TextStyle fileName = TextStyle(
    fontSize: 13,
    height: 18 / 13,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle filePath = TextStyle(
    fontSize: 12,
    height: 16 / 12,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // TERMINAL
  // ═══════════════════════════════════════════════════════════════════════════

  static const TextStyle terminal = TextStyle(
    fontSize: 13,
    height: 18 / 13,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamily: AetherFonts.terminal,
    fontFamilyFallback: AetherFonts.monoFallback,
  );

  static const TextStyle terminalDense = TextStyle(
    fontSize: 12,
    height: 16 / 12,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamily: AetherFonts.terminal,
    fontFamilyFallback: AetherFonts.monoFallback,
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // AGENT
  // ═══════════════════════════════════════════════════════════════════════════

  static const TextStyle agentTitle = TextStyle(
    fontSize: 14,
    height: 20 / 14,
    fontWeight: FontWeight.w600,
    letterSpacing: 0.0,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle agentMessage = TextStyle(
    fontSize: 13,
    height: 20 / 13,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.1,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle agentTool = TextStyle(
    fontSize: 12,
    height: 16 / 12,
    fontWeight: FontWeight.w500,
    letterSpacing: 0.0,
    fontFamily: AetherFonts.code,
    fontFamilyFallback: AetherFonts.monoFallback,
  );

  static const TextStyle agentStatus = TextStyle(
    fontSize: 11,
    height: 16 / 11,
    fontWeight: FontWeight.w500,
    letterSpacing: 0.3,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // TASK / METADATA / NUMERIC
  // ═══════════════════════════════════════════════════════════════════════════

  static const TextStyle taskTitle = TextStyle(
    fontSize: 13,
    height: 18 / 13,
    fontWeight: FontWeight.w600,
    letterSpacing: 0.0,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle taskMetadata = TextStyle(
    fontSize: 11,
    height: 16 / 11,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.2,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle metadata = TextStyle(
    fontSize: 11,
    height: 16 / 11,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.2,
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle numeric = TextStyle(
    fontSize: 14,
    height: 20 / 14,
    fontWeight: FontWeight.w600,
    letterSpacing: -0.3,
    fontFeatures: [FontFeature.tabularFigures()],
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  static const TextStyle numericLarge = TextStyle(
    fontSize: 22,
    height: 28 / 22,
    fontWeight: FontWeight.w600,
    letterSpacing: -0.5,
    fontFeatures: [FontFeature.tabularFigures()],
    fontFamilyFallback: AetherFonts.uiFallback,
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // DIFF
  // ═══════════════════════════════════════════════════════════════════════════

  static const TextStyle diffLine = TextStyle(
    fontSize: 12,
    height: 18 / 12,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.0,
    fontFamily: AetherFonts.code,
    fontFamilyFallback: AetherFonts.monoFallback,
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // Material TextTheme bridge
  // ═══════════════════════════════════════════════════════════════════════════

  static TextTheme get textTheme => const TextTheme(
        displayLarge: uiDisplay,
        displayMedium: uiDisplay,
        displaySmall: uiTitle,
        headlineLarge: uiTitle,
        headlineMedium: uiTitle,
        headlineSmall: uiSubtitle,
        titleLarge: uiTitle,
        titleMedium: uiSubtitle,
        titleSmall: uiSubtitle,
        bodyLarge: uiBody,
        bodyMedium: uiBody,
        bodySmall: uiCaption,
        labelLarge: uiLabel,
        labelMedium: uiLabel,
        labelSmall: uiMicro,
      );

  /// Override mono families at runtime (Settings → Editor Font).
  static TextStyle codeWith({String? fontFamily, double? fontSize}) =>
      code.copyWith(
        fontFamily: fontFamily ?? AetherFonts.code,
        fontSize: fontSize,
      );

  static TextStyle terminalWith({String? fontFamily, double? fontSize}) =>
      terminal.copyWith(
        fontFamily: fontFamily ?? AetherFonts.terminal,
        fontSize: fontSize,
      );
}
