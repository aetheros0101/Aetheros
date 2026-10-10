/// P0 command architecture contract.
///
/// Commands are application intents, not widget callbacks. The registry and
/// executor will be implemented before Command Center is introduced.
abstract final class AetherCommandArchitecture {
  static const registry = 'CommandRegistry';
  static const executor = 'CommandExecutor';
  static const keybindings = 'Keybindings';
}
