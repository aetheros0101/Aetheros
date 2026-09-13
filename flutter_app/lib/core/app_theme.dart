import 'package:flutter/material.dart';

abstract final class AetherColors {
  static const background = Color(0xFF0F0F1A);
  static const backgroundDeep = Color(0xFF0D0D18);
  static const surface = Color(0xFF1A1A2E);
  static const surfaceAlt = Color(0xFF252540);
  static const primary = Color(0xFF6C63FF);
  static const success = Color(0xFF4CAF50);
  static const warning = Color(0xFFFFB74D);
  static const danger = Color(0xFFEF5350);
  static const info = Color(0xFF42A5F5);
  static const teal = Color(0xFF26A69A);
  static const text = Colors.white;
  static const textMuted = Colors.white70;
  static const textSubtle = Colors.white38;
}

abstract final class AetherSpacing {
  static const xs = 4.0;
  static const sm = 8.0;
  static const md = 12.0;
  static const lg = 16.0;
  static const xl = 24.0;
  static const xxl = 32.0;
}

ThemeData buildAetherTheme() {
  final scheme = ColorScheme.fromSeed(
    seedColor: AetherColors.primary,
    brightness: Brightness.dark,
  ).copyWith(
    surface: AetherColors.surface,
    primary: AetherColors.primary,
    error: AetherColors.danger,
  );

  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: AetherColors.background,
    canvasColor: AetherColors.background,
    visualDensity: VisualDensity.standard,
    materialTapTargetSize: MaterialTapTargetSize.padded,
    appBarTheme: const AppBarTheme(
      backgroundColor: AetherColors.background,
      foregroundColor: AetherColors.text,
      elevation: 0,
    ),
    cardTheme: CardTheme(
      color: AetherColors.surface,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: AetherColors.surface,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: BorderSide.none,
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: Colors.white12),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: AetherColors.primary),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size(0, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(0, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
    ),
    snackBarTheme: const SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
    ),
  );
}
