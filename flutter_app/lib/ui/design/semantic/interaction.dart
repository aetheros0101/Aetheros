import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../foundation/borders.dart';
import '../foundation/colors.dart';
import 'surfaces.dart';

/// Interaction state roles used across all workbench components.
enum AetherInteractionState {
  rest,
  hover,
  pressed,
  focused,
  selected,
  disabled,
  loading,
  dragged,
  dropTarget,
  error,
  success,
}

/// Resolved visual style for a single interaction state.
///
/// Components (tree rows, tabs, buttons) consume [AetherInteractionStyle]
/// instead of inventing their own hover/selected colors.
@immutable
class AetherInteractionStyle {
  const AetherInteractionStyle({
    required this.background,
    required this.foreground,
    required this.icon,
    this.border,
    this.focusRing,
    this.cursor = SystemMouseCursors.basic,
    this.opacity = 1.0,
  });

  final Color background;
  final Color foreground;
  final Color icon;
  final BorderSide? border;
  final BorderSide? focusRing;
  final MouseCursor cursor;
  final double opacity;

  AetherInteractionStyle copyWith({
    Color? background,
    Color? foreground,
    Color? icon,
    BorderSide? border,
    BorderSide? focusRing,
    MouseCursor? cursor,
    double? opacity,
  }) {
    return AetherInteractionStyle(
      background: background ?? this.background,
      foreground: foreground ?? this.foreground,
      icon: icon ?? this.icon,
      border: border ?? this.border,
      focusRing: focusRing ?? this.focusRing,
      cursor: cursor ?? this.cursor,
      opacity: opacity ?? this.opacity,
    );
  }
}

/// AetherOS Semantic — Interaction Tokens
///
/// Resolves a full [AetherInteractionStyle] per state so every component
/// shares one interaction language.
///
/// Visual principle: state is communicated by icon + subtle tint first;
/// full-saturation color is reserved for badges and critical indicators.
abstract final class AetherInteraction {
  static final Color _hoverWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.06);
  static final Color _pressedWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.10);
  static final Color _selectedWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.accent400, 0.12);
  static final Color _focusedWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.accent400, 0.08);
  static final Color _dropWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.accent400, 0.16);
  static final Color _draggedWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral1000, 0.08);
  static final Color _errorWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.red400, 0.10);
  static final Color _successWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.green400, 0.10);
  static final Color _disabledWash =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.neutral0, 0.35);

  static AetherInteractionStyle resolve(
    AetherInteractionState state, {
    Color base = AetherSurfaces.sidebar,
  }) {
    switch (state) {
      case AetherInteractionState.rest:
        return AetherInteractionStyle(
          background: base,
          foreground: AetherTextColors.primary,
          icon: AetherTextColors.secondary,
          cursor: SystemMouseCursors.basic,
        );
      case AetherInteractionState.hover:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_hoverWash, base),
          foreground: AetherTextColors.primary,
          icon: AetherTextColors.primary,
          cursor: SystemMouseCursors.click,
        );
      case AetherInteractionState.pressed:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_pressedWash, base),
          foreground: AetherTextColors.primary,
          icon: AetherTextColors.primary,
          cursor: SystemMouseCursors.click,
        );
      case AetherInteractionState.focused:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_focusedWash, base),
          foreground: AetherTextColors.primary,
          icon: AetherTextColors.primary,
          focusRing: AetherBorders.focus,
          cursor: SystemMouseCursors.basic,
        );
      case AetherInteractionState.selected:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_selectedWash, base),
          foreground: AetherTextColors.primary,
          icon: AetherAccent.primary,
          border: BorderSide(
            color: AetherColorPrimitives.withAlpha(
              AetherColorPrimitives.accent400,
              0.35,
            ),
            width: 1,
          ),
          cursor: SystemMouseCursors.basic,
        );
      case AetherInteractionState.disabled:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_disabledWash, base),
          foreground: AetherTextColors.disabled,
          icon: AetherTextColors.disabled,
          cursor: SystemMouseCursors.forbidden,
          opacity: 0.55,
        );
      case AetherInteractionState.loading:
        return AetherInteractionStyle(
          background: base,
          foreground: AetherTextColors.secondary,
          icon: AetherTextColors.secondary,
          cursor: SystemMouseCursors.progress,
          opacity: 0.85,
        );
      case AetherInteractionState.dragged:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_draggedWash, base),
          foreground: AetherTextColors.primary,
          icon: AetherTextColors.primary,
          cursor: SystemMouseCursors.grabbing,
          opacity: 0.75,
        );
      case AetherInteractionState.dropTarget:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_dropWash, base),
          foreground: AetherTextColors.primary,
          icon: AetherAccent.primary,
          border: const BorderSide(
            color: AetherAccent.primary,
            width: 1.5,
          ),
          cursor: SystemMouseCursors.copy,
        );
      case AetherInteractionState.error:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_errorWash, base),
          foreground: AetherColorPrimitives.red400,
          icon: AetherColorPrimitives.red400,
          cursor: SystemMouseCursors.basic,
        );
      case AetherInteractionState.success:
        return AetherInteractionStyle(
          background: Color.alphaBlend(_successWash, base),
          foreground: AetherColorPrimitives.green400,
          icon: AetherColorPrimitives.green400,
          cursor: SystemMouseCursors.basic,
        );
    }
  }

  static bool showFocusRing(AetherInteractionState state) =>
      state == AetherInteractionState.focused;
}
