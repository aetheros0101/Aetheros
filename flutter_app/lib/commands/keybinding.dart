/// Platform-aware keybinding descriptor.
///
/// Actual [ShortcutActivator] wiring happens in the shell layer so this
/// module stays Flutter-light (only string chords stored here).
class Keybinding {
  const Keybinding({
    required this.commandId,
    required this.chord,
    this.when,
    this.macChord,
  });

  final String commandId;

  /// Default chord, e.g. "ctrl+b", "ctrl+shift+p".
  final String chord;

  /// Optional macOS override, e.g. "meta+b".
  final String? macChord;

  /// Optional context expression (reserved for future when-clauses).
  final String? when;

  String chordFor({required bool isMac}) =>
      isMac ? (macChord ?? chord.replaceAll('ctrl', 'meta')) : chord;
}

/// Default workbench keybindings (P2 baseline).
abstract final class DefaultKeybindings {
  static const List<Keybinding> all = [
    Keybinding(commandId: 'workbench.commandPalette', chord: 'ctrl+shift+p', macChord: 'meta+shift+p'),
    Keybinding(commandId: 'workbench.quickOpen', chord: 'ctrl+p', macChord: 'meta+p'),
    Keybinding(commandId: 'workbench.toggleSidebar', chord: 'ctrl+b', macChord: 'meta+b'),
    Keybinding(commandId: 'workbench.togglePanel', chord: 'ctrl+j', macChord: 'meta+j'),
    Keybinding(commandId: 'editor.closeTab', chord: 'ctrl+w', macChord: 'meta+w'),
    Keybinding(commandId: 'workbench.openSearch', chord: 'ctrl+shift+f', macChord: 'meta+shift+f'),
    Keybinding(commandId: 'editor.nextTab', chord: 'ctrl+tab', macChord: 'meta+tab'),
    Keybinding(commandId: 'editor.split', chord: 'ctrl+\\', macChord: 'meta+\\'),
    Keybinding(commandId: 'agent.newSession', chord: 'ctrl+shift+a', macChord: 'meta+shift+a'),
  ];
}
