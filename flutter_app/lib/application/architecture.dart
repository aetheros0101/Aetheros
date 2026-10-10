/// AetherOS application-layer boundary.
///
/// P0 contract: this layer coordinates UI/application state and commands.
/// It must not depend on concrete widgets or legacy screens.
///
/// Backend access remains behind `api/` and existing services during the
/// migration. No runtime behaviour is changed by this contract.
abstract final class AetherApplicationArchitecture {
  static const version = '0.1';
  static const legacyUiBoundary = 'screens/';
  static const backendApiBoundary = 'api/';
  static const serviceBoundary = 'services/';
  static const rustBridgeBoundary = 'src/rust/';
}
