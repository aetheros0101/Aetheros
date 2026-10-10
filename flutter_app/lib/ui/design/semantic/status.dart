import 'package:flutter/material.dart';

import '../foundation/colors.dart';

/// AetherOS Semantic — Generic Status Colors
///
/// Shared success / warning / danger / info used by banners, badges,
/// validation, and non-agent status indicators.
abstract final class AetherStatus {
  static const Color success = AetherColorPrimitives.green400;
  static const Color successContainer = AetherColorPrimitives.green100;
  static const Color onSuccess = AetherColorPrimitives.neutral1000;

  static const Color warning = AetherColorPrimitives.amber400;
  static const Color warningContainer = AetherColorPrimitives.amber100;
  static const Color onWarning = AetherColorPrimitives.neutral0;

  static const Color danger = AetherColorPrimitives.red400;
  static const Color dangerContainer = AetherColorPrimitives.red100;
  static const Color onDanger = AetherColorPrimitives.neutral1000;

  static const Color info = AetherColorPrimitives.blue400;
  static const Color infoContainer = AetherColorPrimitives.blue100;
  static const Color onInfo = AetherColorPrimitives.neutral1000;

  static const Color neutral = AetherColorPrimitives.neutral700;
  static const Color neutralContainer = AetherColorPrimitives.neutral250;
  static const Color onNeutral = AetherColorPrimitives.neutral1000;

  static const Color disabled = AetherColorPrimitives.neutral600;
}
