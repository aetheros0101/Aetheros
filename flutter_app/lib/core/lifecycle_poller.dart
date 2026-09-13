import 'dart:async';
import 'package:flutter/widgets.dart';

/// Screen lifecycle-aware polling. Stops while backgrounded and guarantees
/// that only one in-flight callback is active at a time.
final class LifecyclePoller with WidgetsBindingObserver {
  final Duration interval;
  final Future<void> Function() onTick;
  Timer? _timer;
  bool _active = false;
  bool _running = false;

  LifecyclePoller({required this.interval, required this.onTick});

  void start() {
    if (_active) return;
    _active = true;
    WidgetsBinding.instance.addObserver(this);
    _schedule();
    unawaited(_tick());
  }

  void stop() {
    _active = false;
    _timer?.cancel();
    _timer = null;
    WidgetsBinding.instance.removeObserver(this);
  }

  void _schedule() {
    _timer?.cancel();
    if (!_active) return;
    _timer = Timer(interval, () {
      unawaited(_tick());
    });
  }

  Future<void> _tick() async {
    if (!_active || _running) return;
    _running = true;
    try {
      await onTick();
    } finally {
      _running = false;
      _schedule();
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (!_active) return;
    if (state == AppLifecycleState.resumed) {
      unawaited(_tick());
    } else if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.inactive ||
        state == AppLifecycleState.hidden) {
      _timer?.cancel();
      _timer = null;
    }
  }
}
