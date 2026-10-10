import 'agent_service.dart';
import 'git_service.dart';
import 'terminal_service.dart';
import 'workspace_service.dart';

/// Typed service access — no dynamic / string-key lookups.
class ServiceLocator {
  const ServiceLocator({
    required this.workspace,
    required this.git,
    required this.agent,
    required this.terminal,
  });

  final WorkspaceService workspace;
  final GitService git;
  final AgentService agent;
  final TerminalService terminal;

  void dispose() {
    workspace.dispose();
    git.dispose();
    agent.dispose();
    terminal.dispose();
  }
}
