import 'package:flutter/material.dart';

import '../design/aether_theme.dart';

/// Which workbench region is being resized.
enum ResizeRegion {
  primarySidebar,
  secondarySidebar,
  bottomPanel,
}

/// Abstract resize controller — shell applies deltas to [WorkbenchLayoutState].
typedef ResizeHandler = void Function(ResizeRegion region, double delta);

/// Draggable 1px (hit-target 4px) split handle.
class ResizeHandle extends StatefulWidget {
  const ResizeHandle({
    super.key,
    required this.axis,
    required this.region,
    required this.onResize,
  });

  final Axis axis;
  final ResizeRegion region;
  final ResizeHandler onResize;

  @override
  State<ResizeHandle> createState() => _ResizeHandleState();
}

class _ResizeHandleState extends State<ResizeHandle> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final isHorizontal = widget.axis == Axis.horizontal;
    return MouseRegion(
      cursor: isHorizontal
          ? SystemMouseCursors.resizeColumn
          : SystemMouseCursors.resizeRow,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onHorizontalDragUpdate: isHorizontal
            ? (d) => widget.onResize(widget.region, d.delta.dx)
            : null,
        onVerticalDragUpdate: !isHorizontal
            ? (d) => widget.onResize(widget.region, d.delta.dy)
            : null,
        child: Container(
          width: isHorizontal ? 4 : null,
          height: isHorizontal ? null : 4,
          color: _hover
              ? AetherAccent.primary.withOpacity(0.35)
              : AetherSurfaces.chrome,
          alignment: Alignment.center,
          child: Container(
            width: isHorizontal ? 1 : double.infinity,
            height: isHorizontal ? double.infinity : 1,
            color: _hover ? AetherAccent.primary : Colors.transparent,
          ),
        ),
      ),
    );
  }
}
