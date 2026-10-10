#!/usr/bin/env python3
"""Workbench kabuğu: Scaffold + SafeArea (telefon testi bulguları).
 - Sarı çift alt çizgi: Text'ler Material atası olmadan çiziliyordu.
 - Başlık çubuğu sistem durum çubuğunun altında kalıyordu (SafeArea yok).
Proje kökünde: python3 fix_workbench_ui.py   (idempotent)"""
import pathlib
import sys

p = pathlib.Path("flutter_app/lib/ui/shell/aether_workbench.dart")
if not p.exists():
    sys.exit("HATA: proje kökünde değilsiniz")
t = p.read_text()
if "body: SafeArea(child: Stack(" in t or "child: Scaffold(" in t:
    print("Zaten uygulanmış.")
    sys.exit(0)

old_start = """      child: Focus(
        autofocus: true,
        child: Stack(
          children: ["""
new_start = """      child: Focus(
        autofocus: true,
        child: Scaffold(
          body: SafeArea(child: Stack(
          children: ["""
old_end = """          ],
        ),
      ),
    );
  }

  Map<ShortcutActivator, VoidCallback> _shortcutBindings() {"""
new_end = """          ],
        )),
        ),
      ),
    );
  }

  Map<ShortcutActivator, VoidCallback> _shortcutBindings() {"""
for o in (old_start, old_end):
    if t.count(o) != 1:
        sys.exit(f"HATA: kalıp bulunamadı/benzersiz değil:\n{o[:80]}")
t = t.replace(old_start, new_start, 1).replace(old_end, new_end, 1)
p.write_text(t)
print("Değişti:", p)
