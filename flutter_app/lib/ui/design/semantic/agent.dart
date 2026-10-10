import 'package:flutter/material.dart';

import '../foundation/colors.dart';
import '../foundation/icons.dart';

/// Agent lifecycle states — drives Agent Activity, status bar, thread headers.
enum AetherAgentState {
  idle,
  thinking,
  planning,
  executing,
  waitingApproval,
  verifying,
  completed,
  failed,
  cancelled,
}

/// Task lifecycle states — TaskGraph, task list, approval queues.
enum AetherTaskState {
  queued,
  ready,
  running,
  blocked,
  completed,
  failed,
  cancelled,
}

/// AetherOS Semantic — Agent & Task State Tokens
///
/// Visual principle:
///   "Color communicates state; it does not decorate the interface."
///
/// Default UI uses **icon + subtle tint**.
/// Full-saturation color is reserved for:
///   - state badges
///   - critical status indicators
///   - active-agent pulse
abstract final class AetherAgent {
  static const Color idle = AetherColorPrimitives.neutral600;
  static const Color thinking = AetherColorPrimitives.accent400;
  static const Color planning = AetherColorPrimitives.blue400;
  static const Color executing = AetherColorPrimitives.teal400;
  static const Color waitingApproval = AetherColorPrimitives.amber400;
  static const Color verifying = AetherColorPrimitives.cyan400;
  static const Color completed = AetherColorPrimitives.green400;
  static const Color failed = AetherColorPrimitives.red400;
  static const Color cancelled = AetherColorPrimitives.neutral600;

  static Color colorOf(AetherAgentState state) {
    switch (state) {
      case AetherAgentState.idle:
        return idle;
      case AetherAgentState.thinking:
        return thinking;
      case AetherAgentState.planning:
        return planning;
      case AetherAgentState.executing:
        return executing;
      case AetherAgentState.waitingApproval:
        return waitingApproval;
      case AetherAgentState.verifying:
        return verifying;
      case AetherAgentState.completed:
        return completed;
      case AetherAgentState.failed:
        return failed;
      case AetherAgentState.cancelled:
        return cancelled;
    }
  }

  /// Subtle tint for row/card backgrounds (low alpha — does not decorate).
  static Color tintOf(AetherAgentState state) =>
      AetherColorPrimitives.withAlpha(colorOf(state), 0.10);

  static Color containerOf(AetherAgentState state) =>
      AetherColorPrimitives.withAlpha(colorOf(state), 0.14);

  static String labelOf(AetherAgentState state) {
    switch (state) {
      case AetherAgentState.idle:
        return 'Idle';
      case AetherAgentState.thinking:
        return 'Thinking';
      case AetherAgentState.planning:
        return 'Planning';
      case AetherAgentState.executing:
        return 'Executing';
      case AetherAgentState.waitingApproval:
        return 'Waiting for approval';
      case AetherAgentState.verifying:
        return 'Verifying';
      case AetherAgentState.completed:
        return 'Completed';
      case AetherAgentState.failed:
        return 'Failed';
      case AetherAgentState.cancelled:
        return 'Cancelled';
    }
  }

  /// Semantic icon — primary signal; color is secondary.
  static IconData iconOf(AetherAgentState state) {
    switch (state) {
      case AetherAgentState.idle:
        return AetherIcons.pending;
      case AetherAgentState.thinking:
        return AetherIcons.ai;
      case AetherAgentState.planning:
        return AetherIcons.workflow;
      case AetherAgentState.executing:
        return AetherIcons.running;
      case AetherAgentState.waitingApproval:
        return AetherIcons.approval;
      case AetherAgentState.verifying:
        return AetherIcons.info;
      case AetherAgentState.completed:
        return AetherIcons.success;
      case AetherAgentState.failed:
        return AetherIcons.error;
      case AetherAgentState.cancelled:
        return AetherIcons.cancelled;
    }
  }
}

abstract final class AetherTask {
  static const Color queued = AetherColorPrimitives.neutral600;
  static const Color ready = AetherColorPrimitives.blue400;
  static const Color running = AetherColorPrimitives.teal400;
  static const Color blocked = AetherColorPrimitives.amber400;
  static const Color completed = AetherColorPrimitives.green400;
  static const Color failed = AetherColorPrimitives.red400;
  static const Color cancelled = AetherColorPrimitives.neutral600;

  static Color colorOf(AetherTaskState state) {
    switch (state) {
      case AetherTaskState.queued:
        return queued;
      case AetherTaskState.ready:
        return ready;
      case AetherTaskState.running:
        return running;
      case AetherTaskState.blocked:
        return blocked;
      case AetherTaskState.completed:
        return completed;
      case AetherTaskState.failed:
        return failed;
      case AetherTaskState.cancelled:
        return cancelled;
    }
  }

  static Color tintOf(AetherTaskState state) =>
      AetherColorPrimitives.withAlpha(colorOf(state), 0.10);

  static Color containerOf(AetherTaskState state) =>
      AetherColorPrimitives.withAlpha(colorOf(state), 0.14);

  static String labelOf(AetherTaskState state) {
    switch (state) {
      case AetherTaskState.queued:
        return 'Queued';
      case AetherTaskState.ready:
        return 'Ready';
      case AetherTaskState.running:
        return 'Running';
      case AetherTaskState.blocked:
        return 'Blocked';
      case AetherTaskState.completed:
        return 'Completed';
      case AetherTaskState.failed:
        return 'Failed';
      case AetherTaskState.cancelled:
        return 'Cancelled';
    }
  }

  static IconData iconOf(AetherTaskState state) {
    switch (state) {
      case AetherTaskState.queued:
        return AetherIcons.pending;
      case AetherTaskState.ready:
        return AetherIcons.task;
      case AetherTaskState.running:
        return AetherIcons.running;
      case AetherTaskState.blocked:
        return AetherIcons.blocked;
      case AetherTaskState.completed:
        return AetherIcons.success;
      case AetherTaskState.failed:
        return AetherIcons.error;
      case AetherTaskState.cancelled:
        return AetherIcons.cancelled;
    }
  }
}

abstract final class AetherApproval {
  static const Color pending = AetherColorPrimitives.amber400;
  static const Color approved = AetherColorPrimitives.green400;
  static const Color rejected = AetherColorPrimitives.red400;
  static const Color expired = AetherColorPrimitives.neutral600;

  static IconData get pendingIcon => AetherIcons.approval;
  static IconData get approvedIcon => AetherIcons.success;
  static IconData get rejectedIcon => AetherIcons.error;
}
