import 'package:flutter/widgets.dart';

import '../design/foundation/breakpoints.dart';

// AetherLayoutMode tek yerde (breakpoints.dart) tanımlıdır; bu dosyayı
// import eden kod eskisi gibi görmeye devam etsin diye yeniden dışa aktarılır.
export '../design/foundation/breakpoints.dart' show AetherLayoutMode;

AetherLayoutMode layoutModeForWidth(double width) {
  if (width < AetherBreakpoints.compactMax) return AetherLayoutMode.mobile;
  if (width < AetherBreakpoints.mediumMax) return AetherLayoutMode.tablet;
  if (width < AetherBreakpoints.expandedMax) return AetherLayoutMode.workbench;
  return AetherLayoutMode.wideWorkbench;
}

AetherLayoutMode layoutModeOf(BuildContext context) =>
    layoutModeForWidth(MediaQuery.sizeOf(context).width);

/// Convenience flags for shell visibility decisions.
extension AetherLayoutModeX on AetherLayoutMode {
  bool get showActivityBar => this != AetherLayoutMode.mobile;
  bool get showPrimarySidebar =>
      this == AetherLayoutMode.tablet ||
      this == AetherLayoutMode.workbench ||
      this == AetherLayoutMode.wideWorkbench;
  bool get showSecondarySidebar =>
      this == AetherLayoutMode.workbench ||
      this == AetherLayoutMode.wideWorkbench;
  bool get collapseSidebarsByDefault => this == AetherLayoutMode.mobile;

  /// Mobile uses bottom nav + drawer instead of permanent chrome.
  bool get useMobileNav => this == AetherLayoutMode.mobile;

  /// Tablet: permanent primary optional; secondary still gated by width.
  bool get isTablet => this == AetherLayoutMode.tablet;
}
