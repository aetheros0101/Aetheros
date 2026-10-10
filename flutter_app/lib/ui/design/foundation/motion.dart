import 'package:flutter/material.dart';

/// AetherOS Foundation — Motion Tokens
///
/// Workbench motion is restrained. Prefer short, linear-ish transitions
/// for chrome; reserve longer curves for dialogs and guided flows.
abstract final class AetherMotion {
  // ─── Durations ───────────────────────────────────────────────────────────

  static const Duration instant = Duration(milliseconds: 50);
  static const Duration fastest = Duration(milliseconds: 80);
  static const Duration fast = Duration(milliseconds: 120);
  static const Duration short = Duration(milliseconds: 180);
  static const Duration medium = Duration(milliseconds: 250);
  static const Duration long = Duration(milliseconds: 350);
  static const Duration longer = Duration(milliseconds: 450);

  // ─── Curves ──────────────────────────────────────────────────────────────

  static const Curve standard = Curves.easeInOut;
  static const Curve enter = Curves.easeOutCubic;
  static const Curve exit = Curves.easeInCubic;
  static const Curve sharp = Curves.easeOut;
  static const Curve linear = Curves.linear;

  // ─── Semantic ────────────────────────────────────────────────────────────

  static const Duration tabSwitch = fast;
  static const Duration panelResize = instant; // follow pointer, no lag
  static const Duration sidebarToggle = short;
  static const Duration dialog = medium;
  static const Duration menu = fast;
  static const Duration tooltip = fastest;
  static const Duration expansion = short;
  static const Duration agentState = short;
  static const Duration statusChange = fast;

  // ─── Helpers ─────────────────────────────────────────────────────────────

  static CurvedAnimation curved(
    AnimationController c, {
    Curve curve = standard,
  }) =>
      CurvedAnimation(parent: c, curve: curve);

  static Widget fade({
    required Animation<double> animation,
    required Widget child,
  }) =>
      FadeTransition(opacity: animation, child: child);
}
