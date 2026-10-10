import 'package:flutter/material.dart';

import '../../../state/app_state.dart';
import '../../../state/workbench_state.dart';
import '../../design/aether_theme.dart';
import '../../../screens/ai_settings_screen.dart';

/// Workbench Settings — real chrome toggles + planned sections.
class SettingsView extends StatelessWidget {
  const SettingsView({
    super.key,
    required this.state,
    required this.onStateUpdate,
    this.onOpenLegacy,
  });

  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final VoidCallback? onOpenLegacy;

  WorkbenchLayoutState get _wb => state.workbench;

  void _patch(WorkbenchLayoutState Function(WorkbenchLayoutState) fn) {
    onStateUpdate((s) => s.copyWith(workbench: fn(s.workbench)));
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(AetherSpacing.md),
      children: [
        Text(
          'Settings',
          style: AetherTypography.uiBody.copyWith(
            color: AetherTextColors.primary,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: AetherSpacing.sm),
        Text(
          'Chrome layout options apply immediately and are saved with the '
          'workbench layout. Other sections are planned.',
          style: AetherTypography.uiCaption.copyWith(
            color: AetherTextColors.tertiary,
          ),
        ),
        const SizedBox(height: AetherSpacing.lg),
        Text(
          'Workbench chrome',
          style: AetherTypography.uiCaption.copyWith(
            color: AetherTextColors.secondary,
            letterSpacing: 0.6,
          ),
        ),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: Text(
            'Status bar',
            style: AetherTypography.uiBody.copyWith(color: AetherTextColors.primary),
          ),
          value: _wb.statusBarVisible,
          activeColor: AetherAccent.primary,
          onChanged: (v) => _patch((w) => w.copyWith(statusBarVisible: v)),
        ),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: Text(
            'Bottom panel',
            style: AetherTypography.uiBody.copyWith(color: AetherTextColors.primary),
          ),
          value: _wb.bottomPanelVisible,
          activeColor: AetherAccent.primary,
          onChanged: (v) => _patch((w) => w.copyWith(bottomPanelVisible: v)),
        ),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: Text(
            'Activity bar',
            style: AetherTypography.uiBody.copyWith(color: AetherTextColors.primary),
          ),
          value: _wb.activityBarVisible,
          activeColor: AetherAccent.primary,
          onChanged: (v) => _patch((w) => w.copyWith(activityBarVisible: v)),
        ),
        ListTile(
          contentPadding: EdgeInsets.zero,
          title: Text(
            'Density',
            style: AetherTypography.uiBody.copyWith(color: AetherTextColors.primary),
          ),
          subtitle: Text(
            _wb.density,
            style: AetherTypography.uiCaption.copyWith(color: AetherTextColors.tertiary),
          ),
          trailing: DropdownButton<String>(
            value: _wb.density,
            dropdownColor: AetherSurfaces.popup,
            underline: const SizedBox.shrink(),
            items: const [
              DropdownMenuItem(value: 'compact', child: Text('compact')),
              DropdownMenuItem(value: 'standard', child: Text('standard')),
              DropdownMenuItem(value: 'comfortable', child: Text('comfortable')),
            ],
            onChanged: (v) {
              if (v != null) _patch((w) => w.copyWith(density: v));
            },
          ),
        ),
        const SizedBox(height: AetherSpacing.lg),
        _PlannedSection(title: 'Appearance', items: const ['Theme (light / dark)', 'Font size']),
        _PlannedSection(title: 'Editor', items: const ['Tab size', 'Word wrap', 'Minimap']),
        _PlannedSection(title: 'Workspace', items: const ['Exclude patterns', 'File watcher']),
        Text(
          'AI / Agent',
          style: AetherTypography.uiCaption.copyWith(
            color: AetherTextColors.secondary,
            letterSpacing: 0.6,
          ),
        ),
        ListTile(
          contentPadding: EdgeInsets.zero,
          leading: Icon(Icons.smart_toy_outlined,
              color: AetherTextColors.secondary),
          title: Text(
            'AI sağlayıcıları',
            style: AetherTypography.uiBody
                .copyWith(color: AetherTextColors.primary),
          ),
          subtitle: Text(
            'Anthropic / OpenAI / Gemini / Ollama ekle ve aktif et '
            '(Agent bunsuz plan üretemez)',
            style: AetherTypography.uiCaption
                .copyWith(color: AetherTextColors.tertiary),
          ),
          trailing: Icon(Icons.chevron_right,
              color: AetherTextColors.tertiary),
          onTap: () => Navigator.of(context).push<void>(
            MaterialPageRoute<void>(
              builder: (_) => const AiSettingsScreen(),
            ),
          ),
        ),
        const SizedBox(height: AetherSpacing.md),
        if (onOpenLegacy != null) ...[
          const SizedBox(height: AetherSpacing.lg),
          OutlinedButton(
            onPressed: onOpenLegacy,
            child: const Text('Open legacy AI Settings'),
          ),
        ],
      ],
    );
  }
}

class _PlannedSection extends StatelessWidget {
  const _PlannedSection({required this.title, required this.items});
  final String title;
  final List<String> items;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: AetherSpacing.md),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: AetherTypography.uiCaption.copyWith(
              color: AetherTextColors.secondary,
              letterSpacing: 0.6,
            ),
          ),
          for (final item in items)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Row(
                children: [
                  Icon(AetherIcons.chevronRight, size: 14, color: AetherTextColors.tertiary),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      item,
                      style: AetherTypography.uiBody.copyWith(color: AetherTextColors.tertiary),
                    ),
                  ),
                  Text(
                    'Soon',
                    style: AetherTypography.uiMicro.copyWith(color: AetherTextColors.tertiary),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
