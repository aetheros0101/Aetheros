import 'package:flutter/material.dart';

import '../foundation/colors.dart';
import 'surfaces.dart';

/// AetherOS Semantic — Editor Tokens
///
/// Colors and roles specific to the code editor surface:
/// line numbers, current line, selection, matches, breakpoints, diff gutters.
abstract final class AetherEditor {
  // ─── Surfaces ────────────────────────────────────────────────────────────

  static const Color background = AetherSurfaces.editor;
  static const Color gutter = AetherColorPrimitives.neutral100;
  static const Color lineNumber = AetherColorPrimitives.neutral600;
  static const Color lineNumberActive = AetherColorPrimitives.neutral800;

  // ─── Cursor & selection ──────────────────────────────────────────────────

  static const Color cursor = AetherColorPrimitives.accent400;
  static final Color selection =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.accent400, 0.28);
  static final Color selectionInactive =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral700, 0.30);

  // ─── Current line / find ─────────────────────────────────────────────────

  static final Color currentLine =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.04);
  static final Color findMatch =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.amber400, 0.30);
  static final Color findMatchActive =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.amber400, 0.50);

  // ─── Breakpoints / debugging ─────────────────────────────────────────────

  static const Color breakpoint = AetherColorPrimitives.red400;
  static final Color breakpointBg =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.red400, 0.12);
  static const Color debugCurrentLine = AetherColorPrimitives.amber400;

  // ─── Indent guides ───────────────────────────────────────────────────────

  static final Color indentGuide =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.06);
  static final Color indentGuideActive =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.14);

  // ─── Bracket match ───────────────────────────────────────────────────────

  static final Color bracketMatch =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.accent400, 0.25);

  // ─── Error / warning squiggles ────────────────────────────────────────────

  static const Color errorSquiggle = AetherColorPrimitives.red400;
  static const Color warningSquiggle = AetherColorPrimitives.amber400;
  static const Color infoSquiggle = AetherColorPrimitives.blue400;

  // ─── Minimap ─────────────────────────────────────────────────────────────

  static final Color minimapBackground =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral0, 0.50);
  static final Color minimapSlider =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.12);
}
