import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'foundation/borders.dart';
import 'foundation/breakpoints.dart';
import 'foundation/colors.dart';
import 'foundation/density.dart';
import 'foundation/motion.dart';
import 'foundation/radius.dart';
import 'foundation/shadows.dart';
import 'foundation/spacing.dart';
import 'foundation/typography.dart';
import 'semantic/surfaces.dart';
import 'semantic/status.dart';

// ─── Barrel exports ──────────────────────────────────────────────────────────
export 'foundation/borders.dart';
export 'foundation/breakpoints.dart';
export 'foundation/colors.dart';
export 'foundation/density.dart';
export 'foundation/icons.dart';
export 'foundation/fonts.dart';
export 'foundation/accessibility.dart';
export 'foundation/motion.dart';
export 'foundation/radius.dart';
export 'foundation/shadows.dart';
export 'foundation/spacing.dart';
export 'foundation/typography.dart';
export 'semantic/agent.dart';
export 'semantic/editor.dart';
export 'semantic/git.dart';
export 'semantic/interaction.dart';
export 'semantic/status.dart';
export 'semantic/surfaces.dart';

/// AetherOS Design System — Flutter ThemeData Adapter
///
/// Architecture:
/// ```
/// Foundation tokens  →  Semantic tokens  →  AetherTheme (this)  →  ThemeData
/// ```
///
/// [ThemeData] is the Flutter adapter, not the design system itself.
/// Workbench UI should prefer semantic tokens (AetherSurfaces, AetherAgent,
/// AetherGit, …) over ColorScheme roles when painting domain surfaces.
ThemeData buildAetherTheme({
  AetherDensityMode density = AetherDensityMode.standard,
  String? fontFamily,
}) {
  final visualDensity = AetherDensity.toVisualDensity(density);
  final textTheme = AetherTypography.textTheme;

  final colorScheme = ColorScheme(
    brightness: Brightness.dark,
    primary: AetherAccent.primary,
    onPrimary: AetherAccent.onAccent,
    primaryContainer: AetherAccent.muted,
    onPrimaryContainer: AetherTextColors.primary,
    secondary: AetherColorPrimitives.teal400,
    onSecondary: AetherTextColors.primary,
    secondaryContainer: AetherColorPrimitives.teal100,
    onSecondaryContainer: AetherTextColors.primary,
    tertiary: AetherColorPrimitives.blue400,
    onTertiary: AetherTextColors.primary,
    tertiaryContainer: AetherColorPrimitives.blue100,
    onTertiaryContainer: AetherTextColors.primary,
    error: AetherStatus.danger,
    onError: AetherStatus.onDanger,
    errorContainer: AetherStatus.dangerContainer,
    onErrorContainer: AetherTextColors.primary,
    surface: AetherSurfaces.panel,
    onSurface: AetherTextColors.primary,
    surfaceContainerHighest: AetherSurfaces.elevated,
    onSurfaceVariant: AetherTextColors.secondary,
    outline: AetherColorPrimitives.withAlpha(
      AetherColorPrimitives.neutral1000,
      0.14,
    ),
    outlineVariant: AetherColorPrimitives.withAlpha(
      AetherColorPrimitives.neutral1000,
      0.08,
    ),
    shadow: const Color(0xFF000000),
    scrim: AetherSurfaces.overlay,
    inverseSurface: AetherTextColors.primary,
    onInverseSurface: AetherTextColors.inverse,
    inversePrimary: AetherAccent.hover,
  );

  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: colorScheme,
    textTheme: textTheme.apply(
      bodyColor: AetherTextColors.primary,
      displayColor: AetherTextColors.primary,
    ),
    scaffoldBackgroundColor: AetherSurfaces.canvas,
    canvasColor: AetherSurfaces.canvas,
    visualDensity: visualDensity,
    materialTapTargetSize: density == AetherDensityMode.comfortable
        ? MaterialTapTargetSize.padded
        : MaterialTapTargetSize.shrinkWrap,

    // ─── AppBar / title bar ────────────────────────────────────────────────
    appBarTheme: AppBarTheme(
      backgroundColor: AetherSurfaces.chrome,
      foregroundColor: AetherTextColors.primary,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleTextStyle: AetherTypography.uiSubtitle.copyWith(
        color: AetherTextColors.primary,
      ),
      systemOverlayStyle: const SystemUiOverlayStyle(
        statusBarColor: Colors.transparent,
        statusBarIconBrightness: Brightness.light,
        systemNavigationBarColor: AetherSurfaces.canvas,
        systemNavigationBarIconBrightness: Brightness.light,
      ),
    ),

    // ─── Cards (settings sections, agent cards — not workbench panels) ─────
    cardTheme: CardTheme(
      color: AetherSurfaces.elevated,
      elevation: 0,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.cardR),
      clipBehavior: Clip.antiAlias,
    ),

    // ─── Inputs ────────────────────────────────────────────────────────────
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: AetherSurfaces.input,
      contentPadding: const EdgeInsets.symmetric(
        horizontal: AetherSpacing.md,
        vertical: AetherSpacing.sm,
      ),
      border: AetherBorders.inputNone(),
      enabledBorder: AetherBorders.inputEnabled(),
      focusedBorder: AetherBorders.inputFocused(),
      errorBorder: AetherBorders.inputError(),
      focusedErrorBorder: AetherBorders.inputError(),
      hintStyle: AetherTypography.uiCaption.copyWith(
        color: AetherTextColors.tertiary,
      ),
      labelStyle: AetherTypography.uiCaption.copyWith(
        color: AetherTextColors.secondary,
      ),
      errorStyle: AetherTypography.uiCaption.copyWith(
        color: AetherStatus.danger,
      ),
      prefixIconColor: AetherTextColors.secondary,
      suffixIconColor: AetherTextColors.secondary,
      isDense: density != AetherDensityMode.comfortable,
    ),

    // ─── Buttons ───────────────────────────────────────────────────────────
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: Size(0, AetherDensity.inputHeight(density)),
        padding: const EdgeInsets.symmetric(
          horizontal: AetherSpacing.lg,
          vertical: AetherSpacing.sm,
        ),
        shape: RoundedRectangleBorder(borderRadius: AetherRadius.buttonR),
        textStyle: AetherTypography.uiLabel,
        elevation: 0,
        backgroundColor: AetherAccent.primary,
        foregroundColor: AetherAccent.onAccent,
      ),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        minimumSize: Size(0, AetherDensity.inputHeight(density)),
        padding: const EdgeInsets.symmetric(
          horizontal: AetherSpacing.lg,
          vertical: AetherSpacing.sm,
        ),
        shape: RoundedRectangleBorder(borderRadius: AetherRadius.buttonR),
        textStyle: AetherTypography.uiLabel,
        elevation: 0,
        backgroundColor: AetherSurfaces.elevated,
        foregroundColor: AetherTextColors.primary,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: Size(0, AetherDensity.inputHeight(density)),
        padding: const EdgeInsets.symmetric(
          horizontal: AetherSpacing.lg,
          vertical: AetherSpacing.sm,
        ),
        shape: RoundedRectangleBorder(borderRadius: AetherRadius.buttonR),
        side: AetherBorders.standard,
        textStyle: AetherTypography.uiLabel,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        minimumSize: Size(0, AetherDensity.toolbarHeight(density)),
        padding: const EdgeInsets.symmetric(
          horizontal: AetherSpacing.md,
          vertical: AetherSpacing.xs,
        ),
        shape: RoundedRectangleBorder(borderRadius: AetherRadius.buttonR),
        textStyle: AetherTypography.uiLabel,
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(
        minimumSize: Size(
          AetherDensity.toolbarHeight(density),
          AetherDensity.toolbarHeight(density),
        ),
        foregroundColor: AetherTextColors.secondary,
        hoverColor: AetherColorPrimitives.withAlpha(
          AetherColorPrimitives.neutral1000,
          0.06,
        ),
      ),
    ),
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: AetherAccent.primary,
      foregroundColor: AetherAccent.onAccent,
      elevation: 0,
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.mdR),
    ),

    // ─── Chips ─────────────────────────────────────────────────────────────
    chipTheme: ChipThemeData(
      backgroundColor: AetherSurfaces.elevated,
      selectedColor: AetherAccent.muted,
      disabledColor: AetherSurfaces.panel,
      labelStyle: AetherTypography.uiLabel.copyWith(
        color: AetherTextColors.primary,
      ),
      secondaryLabelStyle: AetherTypography.uiLabel.copyWith(
        color: AetherTextColors.secondary,
      ),
      padding: const EdgeInsets.symmetric(
        horizontal: AetherSpacing.sm,
        vertical: AetherSpacing.xxs,
      ),
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.chipR),
      side: BorderSide.none,
    ),

    // ─── Dialogs & sheets (floating UI — may use shadow) ───────────────────
    dialogTheme: DialogTheme(
      backgroundColor: AetherSurfaces.dialog,
      elevation: 0,
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.dialogR),
      titleTextStyle: AetherTypography.uiTitle.copyWith(
        color: AetherTextColors.primary,
      ),
      contentTextStyle: AetherTypography.uiBody.copyWith(
        color: AetherTextColors.secondary,
      ),
    ),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: AetherSurfaces.dialog,
      elevation: 0,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(
          top: Radius.circular(AetherRadius.md),
        ),
      ),
      clipBehavior: Clip.antiAlias,
      showDragHandle: true,
      dragHandleColor: AetherTextColors.tertiary,
    ),

    // ─── Navigation (mobile shell; workbench uses custom chrome) ───────────
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: AetherSurfaces.chrome,
      indicatorColor: AetherAccent.muted,
      elevation: 0,
      height: 56,
      labelTextStyle: WidgetStateProperty.resolveWith((states) {
        final selected = states.contains(WidgetState.selected);
        return AetherTypography.uiMicro.copyWith(
          color: selected ? AetherAccent.primary : AetherTextColors.secondary,
          fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
        );
      }),
      iconTheme: WidgetStateProperty.resolveWith((states) {
        final selected = states.contains(WidgetState.selected);
        return IconThemeData(
          size: 22,
          color: selected ? AetherAccent.primary : AetherTextColors.secondary,
        );
      }),
    ),
    navigationRailTheme: NavigationRailThemeData(
      backgroundColor: AetherSurfaces.chrome,
      indicatorColor: AetherAccent.muted,
      selectedIconTheme: const IconThemeData(
        color: AetherAccent.primary,
        size: 22,
      ),
      unselectedIconTheme: IconThemeData(
        color: AetherTextColors.secondary,
        size: 22,
      ),
      selectedLabelTextStyle: AetherTypography.uiMicro.copyWith(
        color: AetherAccent.primary,
      ),
      unselectedLabelTextStyle: AetherTypography.uiMicro.copyWith(
        color: AetherTextColors.secondary,
      ),
    ),
    drawerTheme: const DrawerThemeData(
      backgroundColor: AetherSurfaces.sidebar,
      elevation: 0,
      shape: RoundedRectangleBorder(),
    ),
    tabBarTheme: TabBarTheme(
      indicatorColor: AetherAccent.primary,
      labelColor: AetherTextColors.primary,
      unselectedLabelColor: AetherTextColors.secondary,
      labelStyle: AetherTypography.uiLabel,
      unselectedLabelStyle: AetherTypography.uiLabel,
      indicatorSize: TabBarIndicatorSize.label,
      dividerColor: AetherColorPrimitives.withAlpha(
        AetherColorPrimitives.neutral1000,
        0.08,
      ),
    ),

    // ─── Lists & dividers ──────────────────────────────────────────────────
    listTileTheme: ListTileThemeData(
      contentPadding: const EdgeInsets.symmetric(
        horizontal: AetherSpacing.md,
        vertical: AetherSpacing.xxs,
      ),
      dense: density != AetherDensityMode.comfortable,
      iconColor: AetherTextColors.secondary,
      textColor: AetherTextColors.primary,
      titleTextStyle: AetherTypography.uiBody.copyWith(
        color: AetherTextColors.primary,
      ),
      subtitleTextStyle: AetherTypography.uiCaption.copyWith(
        color: AetherTextColors.secondary,
      ),
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.xsR),
    ),
    dividerTheme: DividerThemeData(
      color: AetherColorPrimitives.withAlpha(
        AetherColorPrimitives.neutral1000,
        0.08,
      ),
      thickness: 1,
      space: 1,
    ),

    // ─── Feedback ──────────────────────────────────────────────────────────
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: AetherSurfaces.popup,
      contentTextStyle: AetherTypography.uiBody.copyWith(
        color: AetherTextColors.primary,
      ),
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.smR),
      elevation: 0,
    ),
    tooltipTheme: TooltipThemeData(
      decoration: BoxDecoration(
        color: AetherSurfaces.tooltip,
        borderRadius: AetherRadius.xsR,
        boxShadow: AetherShadows.tooltip,
      ),
      textStyle: AetherTypography.uiCaption.copyWith(
        color: AetherTextColors.primary,
      ),
      waitDuration: AetherMotion.tooltip,
    ),

    // ─── Progress & selection ──────────────────────────────────────────────
    progressIndicatorTheme: const ProgressIndicatorThemeData(
      color: AetherAccent.primary,
      linearTrackColor: AetherSurfaces.elevated,
      circularTrackColor: AetherSurfaces.elevated,
    ),
    switchTheme: SwitchThemeData(
      thumbColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) {
          return AetherAccent.onAccent;
        }
        return AetherTextColors.secondary;
      }),
      trackColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) {
          return AetherAccent.primary;
        }
        return AetherSurfaces.elevated;
      }),
    ),
    checkboxTheme: CheckboxThemeData(
      fillColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) {
          return AetherAccent.primary;
        }
        return Colors.transparent;
      }),
      checkColor: const WidgetStatePropertyAll(AetherAccent.onAccent),
      side: AetherBorders.standard,
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.xxsR),
    ),
    radioTheme: RadioThemeData(
      fillColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) {
          return AetherAccent.primary;
        }
        return AetherTextColors.secondary;
      }),
    ),

    // ─── Menus (floating — shadow allowed) ─────────────────────────────────
    popupMenuTheme: PopupMenuThemeData(
      color: AetherSurfaces.popup,
      elevation: 0,
      shape: RoundedRectangleBorder(borderRadius: AetherRadius.menuR),
      textStyle: AetherTypography.uiBody.copyWith(
        color: AetherTextColors.primary,
      ),
    ),
    menuTheme: MenuThemeData(
      style: MenuStyle(
        backgroundColor: const WidgetStatePropertyAll(AetherSurfaces.popup),
        elevation: const WidgetStatePropertyAll(0),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: AetherRadius.menuR),
        ),
        shadowColor: const WidgetStatePropertyAll(Colors.transparent),
      ),
    ),
    dropdownMenuTheme: DropdownMenuThemeData(
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: AetherSurfaces.input,
        border: AetherBorders.inputEnabled(),
        enabledBorder: AetherBorders.inputEnabled(),
        focusedBorder: AetherBorders.inputFocused(),
      ),
      menuStyle: MenuStyle(
        backgroundColor: const WidgetStatePropertyAll(AetherSurfaces.popup),
        elevation: const WidgetStatePropertyAll(0),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: AetherRadius.menuR),
        ),
      ),
    ),

    // ─── Scrollbar ─────────────────────────────────────────────────────────
    scrollbarTheme: ScrollbarThemeData(
      thumbColor: WidgetStatePropertyAll(
        AetherColorPrimitives.withAlpha(
          AetherColorPrimitives.neutral1000,
          0.25,
        ),
      ),
      radius: const Radius.circular(AetherRadius.full),
      thickness: const WidgetStatePropertyAll(6),
    ),
    dividerColor: AetherColorPrimitives.withAlpha(
      AetherColorPrimitives.neutral1000,
      0.08,
    ),
    disabledColor: AetherTextColors.disabled,
    highlightColor: AetherColorPrimitives.withAlpha(
      AetherColorPrimitives.neutral1000,
      0.06,
    ),
    splashColor: AetherColorPrimitives.withAlpha(
      AetherColorPrimitives.neutral1000,
      0.10,
    ),
    hoverColor: AetherColorPrimitives.withAlpha(
      AetherColorPrimitives.neutral1000,
      0.06,
    ),
    focusColor: AetherColorPrimitives.withAlpha(
      AetherColorPrimitives.accent400,
      0.10,
    ),
  );
}

/// Quick token access from [BuildContext].
extension AetherThemeX on BuildContext {
  ThemeData get aetherTheme => Theme.of(this);
  ColorScheme get aetherColorScheme => Theme.of(this).colorScheme;
  TextTheme get aetherText => Theme.of(this).textTheme;

  AetherWindowClass get windowClass => AetherBreakpoints.windowClassOf(this);
  AetherLayoutMode get layoutMode => AetherBreakpoints.layoutModeOf(this);
}
