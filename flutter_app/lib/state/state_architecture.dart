/// Workbench state ownership contract (WB-201).
///
/// ## Single authority per slice
///
/// | Slice | Authority (source of truth) | UI mirror |
/// |-------|-----------------------------|-----------|
/// | Workspace tree / root | [WorkspaceService] (`FrbWorkspaceService` / stub) | [AppState.workspace] |
/// | Git status / changes | [GitService] | [AppState.git] |
/// | Agent sessions | [AgentService] | [AppState.agent] |
/// | Terminal sessions | [TerminalService] | [AppState.terminal] |
/// | Editor tabs / groups | [EditorState] via commands + shell | [AppState.editor] |
/// | Open document buffers | [EditorSession] (controllers) | not duplicated in AppState |
/// | Layout (sidebars, panel) | [WorkbenchLayoutState] via commands | [AppState.workbench] |
/// | Active activity | [NavigationState] | [AppState.navigation] |
/// | Bottom panel tab | [PanelState] | [AppState.panel] |
/// | Problems | [ProblemsState] | [AppState.problems] |
///
/// ## Rules
///
/// 1. Services own domain state and emit on their `changes` streams.
/// 2. [AetherApplication] mirrors service state into [AppState] (one-way).
/// 3. Widgets must not call Rust/FRB directly; they use services or commands.
/// 4. Widgets update layout/navigation/editor tab state only through
///    `updateState` / command handlers — not by mutating service state.
/// 5. Document content lives in [EditorSession] controllers; dirty flags
///    should stay consistent with those controllers (WB-103).
/// 6. No bidirectional sync loops: service → AppState only.
///
/// ## Async races
///
/// - Search / listFiles: use generation counters in the view (see SearchView).
/// - Watch bus: external disk events → shell may offer reload dialog.
/// - Subscriptions: cancelled in [AetherApplication.dispose].
abstract final class AetherStateArchitecture {
  static const workbench = 'WorkbenchLayoutState';
  static const workspace = 'WorkspaceState';
  static const editor = 'EditorState';
  static const editorBuffers = 'EditorSession';
  static const agent = 'AgentWorkbenchState';
  static const terminal = 'TerminalState';
  static const git = 'GitState';
  static const problems = 'ProblemsState';
  static const navigation = 'NavigationState';
  static const panel = 'PanelState';
}
