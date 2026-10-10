import 'package:flutter/material.dart';

import '../../../agent/agent_models.dart';
import '../../../application/services/agent_service.dart';
import '../../../state/agent_state.dart';
import '../../../state/app_state.dart';
import '../../design/aether_theme.dart';

class AgentPanel extends StatelessWidget {
  const AgentPanel({
    super.key,
    required this.state,
    required this.agentService,
  });

  final AppState state;
  final AgentService agentService;

  @override
  Widget build(BuildContext context) {
    final session = state.agent.activeSession;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Header(session: session, onNew: () => agentService.newSession()),
        Expanded(
          child: session == null
              ? Center(
                  child: Text(
                    'Start an agent session',
                    style: AetherTypography.uiCaption.copyWith(
                      color: AetherTextColors.tertiary,
                    ),
                  ),
                )
              : ListView(
                  padding: const EdgeInsets.all(AetherSpacing.sm),
                  children: [
                    if (session.intent != null) ...[
                      Text('INTENT',
                          style: AetherTypography.uiMicro.copyWith(
                            color: AetherTextColors.tertiary,
                            letterSpacing: 0.6,
                          )),
                      const SizedBox(height: 4),
                      Text(session.intent!,
                          style: AetherTypography.agentMessage.copyWith(
                            color: AetherTextColors.primary,
                          )),
                      const SizedBox(height: 12),
                    ],
                    if (session.tasks.isNotEmpty) ...[
                      Text('PLAN',
                          style: AetherTypography.uiMicro.copyWith(
                            color: AetherTextColors.tertiary,
                            letterSpacing: 0.6,
                          )),
                      const SizedBox(height: 4),
                      for (final t in session.tasks) _TaskRow(task: t),
                      const SizedBox(height: 12),
                    ],
                    if (session.activity.isNotEmpty) ...[
                      Text('ACTIVITY',
                          style: AetherTypography.uiMicro.copyWith(
                            color: AetherTextColors.tertiary,
                            letterSpacing: 0.6,
                          )),
                      const SizedBox(height: 4),
                      for (final a in session.activity)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 4),
                          child: Text(
                            '▸ ${a.summary}',
                            style: AetherTypography.agentTool.copyWith(
                              color: AetherTextColors.secondary,
                            ),
                          ),
                        ),
                      const SizedBox(height: 12),
                    ],
                    if (session.approvals.any((a) => a.status == ApprovalStatus.pending)) ...[
                      Text('APPROVAL REQUIRED',
                          style: AetherTypography.uiMicro.copyWith(
                            color: AetherStatus.warning,
                            letterSpacing: 0.6,
                          )),
                      const SizedBox(height: 4),
                      for (final a in session.approvals)
                        if (a.status == ApprovalStatus.pending)
                          _ApprovalCard(
                            approval: a,
                            onApprove: () => agentService.approve(a.id),
                            onReject: () => agentService.reject(a.id),
                          ),
                      const SizedBox(height: 12),
                    ],
                    for (final m in session.messages)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: Text(
                          '${m.role}: ${m.content}',
                          style: AetherTypography.agentMessage.copyWith(
                            color: m.role == 'user'
                                ? AetherTextColors.primary
                                : AetherTextColors.secondary,
                          ),
                        ),
                      ),
                  ],
                ),
        ),
        _Composer(
          enabled: session != null,
          onSubmit: (text) {
            final id = session?.id;
            if (id == null || text.trim().isEmpty) return;
            agentService.sendMessage(id, text.trim());
          },
        ),
      ],
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.session, required this.onNew});
  final AgentSessionState? session;
  final VoidCallback onNew;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 36,
      padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.sm),
      child: Row(
        children: [
          Icon(AetherIcons.agent, size: 14, color: AetherTextColors.secondary),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              session?.title ?? 'AGENT',
              style: AetherTypography.uiMicro.copyWith(
                color: AetherTextColors.secondary,
                letterSpacing: 0.6,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          IconButton(
            icon: const Icon(AetherIcons.add, size: 16),
            color: AetherTextColors.secondary,
            onPressed: onNew,
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
          ),
        ],
      ),
    );
  }
}

class _TaskRow extends StatelessWidget {
  const _TaskRow({required this.task});
  final AgentTaskNode task;

  @override
  Widget build(BuildContext context) {
    final icon = switch (task.status) {
      AgentTaskStatus.completed => AetherIcons.success,
      AgentTaskStatus.running => AetherIcons.running,
      AgentTaskStatus.failed => AetherIcons.error,
      _ => AetherIcons.pending,
    };
    final color = switch (task.status) {
      AgentTaskStatus.completed => AetherStatus.success,
      AgentTaskStatus.running => AetherAccent.primary,
      AgentTaskStatus.failed => AetherStatus.danger,
      _ => AetherTextColors.tertiary,
    };
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              task.title,
              style: AetherTypography.taskTitle.copyWith(
                color: AetherTextColors.primary,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Composer extends StatefulWidget {
  const _Composer({required this.enabled, required this.onSubmit});
  final bool enabled;
  final ValueChanged<String> onSubmit;

  @override
  State<_Composer> createState() => _ComposerState();
}

class _ComposerState extends State<_Composer> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    widget.onSubmit(_controller.text);
    _controller.clear();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(AetherSpacing.sm),
      decoration: BoxDecoration(border: Border(top: AetherBorders.subtle)),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: _controller,
              enabled: widget.enabled,
              style: AetherTypography.agentMessage
                  .copyWith(color: AetherTextColors.primary),
              onSubmitted: (_) => _submit(),
              decoration: InputDecoration(
                hintText: 'Message agent…',
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: AetherSpacing.sm,
                  vertical: AetherSpacing.sm,
                ),
                border: OutlineInputBorder(
                  borderRadius: AetherRadius.inputR,
                  borderSide: AetherBorders.subtle,
                ),
                enabledBorder: OutlineInputBorder(
                  borderRadius: AetherRadius.inputR,
                  borderSide: AetherBorders.subtle,
                ),
                focusedBorder: OutlineInputBorder(
                  borderRadius: AetherRadius.inputR,
                  borderSide: AetherBorders.focus,
                ),
              ),
            ),
          ),
          const SizedBox(width: AetherSpacing.xs),
          IconButton(
            icon: const Icon(AetherIcons.submit, size: 18),
            color: AetherAccent.primary,
            onPressed: widget.enabled ? _submit : null,
          ),
        ],
      ),
    );
  }
}
