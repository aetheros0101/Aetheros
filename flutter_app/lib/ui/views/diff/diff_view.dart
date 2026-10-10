import 'package:flutter/material.dart';

import '../../../api/aetheros_api.dart';
import '../../../src/rust/api/aetheros.dart' as rust;
import '../../design/aether_theme.dart';

/// Shows unified diff for a path (staged or worktree).
class DiffView extends StatefulWidget {
  const DiffView({
    super.key,
    required this.path,
    this.staged = false,
  });

  final String path;
  final bool staged;

  @override
  State<DiffView> createState() => _DiffViewState();
}

class _DiffViewState extends State<DiffView> {
  rust.GitDiffDto? _diff;
  String? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final d = await AetherApi.gitDiff(staged: widget.staged, path: widget.path);
      if (!mounted) return;
      setState(() {
        _diff = d;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return Center(
        child: Text(_error!,
            style: AetherTypography.uiBody.copyWith(color: AetherStatus.danger)),
      );
    }
    final files = _diff?.files ?? const [];
    if (files.isEmpty) {
      return Center(
        child: Text('No diff',
            style: AetherTypography.uiBody.copyWith(color: AetherTextColors.tertiary)),
      );
    }
    return ListView(
      padding: const EdgeInsets.all(AetherSpacing.sm),
      children: [
        for (final f in files) ...[
          Text(
            f.path,
            style: AetherTypography.fileName.copyWith(color: AetherAccent.primary),
          ),
          for (final h in f.hunks) ...[
            Text(h.header,
                style: AetherTypography.codeSmall
                    .copyWith(color: AetherTextColors.tertiary)),
            for (final line in h.lines)
              Text(
                line,
                style: AetherTypography.diffLine.copyWith(
                  color: line.startsWith('+')
                      ? AetherGit.added
                      : line.startsWith('-')
                          ? AetherGit.deleted
                          : AetherTextColors.primary,
                ),
              ),
          ],
          const SizedBox(height: 12),
        ],
      ],
    );
  }
}
