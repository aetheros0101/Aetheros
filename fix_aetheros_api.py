#!/usr/bin/env python3
"""aetheros_api.dart: AetherApi sınıfı getMetrics'ten sonra erken kapanıyor,
altındaki workspace/git metotları sınıfın dışında kalıyordu ('Can't have
modifier static here' -> build_runner/freezed çöküyordu). Fazladan `}` silinir.
Idempotent. Proje kökünde: python3 fix_aetheros_api.py"""
import pathlib, sys

p = pathlib.Path("flutter_app/lib/api/aetheros_api.dart")
if not p.exists():
    sys.exit("HATA: proje kökünde değilsiniz")
t = p.read_text()
bad = "    return rust.getMetrics();\n  }\n}\n\n  // ── Workspace search"
good = "    return rust.getMetrics();\n  }\n\n  // ── Workspace search"
if bad in t:
    p.write_text(t.replace(bad, good, 1))
    print("Düzeltildi:", p)
elif good in t:
    print("Zaten düzeltilmiş.")
else:
    sys.exit("HATA: beklenen kalıp bulunamadı, dosyayı bana gösterin")
