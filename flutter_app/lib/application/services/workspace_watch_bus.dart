import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../api/aetheros_api.dart';
import 'workspace_service.dart';

/// Pushes filesystem events from Rust [workspace_watch_drain] into a Dart stream.
///
/// This is an event pump (short drain interval), not content polling.
/// Primary external-change path for the workbench.
class WorkspaceWatchBus {
  WorkspaceWatchBus({
    this.drainInterval = const Duration(milliseconds: 250),
    this.maxEventsPerDrain = 64,
  });

  final Duration drainInterval;
  final int maxEventsPerDrain;

  final _controller = StreamController<WorkspaceChangeEvent>.broadcast();
  Timer? _timer;
  bool _running = false;
  bool _watchUnavailable = false;

  Stream<WorkspaceChangeEvent> get events => _controller.stream;

  Future<void> start() async {
    if (_running) return;
    _running = true;
    try {
      await AetherApi.workspaceWatchStart();
    } catch (e) {
      // Expected when watch feature is off or backend not ready.
      _watchUnavailable = true;
      debugPrint('[WorkspaceWatchBus] watch start unavailable: $e');
    }
    _timer = Timer.periodic(drainInterval, (_) => _tick());
  }

  Future<void> _tick() async {
    if (_controller.isClosed || _watchUnavailable) return;
    try {
      final hits = await AetherApi.workspaceWatchDrain(maxEvents: maxEventsPerDrain);
      for (final h in hits) {
        _controller.add(WorkspaceChangeEvent(
          path: h.path,
          kind: switch (h.kind) {
            'created' => WorkspaceChangeKind.created,
            'removed' => WorkspaceChangeKind.removed,
            'renamed' => WorkspaceChangeKind.renamed,
            _ => WorkspaceChangeKind.modified,
          },
          oldPath: h.oldPath,
        ));
      }
    } catch (e) {
      // Drain failures are non-fatal; avoid spamming every 250ms after first.
      debugPrint('[WorkspaceWatchBus] drain error: $e');
      _watchUnavailable = true;
    }
  }

  void stop() {
    _timer?.cancel();
    _timer = null;
    _running = false;
  }

  void dispose() {
    stop();
    _controller.close();
  }
}
