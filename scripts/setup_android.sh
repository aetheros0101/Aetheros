#!/usr/bin/env bash
# =============================================================
# scripts/setup_android.sh
#
# Tek seferlik Android geliştirme ortamı kurulumu.
# Yeni bir makinede veya CI'da bir kez çalıştırılır.
#
# KULLANIM:
#   chmod +x scripts/setup_android.sh
#   ./scripts/setup_android.sh
# =============================================================

set -euo pipefail

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; RESET='\033[0m'
log()  { echo -e "${BLUE}[setup]${RESET} $*"; }
ok()   { echo -e "${GREEN}  ✓${RESET} $*"; }
warn() { echo -e "${YELLOW}  ⚠${RESET} $*"; }

echo ""
log "AetherOS Android geliştirme ortamı kuruluyor..."
echo ""

# ── 1. Rust Android target'ları ───────────────────────────
log "Rust Android target'ları ekleniyor..."
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android   # Emülatör için
ok "Rust target'lar hazır"

# ── 2. cargo-ndk ──────────────────────────────────────────
log "cargo-ndk kuruluyor..."
if command -v cargo-ndk >/dev/null 2>&1; then
    ok "cargo-ndk zaten kurulu: $(cargo ndk --version)"
else
    cargo install cargo-ndk
    ok "cargo-ndk kuruldu"
fi

# ── 3. flutter_rust_bridge_codegen ────────────────────────
log "flutter_rust_bridge_codegen kuruluyor..."
if command -v flutter_rust_bridge_codegen >/dev/null 2>&1; then
    ok "flutter_rust_bridge_codegen zaten kurulu"
else
    cargo install flutter_rust_bridge_codegen
    ok "flutter_rust_bridge_codegen kuruldu"
fi

# ── 4. Flutter bağımlılıkları ─────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FLUTTER_APP="$(dirname "$SCRIPT_DIR")/flutter_app"

if [ -d "$FLUTTER_APP" ]; then
    log "Flutter bağımlılıkları yükleniyor..."
    cd "$FLUTTER_APP"
    flutter pub get
    ok "Flutter pub get tamamlandı"
fi

# ── 5. Özet ───────────────────────────────────────────────
echo ""
log "Ortam hazır. Build için:"
echo ""
echo "  ./scripts/build_android.sh"
echo ""

warn "ANDROID_NDK_HOME tanımlı değilse build.gradle'ı kontrol et."
warn "Örnek: export ANDROID_NDK_HOME=\$HOME/Android/Sdk/ndk/27.0.12077973"
echo ""
