import 'package:flutter/material.dart';

import '../../../application/services/workspace_service.dart';
import '../../../state/app_state.dart';
import '../../design/aether_theme.dart';

class SearchView extends StatefulWidget {
  const SearchView({
    super.key,
    required this.state,
    required this.workspace,
    required this.onOpenResult,
  });

  final AppState state;
  final WorkspaceService workspace;
  final void Function(String path, int line) onOpenResult;

  @override
  State<SearchView> createState() => _SearchViewState();
}

class _SearchViewState extends State<SearchView> {
  final _query = TextEditingController();
  List<SearchHit> _hits = [];
  bool _searching = false;
  String? _error;
  bool _hasRun = false;

  /// Monotonic generation to ignore stale async results.
  int _searchGen = 0;

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  Future<void> _run() async {
    final q = _query.text.trim();
    final gen = ++_searchGen;

    if (q.isEmpty) {
      if (!mounted) return;
      setState(() {
        _hits = [];
        _searching = false;
        _error = null;
        _hasRun = false;
      });
      return;
    }

    setState(() {
      _searching = true;
      _error = null;
      _hasRun = true;
    });

    try {
      final hits = await widget.workspace.search(q);
      if (!mounted || gen != _searchGen) return;
      setState(() {
        _hits = hits;
        _searching = false;
        _error = null;
      });
    } catch (e) {
      if (!mounted || gen != _searchGen) return;
      setState(() {
        _hits = [];
        _searching = false;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(AetherSpacing.sm),
          child: TextField(
            controller: _query,
            style: AetherTypography.uiBody.copyWith(color: AetherTextColors.primary),
            decoration: InputDecoration(
              hintText: 'Search',
              isDense: true,
              suffixIcon: IconButton(
                icon: const Icon(AetherIcons.search, size: 16),
                onPressed: _run,
              ),
            ),
            onSubmitted: (_) => _run(),
          ),
        ),
        if (_searching) const LinearProgressIndicator(minHeight: 2),
        Expanded(child: _body()),
      ],
    );
  }

  Widget _body() {
    if (_searching) {
      return const SizedBox.shrink();
    }
    if (_error != null) {
      return Padding(
        padding: const EdgeInsets.all(AetherSpacing.md),
        child: Text(
          _error!,
          style: AetherTypography.uiCaption.copyWith(color: AetherStatus.danger),
        ),
      );
    }
    if (_hasRun && _hits.isEmpty) {
      return Center(
        child: Text(
          'No results',
          style: AetherTypography.uiCaption.copyWith(
            color: AetherTextColors.tertiary,
          ),
        ),
      );
    }
    if (!_hasRun) {
      return Center(
        child: Text(
          'Enter a query to search the workspace',
          style: AetherTypography.uiCaption.copyWith(
            color: AetherTextColors.tertiary,
          ),
        ),
      );
    }
    return ListView.builder(
      itemCount: _hits.length,
      itemBuilder: (context, i) {
        final h = _hits[i];
        final name = h.path.split('/').last;
        return InkWell(
          onTap: () => widget.onOpenResult(h.path, h.line),
          child: Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: AetherSpacing.md,
              vertical: AetherSpacing.xs,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '$name:${h.line}',
                  style: AetherTypography.fileName.copyWith(
                    color: AetherAccent.primary,
                  ),
                ),
                Text(
                  h.preview,
                  style: AetherTypography.codeSmall.copyWith(
                    color: AetherTextColors.secondary,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
