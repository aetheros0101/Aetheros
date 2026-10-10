import 'package:flutter/material.dart';

import '../foundation/colors.dart';

/// Git change kind used across explorer, diff, SCM views.
enum AetherGitChange {
  added,
  modified,
  deleted,
  renamed,
  untracked,
  conflict,
  ignored,
  staged,
}

/// AetherOS Semantic — Git State Colors
///
/// First-class git visual language. Same colors in explorer badges,
/// diff gutters, SCM lists, and status bar.
abstract final class AetherGit {
  static const Color added = AetherColorPrimitives.green400;
  static const Color modified = AetherColorPrimitives.amber400;
  static const Color deleted = AetherColorPrimitives.red400;
  static const Color renamed = AetherColorPrimitives.blue400;
  static const Color untracked = AetherColorPrimitives.teal400;
  static const Color conflict = AetherColorPrimitives.purple400;
  static const Color ignored = AetherColorPrimitives.neutral600;
  static const Color staged = AetherColorPrimitives.accent400;

  /// Diff line backgrounds (editor gutter / inline diff).
  static final Color diffAddedBg =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.green400, 0.12);
  static final Color diffDeletedBg =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.red400, 0.12);
  static final Color diffModifiedBg =
      AetherColorPrimitives.withAlpha(AetherColorPrimitives.amber400, 0.10);

  static Color colorOf(AetherGitChange change) {
    switch (change) {
      case AetherGitChange.added:
        return added;
      case AetherGitChange.modified:
        return modified;
      case AetherGitChange.deleted:
        return deleted;
      case AetherGitChange.renamed:
        return renamed;
      case AetherGitChange.untracked:
        return untracked;
      case AetherGitChange.conflict:
        return conflict;
      case AetherGitChange.ignored:
        return ignored;
      case AetherGitChange.staged:
        return staged;
    }
  }

  /// Single-letter badge label (M, A, D, R, U, C, I, S).
  static String badgeOf(AetherGitChange change) {
    switch (change) {
      case AetherGitChange.added:
        return 'A';
      case AetherGitChange.modified:
        return 'M';
      case AetherGitChange.deleted:
        return 'D';
      case AetherGitChange.renamed:
        return 'R';
      case AetherGitChange.untracked:
        return 'U';
      case AetherGitChange.conflict:
        return 'C';
      case AetherGitChange.ignored:
        return 'I';
      case AetherGitChange.staged:
        return 'S';
    }
  }
}
