import 'package:flutter/material.dart';

import 'motion.dart';

/// AetherOS Foundation — Accessibility Tokens & Preferences
///
/// Corporate baseline: state is never communicated by color alone.
/// Every status, approval, git change, and agent state must also expose
/// an icon and a textual label (see [AetherAgent.iconOf] / [labelOf]).
///
/// Wire [AetherAccessibility] to MediaQuery / user Settings so the
/// workbench respects system reduced-motion and contrast preferences.
@immutable
class AetherAccessibility {
  const AetherAccessibility({
    this.reducedMotion = false,
    this.highContrast = false,
    this.largeText = false,
    this.boldText = false,
    this.accessibleNavigation = false,
    this.enforceFocusIndicators = true,
  });

  /// Disable non-essential animation (respects system "reduce motion").
  final bool reducedMotion;

  /// Increase border/contrast weight for low-vision users.
  final bool highContrast;

  /// Scale up body/UI text (~1.15×).
  final bool largeText;

  /// Prefer heavier font weights.
  final bool boldText;

  /// Platform accessibility navigation mode (TalkBack / VoiceOver).
  final bool accessibleNavigation;

  /// Always paint focus rings on keyboard navigation (default true).
  final bool enforceFocusIndicators;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AetherAccessibility &&
          reducedMotion == other.reducedMotion &&
          highContrast == other.highContrast &&
          largeText == other.largeText &&
          boldText == other.boldText &&
          accessibleNavigation == other.accessibleNavigation &&
          enforceFocusIndicators == other.enforceFocusIndicators;

  @override
  int get hashCode => Object.hash(
        reducedMotion,
        highContrast,
        largeText,
        boldText,
        accessibleNavigation,
        enforceFocusIndicators,
      );

  // ─── Resolve from MediaQuery ─────────────────────────────────────────────

  factory AetherAccessibility.of(BuildContext context) {
    final mq = MediaQuery.of(context);
    return AetherAccessibility(
      reducedMotion: mq.disableAnimations,
      highContrast: mq.highContrast,
      largeText: mq.textScaler.scale(14) > 16,
      boldText: mq.boldText,
      accessibleNavigation: mq.accessibleNavigation,
      enforceFocusIndicators: true,
    );
  }

  // ─── Motion helpers ──────────────────────────────────────────────────────

  /// Returns [Duration.zero] when reduced motion is on.
  Duration duration(Duration normal) =>
      reducedMotion ? Duration.zero : normal;

  /// Standard page/panel transition respecting reduced motion.
  Duration get panelTransition => duration(AetherMotion.sidebarToggle);

  Duration get dialogTransition => duration(AetherMotion.dialog);

  // ─── Focus ───────────────────────────────────────────────────────────────

  /// Whether focus rings must be visible for the current input modality.
  bool get showFocusRings => enforceFocusIndicators || accessibleNavigation;

  // ─── Text scale ──────────────────────────────────────────────────────────

  double get textScaleFactor => largeText ? 1.15 : 1.0;

  FontWeight adjustWeight(FontWeight base) {
    if (!boldText) return base;
    final index = base.index + 2;
    return FontWeight.values[index.clamp(0, FontWeight.values.length - 1)];
  }

  // ─── Contrast helpers ────────────────────────────────────────────────────

  /// Border width multiplier under high contrast.
  double get borderScale => highContrast ? 1.5 : 1.0;

  /// Minimum recommended contrast ratio reminder (WCAG AA = 4.5).
  static const double minContrastRatio = 4.5;

  // ─── Copy ────────────────────────────────────────────────────────────────

  AetherAccessibility copyWith({
    bool? reducedMotion,
    bool? highContrast,
    bool? largeText,
    bool? boldText,
    bool? accessibleNavigation,
    bool? enforceFocusIndicators,
  }) {
    return AetherAccessibility(
      reducedMotion: reducedMotion ?? this.reducedMotion,
      highContrast: highContrast ?? this.highContrast,
      largeText: largeText ?? this.largeText,
      boldText: boldText ?? this.boldText,
      accessibleNavigation: accessibleNavigation ?? this.accessibleNavigation,
      enforceFocusIndicators:
          enforceFocusIndicators ?? this.enforceFocusIndicators,
    );
  }

  static const AetherAccessibility defaults = AetherAccessibility();
}

/// Inherited access to [AetherAccessibility] down the tree.
class AetherAccessibilityScope extends InheritedWidget {
  const AetherAccessibilityScope({
    super.key,
    required this.data,
    required super.child,
  });

  final AetherAccessibility data;

  static AetherAccessibility of(BuildContext context) {
    final scope =
        context.dependOnInheritedWidgetOfExactType<AetherAccessibilityScope>();
    return scope?.data ?? AetherAccessibility.of(context);
  }

  @override
  bool updateShouldNotify(AetherAccessibilityScope oldWidget) =>
      data.reducedMotion != oldWidget.data.reducedMotion ||
      data.highContrast != oldWidget.data.highContrast ||
      data.largeText != oldWidget.data.largeText ||
      data.boldText != oldWidget.data.boldText ||
      data.accessibleNavigation != oldWidget.data.accessibleNavigation ||
      data.enforceFocusIndicators != oldWidget.data.enforceFocusIndicators;
}
