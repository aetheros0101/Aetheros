import 'package:flutter/material.dart';

import '../../../application/services/git_service.dart';
import '../../../state/app_state.dart';
import '../../../state/git_state.dart';
import '../../design/aether_theme.dart';

class ScmView extends StatefulWidget {
  const ScmView({
    super.key,
    required this.state,
    required this.git,
    this.onOpenFile,
  });

  final AppState state;
  final GitService git;
  final Future<void> Function(String path)? onOpenFile;

  @override
  State<ScmView> createState() => _ScmViewState();
}

class _ScmViewState extends State<ScmView> {
  final _message = TextEditingController();

  @override
  void dispose() {
    _message.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final g = widget.state.git;
    final staged = g.changes.where((c) => c.staged).toList();
    final unstaged = g.changes.where((c) => !c.staged).toList();
    final errLower = g.error?.toLowerCase() ?? '';
    final noRepo = g.branch == null &&
        g.changes.isEmpty &&
        g.error != null &&
        (errLower.contains('not a git') ||
            errLower.contains('no repository') ||
            errLower.contains('not a repo') ||
            errLower.contains('no git'));

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.all(AetherSpacing.sm),
          child: Row(
            children: [
              Icon(AetherIcons.branch,
                  size: 14, color: AetherTextColors.secondary),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  g.branch ?? (noRepo ? 'No repository' : '—'),
                  style: AetherTypography.uiCaption.copyWith(
                    color: AetherTextColors.primary,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              if (g.isLoading)
                const SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              else
                IconButton(
                  icon: const Icon(AetherIcons.refresh, size: 16),
                  color: AetherTextColors.secondary,
                  onPressed: () => widget.git.refresh(),
                  padding: EdgeInsets.zero,
                  constraints:
                      const BoxConstraints(minWidth: 28, minHeight: 28),
                ),
            ],
          ),
        ),
        if (!noRepo)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.sm),
            child: TextField(
              controller: _message,
              style: AetherTypography.uiBody
                  .copyWith(color: AetherTextColors.primary),
              decoration: InputDecoration(
                hintText: 'Commit message',
                isDense: true,
                suffixIcon: IconButton(
                  icon: const Icon(AetherIcons.check, size: 16),
                  onPressed: g.isLoading
                      ? null
                      : () async {
                          final msg = _message.text.trim();
                          if (msg.isEmpty) return;
                          await widget.git.commit(msg);
                          if (mounted) _message.clear();
                        },
                ),
              ),
            ),
          ),
        if (g.error != null)
          Padding(
            padding: const EdgeInsets.all(AetherSpacing.sm),
            child: Text(
              g.error!,
              style: AetherTypography.uiMicro
                  .copyWith(color: AetherStatus.danger),
            ),
          ),
        Expanded(
          child: noRepo
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(AetherSpacing.md),
                    child: Text(
                      'No Git repository detected for this workspace.\n'
                      'Open a folder that is a Git repo, or initialize one externally.',
                      textAlign: TextAlign.center,
                      style: AetherTypography.uiCaption.copyWith(
                        color: AetherTextColors.tertiary,
                      ),
                    ),
                  ),
                )
              : ListView(
                  padding: const EdgeInsets.all(AetherSpacing.sm),
                  children: [
                    if (staged.isEmpty && unstaged.isEmpty && !g.isLoading)
                      Padding(
                        padding: const EdgeInsets.symmetric(
                            vertical: AetherSpacing.md),
                        child: Text(
                          'No changes',
                          style: AetherTypography.uiCaption.copyWith(
                            color: AetherTextColors.tertiary,
                          ),
                        ),
                      ),
                    _Section(
                      title: 'STAGED',
                      entries: staged,
                      onOpen: widget.onOpenFile,
                      onAction: (c) => widget.git.unstage(c.path),
                      actionLabel: 'Unstage',
                    ),
                    _Section(
                      title: 'CHANGES',
                      entries: unstaged,
                      onOpen: widget.onOpenFile,
                      onAction: (c) => widget.git.stage(c.path),
                      actionLabel: 'Stage',
                    ),
                  ],
                ),
        ),
      ],
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({
    required this.title,
    required this.entries,
    required this.onAction,
    required this.actionLabel,
    this.onOpen,
  });

  final String title;
  final List<GitChangeEntry> entries;
  final Future<void> Function(String path)? onOpen;
  final void Function(GitChangeEntry) onAction;
  final String actionLabel;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Text(
            '$title (${entries.length})',
            style: AetherTypography.uiMicro.copyWith(
              color: AetherTextColors.tertiary,
              letterSpacing: 0.6,
            ),
          ),
        ),
        for (final e in entries)
          InkWell(
            onTap: onOpen == null ? null : () => onOpen!(e.path),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Row(
                children: [
                  Text(
                    _letter(e.status),
                    style: AetherTypography.uiMicro.copyWith(
                      color: _color(e.status),
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      e.path,
                      style: AetherTypography.fileName.copyWith(
                        color: AetherTextColors.primary,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  TextButton(
                    onPressed: () => onAction(e),
                    child: Text(
                      actionLabel,
                      style: AetherTypography.uiMicro.copyWith(
                        color: AetherAccent.primary,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
      ],
    );
  }

  String _letter(String s) => switch (s) {
        'added' => 'A',
        'modified' => 'M',
        'deleted' => 'D',
        'renamed' => 'R',
        'untracked' => 'U',
        'conflict' => 'C',
        _ => 'M',
      };

  Color _color(String s) => switch (s) {
        'added' => AetherGit.added,
        'modified' => AetherGit.modified,
        'deleted' => AetherGit.deleted,
        'untracked' => AetherGit.untracked,
        'conflict' => AetherGit.conflict,
        _ => AetherTextColors.secondary,
      };
}
