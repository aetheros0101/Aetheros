import 'package:flutter/material.dart';

import '../../application/services/agent_service.dart';
import '../../state/app_state.dart';
import '../design/aether_theme.dart';
import '../views/agent/agent_panel.dart';

class AetherSecondarySidebar extends StatelessWidget {
  const AetherSecondarySidebar({
    super.key,
    required this.state,
    required this.onStateUpdate,
    required this.agentService,
  });

  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final AgentService agentService;

  @override
  Widget build(BuildContext context) {
    final width = state.workbench.clampedSecondarySidebarWidth;
    return SizedBox(
      width: width,
      child: Container(
        color: AetherSurfaces.sidebar,
        child: AgentPanel(
          state: state,
          agentService: agentService,
        ),
      ),
    );
  }
}
