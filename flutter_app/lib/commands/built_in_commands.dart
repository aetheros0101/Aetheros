import '../navigation/navigation_item.dart';
import 'command.dart';

List<Command> builtInCommands() => [
      Command(
        id: 'workbench.toggleSidebar',
        title: 'Toggle Primary Sidebar',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(
                workbench: s.workbench.copyWith(
                  primarySidebarVisible: !s.workbench.primarySidebarVisible,
                ),
              ));
        },
      ),
      Command(
        id: 'workbench.toggleSecondarySidebar',
        title: 'Toggle Secondary Sidebar',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(
                workbench: s.workbench.copyWith(
                  secondarySidebarVisible: !s.workbench.secondarySidebarVisible,
                ),
              ));
        },
      ),
      Command(
        id: 'workbench.togglePanel',
        title: 'Toggle Bottom Panel',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(
                workbench: s.workbench.copyWith(
                  bottomPanelVisible: !s.workbench.bottomPanelVisible,
                ),
              ));
        },
      ),
      Command(
        id: 'workbench.toggleTerminal',
        title: 'Toggle Terminal',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) {
            final show = !s.workbench.bottomPanelVisible ||
                s.panel.activePanelId != 'terminal';
            return s.copyWith(
              workbench: s.workbench.copyWith(bottomPanelVisible: show),
              panel: s.panel.activate('terminal'),
            );
          });
        },
      ),
      Command(
        id: 'workbench.openExplorer',
        title: 'Show Explorer',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(
                navigation: s.navigation.activate(BuiltInActivities.explorer),
                workbench: s.workbench.copyWith(primarySidebarVisible: true),
              ));
        },
      ),
      Command(
        id: 'workbench.openSearch',
        title: 'Show Search',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(
                navigation: s.navigation.activate(BuiltInActivities.search),
                workbench: s.workbench.copyWith(primarySidebarVisible: true),
              ));
        },
      ),
      Command(
        id: 'workbench.openScm',
        title: 'Show Source Control',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(
                navigation: s.navigation.activate(BuiltInActivities.scm),
                workbench: s.workbench.copyWith(primarySidebarVisible: true),
              ));
        },
      ),
      Command(
        id: 'workbench.openAgent',
        title: 'Show Agent',
        category: 'View',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(
                navigation: s.navigation.activate(BuiltInActivities.agent),
                workbench: s.workbench.copyWith(secondarySidebarVisible: true),
              ));
        },
      ),
      Command(
        id: 'workbench.commandPalette',
        title: 'Command Palette',
        category: 'View',
        description: 'Open the command palette (shell-owned overlay)',
        handler: (ctx) async {
          // Intentionally empty: AetherWorkbench._run intercepts this id
          // and opens the overlay. Registered so palette search lists it.
        },
      ),
      Command(
        id: 'workbench.quickOpen',
        title: 'Quick Open',
        category: 'View',
        description: 'Open file by name (shell-owned overlay)',
        handler: (ctx) async {
          // Intentionally empty: intercepted by AetherWorkbench._run.
        },
      ),
      Command(
        id: 'editor.closeTab',
        title: 'Close Editor',
        category: 'Editor',
        description: 'Close active tab with dirty protection (shell-owned)',
        handler: (ctx) async {
          // Intercepted by AetherWorkbench._run for unsaved-changes dialog.
          // Fallback if executed without shell:
          ctx.updateState(
              (s) => s.copyWith(editor: s.editor.closeActiveTab()));
        },
      ),
      Command(
        id: 'editor.nextTab',
        title: 'Next Tab',
        category: 'Editor',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(editor: s.editor.nextTab()));
        },
      ),
      Command(
        id: 'editor.previousTab',
        title: 'Previous Tab',
        category: 'Editor',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(editor: s.editor.previousTab()));
        },
      ),
      Command(
        id: 'editor.split',
        title: 'Split Editor',
        category: 'Editor',
        handler: (ctx) async {
          ctx.updateState((s) => s.copyWith(editor: s.editor.split()));
        },
      ),
      Command(
        id: 'editor.openFile',
        title: 'Open File',
        category: 'Editor',
        handler: (ctx) async {
          final path = ctx.arg<String>('path');
          if (path == null) return;
          final preview = ctx.arg<bool>('preview') ?? false;
          ctx.updateState(
              (s) => s.copyWith(editor: s.editor.openFile(path, preview: preview)));
        },
      ),
      Command(
        id: 'editor.save',
        title: 'Save',
        category: 'Editor',
        description: 'Save active editor (shell-owned; conflict UI)',
        handler: (ctx) async {
          // Intercepted by AetherWorkbench._run for path + conflict dialog.
        },
      ),
      Command(
        id: 'editor.undo',
        title: 'Undo',
        category: 'Editor',
        description: 'Undo last edit in active editor (shell-owned)',
        handler: (ctx) async {
          // Intercepted by AetherWorkbench._run → EditorController.undo
        },
      ),
      Command(
        id: 'editor.redo',
        title: 'Redo',
        category: 'Editor',
        description: 'Redo in active editor (shell-owned)',
        handler: (ctx) async {
          // Intercepted by AetherWorkbench._run → EditorController.redo
        },
      ),
      Command(
        id: 'editor.saveAll',
        title: 'Save All',
        category: 'Editor',
        description: 'Save all dirty editors (shell-owned)',
        handler: (ctx) async {
          // Intercepted by AetherWorkbench._run
        },
      ),
      Command(
        id: 'agent.newSession',
        title: 'New Agent Session',
        category: 'Agent',
        handler: (ctx) async {
          await ctx.services.agent.newSession();
          ctx.updateState((s) => s.copyWith(
                navigation: s.navigation.activate(BuiltInActivities.agent),
                workbench: s.workbench.copyWith(secondarySidebarVisible: true),
              ));
        },
      ),
      Command(
        id: 'agent.stop',
        title: 'Stop Agent',
        category: 'Agent',
        handler: (ctx) async {
          final id = ctx.state.agent.activeSessionId;
          if (id == null) return;
          await ctx.services.agent.stop(id);
        },
      ),
      Command(
        id: 'workspace.refresh',
        title: 'Refresh Explorer',
        category: 'Workspace',
        handler: (ctx) async {
          await ctx.services.workspace.refresh();
        },
      ),
      Command(
        id: 'git.refresh',
        title: 'Git: Refresh',
        category: 'Git',
        handler: (ctx) async {
          await ctx.services.git.refresh();
        },
      ),
    ];
