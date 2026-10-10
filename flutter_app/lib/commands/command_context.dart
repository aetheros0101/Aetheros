import '../application/services/service_locator.dart';
import '../state/app_state.dart';

/// Runtime context passed to every command handler.
///
/// Typed [services] — no string-key / dynamic lookups.
class CommandContext {
  CommandContext({
    required this.state,
    required this.updateState,
    required this.services,
    this.args = const {},
  });

  final AppState state;
  final void Function(AppState Function(AppState) updater) updateState;
  final ServiceLocator services;
  final Map<String, Object?> args;

  T? arg<T>(String key) {
    final v = args[key];
    return v is T ? v : null;
  }
}
