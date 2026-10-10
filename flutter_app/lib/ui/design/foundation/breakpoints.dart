import 'package:flutter/material.dart';

/// AetherOS Foundation — Window Size Classes
///
/// Width-based, device-agnostic. Prefer these over isMobile/isDesktop thinking.
enum AetherWindowClass {
  /// < 600 — single-column, bottom nav or drawer.
  compact,

  /// 600–899 — narrow workbench, collapsible sidebar.
  medium,

  /// 900–1199 — standard workbench with permanent sidebar.
  expanded,

  /// 1200–1599 — wide workbench, optional secondary sidebar.
  wide,

  /// ≥ 1600 — ultra-wide, multi-pane layouts comfortable.
  ultraWide,
}

/// High-level layout mode derived from window class.
enum AetherLayoutMode {
  /// Compact phone-like shell.
  mobile,

  /// Tablet / narrow desktop with toggleable chrome.
  tablet,

  /// Full IDE workbench (sidebar + editor + panel).
  workbench,

  /// Wide workbench with secondary sidebar / multi-editor groups.
  wideWorkbench,
}

/// Breakpoint thresholds and helpers.
abstract final class AetherBreakpoints {
  static const double compactMax = 600;
  static const double mediumMax = 900;
  static const double expandedMax = 1200;
  static const double wideMax = 1600;

  static double widthOf(BuildContext context) =>
      MediaQuery.sizeOf(context).width;

  static AetherWindowClass windowClassOf(BuildContext context) {
    final w = widthOf(context);
    if (w < compactMax) return AetherWindowClass.compact;
    if (w < mediumMax) return AetherWindowClass.medium;
    if (w < expandedMax) return AetherWindowClass.expanded;
    if (w < wideMax) return AetherWindowClass.wide;
    return AetherWindowClass.ultraWide;
  }

  static AetherLayoutMode layoutModeOf(BuildContext context) {
    switch (windowClassOf(context)) {
      case AetherWindowClass.compact:
        return AetherLayoutMode.mobile;
      case AetherWindowClass.medium:
        return AetherLayoutMode.tablet;
      case AetherWindowClass.expanded:
        return AetherLayoutMode.workbench;
      case AetherWindowClass.wide:
      case AetherWindowClass.ultraWide:
        return AetherLayoutMode.wideWorkbench;
    }
  }

  // ─── Layout constants ────────────────────────────────────────────────────

  static const double activityBarWidth = 48;
  static const double sidebarDefaultWidth = 260;
  static const double sidebarMinWidth = 160;
  static const double sidebarMaxWidth = 480;
  static const double secondarySidebarDefaultWidth = 280;
  static const double bottomPanelDefaultHeight = 220;
  static const double bottomPanelMinHeight = 100;
  static const double contentMaxWidth = 900;
  static const double formMaxWidth = 480;

  // ─── Queries ─────────────────────────────────────────────────────────────

  static bool showPermanentSidebar(BuildContext context) =>
      widthOf(context) >= mediumMax;

  static bool showSecondarySidebar(BuildContext context) =>
      widthOf(context) >= expandedMax;

  static bool useMasterDetail(BuildContext context) =>
      widthOf(context) >= mediumMax;

  static int editorGroupColumns(BuildContext context) {
    final w = widthOf(context);
    if (w < mediumMax) return 1;
    if (w < wideMax) return 2;
    return 3;
  }
}

extension AetherBreakpointX on BuildContext {
  AetherWindowClass get windowClass => AetherBreakpoints.windowClassOf(this);
  AetherLayoutMode get layoutMode => AetherBreakpoints.layoutModeOf(this);
}
