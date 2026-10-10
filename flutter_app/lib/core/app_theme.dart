/// Backward-compatible theme entry for legacy screens.
///
/// Prefer: `import 'package:aetheros_app/ui/design/aether_theme.dart';`
///
/// Re-exports the P1 design system so screens importing `core/app_theme.dart`
/// continue to compile. Also provides legacy [AetherColors] aliases.
export '../ui/design/aether_theme.dart';

import 'package:flutter/material.dart';
import '../ui/design/semantic/surfaces.dart';
import '../ui/design/semantic/status.dart';

/// Legacy color aliases — migrate call sites to AetherSurfaces / AetherAccent.
abstract final class AetherColors {
  static const background = AetherSurfaces.canvas;
  static const backgroundDeep = AetherSurfaces.chrome;
  static const surface = AetherSurfaces.panel;
  static const surfaceAlt = AetherSurfaces.elevated;
  static const primary = AetherAccent.primary;
  static const success = AetherStatus.success;
  static const warning = AetherStatus.warning;
  static const danger = AetherStatus.danger;
  static const info = AetherStatus.info;
  static const teal = Color(0xFF26A69A);
  static const text = Color(0xFFFFFFFF);
  static const textMuted = Color(0xB3FFFFFF);
  static const textSubtle = Color(0x61FFFFFF);
}
