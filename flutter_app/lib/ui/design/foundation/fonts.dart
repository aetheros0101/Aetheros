/// AetherOS Foundation — Font Family Tokens
///
/// Font strategy is intentional and overridable.
/// Do NOT hardcode platform fonts that may be missing from the bundle.
///
/// Recommended shipping set (add under `fonts/` in pubspec):
/// ```
/// fonts/
/// ├── Inter/
/// │   ├── Inter-Regular.ttf
/// │   ├── Inter-Medium.ttf
/// │   └── Inter-SemiBold.ttf
/// └── JetBrainsMono/
///     ├── JetBrainsMono-Regular.ttf
///     └── JetBrainsMono-Medium.ttf
/// ```
///
/// Until custom fonts are bundled, [ui] / [code] / [terminal] resolve to
/// null (system default) / monospace fallback chain so the app never
/// crashes on a missing family.
abstract final class AetherFonts {
  /// UI chrome, panels, dialogs, agent prose.
  /// Prefer a humanist sans (Inter, SF Pro, Roboto).
  static const String? ui = null; // system default until bundled

  /// Code editor buffer.
  /// Prefer a coding ligature font (JetBrains Mono, Fira Code, Cascadia).
  static const String code = 'JetBrainsMono';

  /// Terminal / log streams (may match [code] or differ).
  static const String terminal = 'JetBrainsMono';

  /// Fallback chain when the preferred mono family is unavailable.
  static const List<String> monoFallback = [
    'Roboto Mono',
    'Cascadia Code',
    'Fira Code',
    'Courier New',
    'monospace',
  ];

  /// UI fallback chain.
  static const List<String> uiFallback = [
    'Inter',
    'SF Pro Text',
    'Roboto',
    'Segoe UI',
    'sans-serif',
  ];
}
