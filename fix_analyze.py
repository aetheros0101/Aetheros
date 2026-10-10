#!/usr/bin/env python3
"""flutter analyze temizliği (CI logu: 133 bulgu, 2 error). Idempotent.
  - analysis_options: tanınmayan kod silinir, üretilmiş Dart dışlanır
  - backup_screen.dart: 2 gerçek error (dynamic -> Iterable) giderilir
  - application.dart: ölü `??` kaldırılır
  - build_apk.yml: analyze (yalnız error'da düşer) ve test artık ZORUNLU
Proje kökünde: python3 fix_analyze.py"""
import pathlib, sys

changed = []

def edit(rel, fn):
    p = pathlib.Path(rel)
    if not p.exists():
        sys.exit(f"HATA: {rel} yok (proje kökünde misiniz?)")
    o = p.read_text(); n = fn(o)
    if n != o:
        p.write_text(n); changed.append(rel)

def rep(t, old, new, rel, cnt=1):
    if new in t and old not in t:
        return t
    if old not in t:
        sys.exit(f"HATA: {rel}: kalıp yok: {old[:70]!r}")
    return t.replace(old, new)

def opts(t):
    t = t.replace("    invalid_return_type: error\n", "")
    if "exclude:" in t:
        return t
    return t.replace(
        "analyzer:\n",
        "analyzer:\n  exclude:\n"
        "    - lib/src/rust/frb_generated*.dart\n"
        "    - lib/src/rust/bridge/**\n"
        "    - lib/src/rust/api/rest/**\n"
        "    - lib/src/rust/metrics/**\n"
        "    - lib/src/rust/registry/**\n"
        "    - lib/src/rust/types/**\n"
        "    - '**/*.freezed.dart'\n"
        "    - '**/*.g.dart'\n", 1)

def backup(t):
    t = rep(t, "List<int>.from(content))) as Map<String, dynamic>;",
            "List<int>.from(content as Iterable<dynamic>))) as Map<String, dynamic>;",
            "backup_screen.dart")
    t = rep(t, "final wasm = content is List<int> ? content : List<int>.from(content);",
            "final wasm = content is List<int>\n                ? content\n                : List<int>.from(content as Iterable<dynamic>);",
            "backup_screen.dart")
    return t

def app(t):
    return rep(t, "saved.navigation.activeId ?? s.navigation.activeId",
               "saved.navigation.activeId", "application.dart")

def wf(t):
    t = t.replace("        continue-on-error: true\n", "")
    if "--no-fatal-warnings" not in t:
        t = t.replace("flutter analyze --no-fatal-infos",
                      "flutter analyze --no-fatal-infos --no-fatal-warnings")
    t = t.replace("flutter analyze (bilgilendirici)", "flutter analyze (yalnız error)")
    t = t.replace("flutter test (bilgilendirici)", "flutter test")
    t = t.replace("        # Workbench kodu ilk kez burada analiz ediliyor; bulguları görmek için\n"
                  "        # şimdilik işi düşürmez. Temizlenince continue-on-error kaldırılır.\n", "")
    return t

edit("flutter_app/analysis_options.yaml", opts)
edit("flutter_app/lib/screens/backup_screen.dart", backup)
edit("flutter_app/lib/application/application.dart", app)
edit(".github/workflows/build_apk.yml", wf)
print("Değişenler:" if changed else "Zaten uygulanmış.")
for c in changed: print("  -", c)
