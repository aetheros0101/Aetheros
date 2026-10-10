/// AetherOS Application State — Workbench Chrome Layout
///
/// Owns only chrome visibility and sizes.
/// Active activity lives in [NavigationState] — single source of truth.
class WorkbenchLayoutState {
  const WorkbenchLayoutState({
    this.activityBarVisible = true,
    this.primarySidebarVisible = true,
    this.primarySidebarWidth = 260,
    this.secondarySidebarVisible = false,
    this.secondarySidebarWidth = 280,
    this.bottomPanelVisible = true,
    this.bottomPanelHeight = 220,
    this.statusBarVisible = true,
    this.density = 'standard',
    this.windowMode = 'workbench',
  });

  final bool activityBarVisible;
  final bool primarySidebarVisible;
  final double primarySidebarWidth;
  final bool secondarySidebarVisible;
  final double secondarySidebarWidth;
  final bool bottomPanelVisible;
  final double bottomPanelHeight;
  final bool statusBarVisible;
  final String density;
  final String windowMode;

  static const double sidebarMin = 160;
  static const double sidebarMax = 480;
  static const double panelMin = 100;
  static const double panelMax = 600;

  double get clampedPrimarySidebarWidth =>
      primarySidebarWidth.clamp(sidebarMin, sidebarMax);
  double get clampedSecondarySidebarWidth =>
      secondarySidebarWidth.clamp(sidebarMin, sidebarMax);
  double get clampedBottomPanelHeight =>
      bottomPanelHeight.clamp(panelMin, panelMax);

  WorkbenchLayoutState copyWith({
    bool? activityBarVisible,
    bool? primarySidebarVisible,
    double? primarySidebarWidth,
    bool? secondarySidebarVisible,
    double? secondarySidebarWidth,
    bool? bottomPanelVisible,
    double? bottomPanelHeight,
    bool? statusBarVisible,
    String? density,
    String? windowMode,
  }) {
    return WorkbenchLayoutState(
      activityBarVisible: activityBarVisible ?? this.activityBarVisible,
      primarySidebarVisible: primarySidebarVisible ?? this.primarySidebarVisible,
      primarySidebarWidth: primarySidebarWidth ?? this.primarySidebarWidth,
      secondarySidebarVisible: secondarySidebarVisible ?? this.secondarySidebarVisible,
      secondarySidebarWidth: secondarySidebarWidth ?? this.secondarySidebarWidth,
      bottomPanelVisible: bottomPanelVisible ?? this.bottomPanelVisible,
      bottomPanelHeight: bottomPanelHeight ?? this.bottomPanelHeight,
      statusBarVisible: statusBarVisible ?? this.statusBarVisible,
      density: density ?? this.density,
      windowMode: windowMode ?? this.windowMode,
    );
  }

  Map<String, dynamic> toJson() => {
        'activityBarVisible': activityBarVisible,
        'primarySidebarVisible': primarySidebarVisible,
        'primarySidebarWidth': primarySidebarWidth,
        'secondarySidebarVisible': secondarySidebarVisible,
        'secondarySidebarWidth': secondarySidebarWidth,
        'bottomPanelVisible': bottomPanelVisible,
        'bottomPanelHeight': bottomPanelHeight,
        'statusBarVisible': statusBarVisible,
        'density': density,
        'windowMode': windowMode,
      };

  factory WorkbenchLayoutState.fromJson(Map<String, dynamic> json) {
    return WorkbenchLayoutState(
      activityBarVisible: json['activityBarVisible'] as bool? ?? true,
      primarySidebarVisible: json['primarySidebarVisible'] as bool? ?? true,
      primarySidebarWidth: (json['primarySidebarWidth'] as num?)?.toDouble() ?? 260,
      secondarySidebarVisible: json['secondarySidebarVisible'] as bool? ?? false,
      secondarySidebarWidth: (json['secondarySidebarWidth'] as num?)?.toDouble() ?? 280,
      bottomPanelVisible: json['bottomPanelVisible'] as bool? ?? true,
      bottomPanelHeight: (json['bottomPanelHeight'] as num?)?.toDouble() ?? 220,
      statusBarVisible: json['statusBarVisible'] as bool? ?? true,
      density: json['density'] as String? ?? 'standard',
      windowMode: json['windowMode'] as String? ?? 'workbench',
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WorkbenchLayoutState &&
          activityBarVisible == other.activityBarVisible &&
          primarySidebarVisible == other.primarySidebarVisible &&
          primarySidebarWidth == other.primarySidebarWidth &&
          secondarySidebarVisible == other.secondarySidebarVisible &&
          secondarySidebarWidth == other.secondarySidebarWidth &&
          bottomPanelVisible == other.bottomPanelVisible &&
          bottomPanelHeight == other.bottomPanelHeight &&
          statusBarVisible == other.statusBarVisible &&
          density == other.density &&
          windowMode == other.windowMode;

  @override
  int get hashCode => Object.hash(
        activityBarVisible, primarySidebarVisible, primarySidebarWidth,
        secondarySidebarVisible, secondarySidebarWidth, bottomPanelVisible,
        bottomPanelHeight, statusBarVisible, density, windowMode);
}
