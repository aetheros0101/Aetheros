/// P0 navigation contract.
///
/// Workbench navigation is independent from the legacy Navigator routes used
/// by existing screens. Migration can therefore happen feature by feature.
abstract final class AetherNavigationArchitecture {
  static const activity = 'Activity';
  static const primarySidebar = 'PrimarySidebar';
  static const secondarySidebar = 'SecondarySidebar';
  static const editor = 'Editor';
  static const bottomPanel = 'BottomPanel';
}
