import '../navigation/navigation_state.dart';
import 'agent_state.dart';
import 'editor_state.dart';
import 'git_state.dart';
import 'panel_state.dart';
import 'problems_state.dart';
import 'terminal_state.dart';
import 'workbench_state.dart';
import 'workspace_state.dart';

/// Root application state — single serializable snapshot for persistence.
///
/// Domain slices owned by services (workspace/git/agent/terminal) are
/// mirrored here by [AetherApplication] so UI has one read model.
class AppState {
  const AppState({
    this.workbench = const WorkbenchLayoutState(),
    this.navigation = const NavigationState(),
    this.editor = const EditorState(),
    this.panel = const PanelState(),
    this.workspace = const WorkspaceState(),
    this.agent = const AgentWorkbenchState(),
    this.terminal = const TerminalState(),
    this.git = const GitState(),
    this.problems = const ProblemsState(),
  });

  final WorkbenchLayoutState workbench;

  /// Single source of truth for active activity + registered items.
  final NavigationState navigation;

  final EditorState editor;
  final PanelState panel;
  final WorkspaceState workspace;
  final AgentWorkbenchState agent;
  final TerminalState terminal;
  final GitState git;
  final ProblemsState problems;

  AppState copyWith({
    WorkbenchLayoutState? workbench,
    NavigationState? navigation,
    EditorState? editor,
    PanelState? panel,
    WorkspaceState? workspace,
    AgentWorkbenchState? agent,
    TerminalState? terminal,
    GitState? git,
    ProblemsState? problems,
  }) {
    return AppState(
      workbench: workbench ?? this.workbench,
      navigation: navigation ?? this.navigation,
      editor: editor ?? this.editor,
      panel: panel ?? this.panel,
      workspace: workspace ?? this.workspace,
      agent: agent ?? this.agent,
      terminal: terminal ?? this.terminal,
      git: git ?? this.git,
      problems: problems ?? this.problems,
    );
  }

  Map<String, dynamic> toJson() => {
        'workbench': workbench.toJson(),
        'navigation': navigation.toJson(),
        'editor': editor.toJson(),
        'panel': panel.toJson(),
        'workspace': workspace.toJson(),
        'agent': agent.toJson(),
        'terminal': terminal.toJson(),
        'git': git.toJson(),
        'problems': problems.toJson(),
      };

  factory AppState.fromJson(Map<String, dynamic> json) {
    return AppState(
      workbench: json['workbench'] != null
          ? WorkbenchLayoutState.fromJson(json['workbench'] as Map<String, dynamic>)
          : const WorkbenchLayoutState(),
      navigation: json['navigation'] != null
          ? NavigationState.fromJson(json['navigation'] as Map<String, dynamic>)
          : const NavigationState(),
      editor: json['editor'] != null
          ? EditorState.fromJson(json['editor'] as Map<String, dynamic>)
          : const EditorState(),
      panel: json['panel'] != null
          ? PanelState.fromJson(json['panel'] as Map<String, dynamic>)
          : const PanelState(),
      workspace: json['workspace'] != null
          ? WorkspaceState.fromJson(json['workspace'] as Map<String, dynamic>)
          : const WorkspaceState(),
      agent: json['agent'] != null
          ? AgentWorkbenchState.fromJson(json['agent'] as Map<String, dynamic>)
          : const AgentWorkbenchState(),
      terminal: json['terminal'] != null
          ? TerminalState.fromJson(json['terminal'] as Map<String, dynamic>)
          : const TerminalState(),
      git: json['git'] != null
          ? GitState.fromJson(json['git'] as Map<String, dynamic>)
          : const GitState(),
      problems: json['problems'] != null
          ? ProblemsState.fromJson(json['problems'] as Map<String, dynamic>)
          : const ProblemsState(),
    );
  }
}
