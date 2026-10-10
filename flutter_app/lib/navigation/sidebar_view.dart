import 'package:flutter/widgets.dart';

/// Contract for a primary-sidebar view (Explorer, Search, SCM, …).
abstract class SidebarView {
  String get id;
  String get title;
  Widget build(BuildContext context);
}
