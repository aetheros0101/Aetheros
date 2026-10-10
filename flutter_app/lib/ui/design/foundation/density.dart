import 'package:flutter/material.dart';

/// AetherOS Foundation — Density
///
/// Four workbench density modes. Density primarily affects row heights,
/// tree indentation visual weight, and toolbar padding — not overall scale.
enum AetherDensityMode {
  /// Touch-first / mobile.
  comfortable,

  /// Default desktop workbench.
  standard,

  /// Code, terminal, git, explorer — denser rows.
  compact,

  /// Large repositories, data-heavy tables, log streams.
  ultraCompact,
}

/// Density token values resolved per [AetherDensityMode].
abstract final class AetherDensity {
  // ─── Material bridges ────────────────────────────────────────────────────

  static const VisualDensity materialStandard = VisualDensity.standard;
  static const VisualDensity materialCompact = VisualDensity.compact;
  static const VisualDensity materialComfortable = VisualDensity.comfortable;

  // ─── Touch targets ───────────────────────────────────────────────────────

  static const double minTouchTarget = 48.0;
  static const double minTouchTargetCompact = 36.0;

  // ─── Resolve helpers ─────────────────────────────────────────────────────

  static double rowHeight(AetherDensityMode mode) {
    switch (mode) {
      case AetherDensityMode.comfortable:
        return 40;
      case AetherDensityMode.standard:
        return 32;
      case AetherDensityMode.compact:
        return 26;
      case AetherDensityMode.ultraCompact:
        return 22;
    }
  }

  static double treeRowHeight(AetherDensityMode mode) => rowHeight(mode);

  static double toolbarHeight(AetherDensityMode mode) {
    switch (mode) {
      case AetherDensityMode.comfortable:
        return 44;
      case AetherDensityMode.standard:
        return 36;
      case AetherDensityMode.compact:
        return 32;
      case AetherDensityMode.ultraCompact:
        return 28;
    }
  }

  static double tabHeight(AetherDensityMode mode) {
    switch (mode) {
      case AetherDensityMode.comfortable:
        return 40;
      case AetherDensityMode.standard:
        return 34;
      case AetherDensityMode.compact:
        return 30;
      case AetherDensityMode.ultraCompact:
        return 26;
    }
  }

  static double inputHeight(AetherDensityMode mode) {
    switch (mode) {
      case AetherDensityMode.comfortable:
        return 40;
      case AetherDensityMode.standard:
        return 34;
      case AetherDensityMode.compact:
        return 30;
      case AetherDensityMode.ultraCompact:
        return 26;
    }
  }

  static double iconSize(AetherDensityMode mode) {
    switch (mode) {
      case AetherDensityMode.comfortable:
        return 20;
      case AetherDensityMode.standard:
        return 16;
      case AetherDensityMode.compact:
        return 14;
      case AetherDensityMode.ultraCompact:
        return 12;
    }
  }

  static double treeIndent(AetherDensityMode mode) {
    switch (mode) {
      case AetherDensityMode.comfortable:
        return 20;
      case AetherDensityMode.standard:
        return 16;
      case AetherDensityMode.compact:
        return 14;
      case AetherDensityMode.ultraCompact:
        return 12;
    }
  }

  static VisualDensity toVisualDensity(AetherDensityMode mode) {
    switch (mode) {
      case AetherDensityMode.comfortable:
        return materialComfortable;
      case AetherDensityMode.standard:
        return materialStandard;
      case AetherDensityMode.compact:
      case AetherDensityMode.ultraCompact:
        return materialCompact;
    }
  }
}
