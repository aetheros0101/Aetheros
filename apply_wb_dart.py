#!/usr/bin/env python3
"""Workbench Dart/CI düzeltmeleri (idempotent). Proje kökünde çalıştırın:

    python3 apply_wb_dart.py

1) aetheros.dart: eski `bridge/api.dart` yerine CI'ın ürettiği barrel.
2) FrbAgentService: Rust'ın tanımadığı yetki adlarını (file_read/file_write)
   katalogdaki geçerli adlarla değiştirir.
3) Eski el yazması bridge/api.dart taslağını siler (CI üretir).
4) build_apk.yml: codegen sonrası bilgilendirici flutter analyze/test.
"""
import pathlib
import sys

ROOT = pathlib.Path(".")
changed = []


def edit(rel, fn, required=True):
    p = ROOT / rel
    if not p.exists():
        if required:
            sys.exit(f"HATA: {rel} bulunamadı (proje kökünde misiniz?)")
        return
    old = p.read_text()
    new = fn(old)
    if new != old:
        p.write_text(new)
        changed.append(rel)


def need(t, anchor, rel):
    if anchor not in t:
        sys.exit(f"HATA: {rel} içinde çapa bulunamadı: {anchor!r}")


def aetheros_dart(t):
    rel = "flutter_app/lib/src/rust/api/aetheros.dart"
    if "import 'bridge_api.dart' as _bridge;" in t:
        return t
    old = "import '../bridge/api.dart' as _bridge;"
    need(t, old, rel)
    new = (
        "// FRB, Rust tarafındaki api/ alt modüllerini ayrı Dart dosyalarına böler;\n"
        "// bridge_api.dart bunları tek yüzeyde toplayan barrel'dır (CI üretir).\n"
        "import 'bridge_api.dart' as _bridge;"
    )
    return t.replace(old, new, 1)


DEFAULTS = """
/// Workbench agent paneli yeni bir agent başlatırken verilen varsayılan
/// yetkiler. Adlar Rust'taki `parse_capability` ile birebir aynı olmalı;
/// bilinmeyen ad `start_agent`'ı reddeder (test: agent_capabilities_test).
const List<String> kDefaultWorkbenchCapabilities = [
  'workspace_read',
  'workspace_write',
  'terminal_execution',
];
"""


def caps_dart(t):
    if "kDefaultWorkbenchCapabilities" in t:
        return t
    return t.rstrip("\n") + "\n" + DEFAULTS


def frb_agent(t):
    rel = "flutter_app/lib/application/services/frb_agent_service.dart"
    if "kDefaultWorkbenchCapabilities" in t:
        return t
    old = """    this.defaultCapabilities = const [
      'file_read',
      'file_write',
      'terminal_execution',
    ],"""
    need(t, old, rel)
    t = t.replace(old, "    this.defaultCapabilities = kDefaultWorkbenchCapabilities,", 1)
    imp = "import '../../state/agent_state.dart';"
    need(t, imp, rel)
    return t.replace(
        imp, "import '../../core/agent_capabilities.dart';\n" + imp, 1
    )


STEPS = """      - name: 🔍 flutter analyze (bilgilendirici)
        working-directory: flutter_app
        # Workbench kodu ilk kez burada analiz ediliyor; bulguları görmek için
        # şimdilik işi düşürmez. Temizlenince continue-on-error kaldırılır.
        continue-on-error: true
        run: flutter analyze --no-fatal-infos

      - name: 🧪 flutter test (bilgilendirici)
        working-directory: flutter_app
        continue-on-error: true
        run: flutter test

"""


def build_apk(t):
    rel = ".github/workflows/build_apk.yml"
    if "flutter analyze" in t:
        return t
    anchor = "      - name: 📤 Üretilen dosyaları artifact olarak yükle"
    need(t, anchor, rel)
    return t.replace(anchor, STEPS + anchor, 1)


edit("flutter_app/lib/src/rust/api/aetheros.dart", aetheros_dart)
edit("flutter_app/lib/core/agent_capabilities.dart", caps_dart)
edit("flutter_app/lib/application/services/frb_agent_service.dart", frb_agent)
edit(".github/workflows/build_apk.yml", build_apk)

stale = ROOT / "flutter_app/lib/src/rust/bridge/api.dart"
if stale.exists():
    stale.unlink()
    changed.append("flutter_app/lib/src/rust/bridge/api.dart (silindi)")

print("Değişenler:" if changed else "Zaten uygulanmış, değişiklik yok.")
for c in changed:
    print("  -", c)
