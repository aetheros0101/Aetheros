#!/usr/bin/env bash
# ============================================================
# generate_bridge.sh
#
# FRB v2.12.0 Dart köprüsünü üretir.
# Bunu projenin ROOT dizininde çalıştır:
#   chmod +x scripts/generate_bridge.sh
#   ./scripts/generate_bridge.sh
#
# Üretilen dosyalar git'e commit edilmeli:
#   git add flutter_app/lib/src/rust/
#   git commit -m "bridge: FRB codegen güncelle"
# ============================================================

set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "🦀 AetherOS FRB Bridge Codegen"
echo "   Root: $ROOT"
echo ""

# ── 1. cargo-expand (FRB macro expansion için) ────────────
if ! command -v cargo-expand &>/dev/null; then
  echo "📦 cargo-expand kuruluyor..."
  cargo install cargo-expand --locked 2>/dev/null || cargo install cargo-expand
fi

# ── 2. flutter_rust_bridge_codegen ───────────────────────
if ! command -v flutter_rust_bridge_codegen &>/dev/null; then
  echo "📦 flutter_rust_bridge_codegen kuruluyor..."
  cargo install flutter_rust_bridge_codegen --version 2.12.0
fi

echo "✅ Araçlar hazır"
echo ""

# ── 3. Dart bağımlılıkları ────────────────────────────────
echo "🐦 Flutter pub get..."
cd "$ROOT/flutter_app"
flutter pub get

# ── 4. Codegen ────────────────────────────────────────────
echo ""
echo "⚙️  Codegen çalışıyor..."
flutter_rust_bridge_codegen generate

echo ""
echo "✅ Codegen tamamlandı!"
echo ""
echo "Üretilen dosyalar:"
find lib/src/rust -name "*.dart" | sort | while read f; do
  echo "  📄 $f"
done

echo ""
echo "Sonraki adım:"
echo "  git add flutter_app/lib/src/rust/"
echo "  git commit -m 'bridge: FRB codegen $(date +%Y-%m-%d)'"
echo "  git push"
