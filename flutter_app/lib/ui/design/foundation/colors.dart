import 'package:flutter/material.dart';

/// AetherOS Foundation — Raw Color Primitives
///
/// Neutral-first palette. Accent (violet) is reserved for intentional
/// emphasis only — never used as a broad surface fill.
///
/// Consumers should prefer semantic tokens
/// ([AetherSurfaces], [AetherStatus], [AetherGit], [AetherAgent])
/// over these primitives.
abstract final class AetherColorPrimitives {
  // ─── Neutrals (dark workbench scale) ─────────────────────────────────────
  // Lower index = deeper. Designed for surface contrast hierarchy,
  // not Material elevation.

  static const Color neutral0 = Color(0xFF0A0A10); // deepest canvas
  static const Color neutral50 = Color(0xFF0D0D14);
  static const Color neutral100 = Color(0xFF111118);
  static const Color neutral150 = Color(0xFF15151E);
  static const Color neutral200 = Color(0xFF1A1A24);
  static const Color neutral250 = Color(0xFF1E1E2A);
  static const Color neutral300 = Color(0xFF242430);
  static const Color neutral350 = Color(0xFF2A2A38);
  static const Color neutral400 = Color(0xFF323242);
  static const Color neutral500 = Color(0xFF3E3E52);
  static const Color neutral600 = Color(0xFF52526A);
  static const Color neutral700 = Color(0xFF6E6E88);
  static const Color neutral800 = Color(0xFF9A9AB0);
  static const Color neutral900 = Color(0xFFC8C8D8);
  static const Color neutral950 = Color(0xFFE8E8F0);
  static const Color neutral1000 = Color(0xFFFFFFFF);

  // ─── Accent (violet) — intentional emphasis only ─────────────────────────

  static const Color accent50 = Color(0xFF1A1830);
  static const Color accent100 = Color(0xFF2A2750);
  static const Color accent200 = Color(0xFF3D3A7A);
  static const Color accent300 = Color(0xFF524EC0);
  static const Color accent400 = Color(0xFF6C63FF); // primary accent
  static const Color accent500 = Color(0xFF7B73FF);
  static const Color accent600 = Color(0xFF9B94FF);
  static const Color accent700 = Color(0xFFB8B3FF);

  // ─── Status primitives ───────────────────────────────────────────────────

  static const Color green400 = Color(0xFF4CAF50);
  static const Color green200 = Color(0xFF2E7D32);
  static const Color green100 = Color(0xFF1B3D1E);

  static const Color amber400 = Color(0xFFFFB74D);
  static const Color amber200 = Color(0xFFF57C00);
  static const Color amber100 = Color(0xFF3D2E14);

  static const Color red400 = Color(0xFFEF5350);
  static const Color red200 = Color(0xFFC62828);
  static const Color red100 = Color(0xFF3D1514);

  static const Color blue400 = Color(0xFF42A5F5);
  static const Color blue200 = Color(0xFF1565C0);
  static const Color blue100 = Color(0xFF0D2A3D);

  static const Color teal400 = Color(0xFF26A69A);
  static const Color teal200 = Color(0xFF00796B);
  static const Color teal100 = Color(0xFF0D2E2B);

  static const Color purple400 = Color(0xFFAB47BC);
  static const Color cyan400 = Color(0xFF26C6DA);
  static const Color orange400 = Color(0xFFFF8A65);

  // ─── Alpha helpers ───────────────────────────────────────────────────────

  static Color withAlpha(Color c, double opacity) =>
      c.withOpacity(opacity.clamp(0.0, 1.0));
}
