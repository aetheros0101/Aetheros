import 'package:flutter/material.dart';

/// AetherOS Foundation — Icon Tokens
///
/// Semantic icon map with a stable public API.
///
/// Architecture principle (locked):
/// ```
/// AetherIcons.agent   ← never changes at call sites
///        ↓
/// Icon Provider       ← swap backend here
///        ↓
/// Material / custom SVG / platform icons
/// ```
///
/// Today the backend is Material Icons. Future custom iconography
/// only requires changing the mapping below — not consumers.
abstract final class AetherIcons {
  // ─── Size scale ──────────────────────────────────────────────────────────

  static const double sizeXs = 12;
  static const double sizeSm = 14;
  static const double sizeMd = 16;
  static const double sizeLg = 20;
  static const double sizeXl = 24;
  static const double sizeXxl = 32;

  // ─── Chrome / navigation ─────────────────────────────────────────────────

  static const IconData home = Icons.home_rounded;
  static const IconData settings = Icons.settings_rounded;
  static const IconData back = Icons.arrow_back_rounded;
  static const IconData forward = Icons.arrow_forward_rounded;
  static const IconData close = Icons.close_rounded;
  static const IconData menu = Icons.menu_rounded;
  static const IconData more = Icons.more_vert_rounded;
  static const IconData moreHorizontal = Icons.more_horiz_rounded;
  static const IconData search = Icons.search_rounded;
  static const IconData replace = Icons.find_replace_rounded;
  static const IconData filter = Icons.filter_list_rounded;
  static const IconData refresh = Icons.refresh_rounded;
  static const IconData split = Icons.vertical_split_rounded;
  static const IconData pin = Icons.push_pin_rounded;
  static const IconData unpin = Icons.push_pin_outlined;
  static const IconData preview = Icons.preview_rounded;
  static const IconData maximize = Icons.open_in_full_rounded;
  static const IconData restore = Icons.close_fullscreen_rounded;
  static const IconData collapse = Icons.unfold_less_rounded;
  static const IconData expand = Icons.unfold_more_rounded;
  static const IconData chevronRight = Icons.chevron_right_rounded;
  static const IconData chevronDown = Icons.expand_more_rounded;
  static const IconData chevronLeft = Icons.chevron_left_rounded;
  static const IconData breadcrumb = Icons.chevron_right_rounded;

  // ─── Explorer / files ────────────────────────────────────────────────────

  static const IconData folder = Icons.folder_rounded;
  static const IconData folderOpen = Icons.folder_open_rounded;
  static const IconData file = Icons.insert_drive_file_rounded;
  static const IconData fileCode = Icons.code_rounded;
  static const IconData fileModified = Icons.edit_document;
  static const IconData fileAdded = Icons.note_add_rounded;
  static const IconData fileDeleted = Icons.delete_outline_rounded;
  static const IconData fileGit = Icons.account_tree_outlined;

  // ─── Git ─────────────────────────────────────────────────────────────────

  static const IconData branch = Icons.fork_right_rounded;
  static const IconData commit = Icons.commit_rounded;
  static const IconData diff = Icons.difference_rounded;
  static const IconData merge = Icons.merge_rounded;
  static const IconData stash = Icons.inventory_2_outlined;
  static const IconData remote = Icons.cloud_rounded;

  // ─── Domain: agents & tasks ──────────────────────────────────────────────

  static const IconData agent = Icons.smart_toy_rounded;
  static const IconData task = Icons.task_alt_rounded;
  static const IconData workflow = Icons.account_tree_rounded;
  static const IconData approval = Icons.verified_user_rounded;
  static const IconData audit = Icons.policy_rounded;
  static const IconData terminal = Icons.terminal_rounded;
  static const IconData script = Icons.code_rounded;
  static const IconData files = Icons.folder_rounded;
  static const IconData backup = Icons.backup_rounded;
  static const IconData wasm = Icons.extension_rounded;
  static const IconData chat = Icons.chat_rounded;
  static const IconData ai = Icons.auto_awesome_rounded;

  // ─── Actions ─────────────────────────────────────────────────────────────

  static const IconData add = Icons.add_rounded;
  static const IconData edit = Icons.edit_rounded;
  static const IconData delete = Icons.delete_outline_rounded;
  static const IconData save = Icons.save_rounded;
  static const IconData copy = Icons.copy_rounded;
  static const IconData share = Icons.share_rounded;
  static const IconData download = Icons.download_rounded;
  static const IconData upload = Icons.upload_rounded;
  static const IconData play = Icons.play_arrow_rounded;
  static const IconData pause = Icons.pause_rounded;
  static const IconData stop = Icons.stop_rounded;
  static const IconData submit = Icons.send_rounded;
  static const IconData undo = Icons.undo_rounded;
  static const IconData redo = Icons.redo_rounded;

  // ─── Status ──────────────────────────────────────────────────────────────

  static const IconData success = Icons.check_circle_rounded;
  static const IconData check = Icons.check_rounded;
  static const IconData warning = Icons.warning_amber_rounded;
  static const IconData error = Icons.error_rounded;
  static const IconData info = Icons.info_rounded;
  static const IconData pending = Icons.schedule_rounded;
  static const IconData running = Icons.sync_rounded;
  static const IconData cancelled = Icons.cancel_rounded;
  static const IconData blocked = Icons.block_rounded;

  // ─── Helper ──────────────────────────────────────────────────────────────

  static Icon of(
    IconData data, {
    double size = sizeMd,
    Color? color,
    String? semanticLabel,
  }) =>
      Icon(data, size: size, color: color, semanticLabel: semanticLabel);
}
