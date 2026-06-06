#!/usr/bin/env bash
# =============================================================
# scripts/build_android.sh
#
# AetherOS → Android APK tam build pipeline.
#
# KULLANIM:
#   chmod +x scripts/build_android.sh
#   ./scripts/build_android.sh
#
# ÖN KOŞULLAR:
#   1. Rust + cargo kurulu
#   2. Android NDK kurulu
#   3. cargo-ndk kurulu:  cargo install cargo-ndk
#   4. Android target'lar eklendi:
#        rustup target add aarch64-linux-android
#        rustup target add armv7-linux-androideabi
#   5. Flutter kurulu
#
# ÇIKTI:
#   flutter_app/android/app/src/main/jniLibs/
#     arm64-v8a/libaetheros.so       (~12-15 MB — wasmi backend)
#     armeabi-v7a/libaetheros.so
#   flutter_app/build/app/outputs/flutter-apk/app-release.apk
# =============================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
FLUTTER_APP="$PROJECT_ROOT/flutter_app"
JNI_LIBS="$FLUTTER_APP/android/app/src/main/jniLibs"

# ── Renkler ───────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; RESET='\033[0m'

log()  { echo -e "${BLUE}[AetherOS]${RESET} $*"; }
ok()   { echo -e "${GREEN}  ✓${RESET} $*"; }
warn() { echo -e "${YELLOW}  ⚠${RESET} $*"; }
fail() { echo -e "${RED}  ✗${RESET} $*"; exit 1; }

echo ""
echo -e "${CYAN}╔═══════════════════════════════════════╗${RESET}"
echo -e "${CYAN}║    AetherOS Android Build Pipeline    ║${RESET}"
echo -e "${CYAN}╚═══════════════════════════════════════╝${RESET}"
echo ""

# ── Ön kontroller ─────────────────────────────────────────
log "Ön kontroller yapılıyor..."

command -v cargo      >/dev/null || fail "cargo bulunamadı. rustup.rs"
command -v cargo-ndk  >/dev/null || fail "cargo-ndk bulunamadı: cargo install cargo-ndk"
command -v flutter    >/dev/null || fail "flutter bulunamadı"

# Android NDK
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -z "${NDK_HOME:-}" ]; then
    warn "ANDROID_NDK_HOME veya NDK_HOME tanımlı değil."
    warn "Örnek: export ANDROID_NDK_HOME=\$HOME/Android/Sdk/ndk/27.0.12077973"
fi

# Rust target kontrolü
TARGETS_OK=true
for target in "aarch64-linux-android" "armv7-linux-androideabi"; do
    if ! rustup target list --installed | grep -q "$target"; then
        warn "Eksik target: $target → rustup target add $target"
        TARGETS_OK=false
    fi
done
[ "$TARGETS_OK" = true ] && ok "Rust Android target'ları hazır"

# ── Adım 1: Rust → .so (wasmi backend) ────────────────────
log "Rust derleniyor (wasmi backend, Android)..."
echo ""

cd "$PROJECT_ROOT"

cargo ndk \
    --target aarch64-linux-android \
    --target armeabi-v7a \
    --output-dir "$JNI_LIBS" \
    build --release \
    --no-default-features \
    --features backend-wasmi \
    2>&1 | grep -E "Compiling|Finished|error|warning\[" || true

echo ""

# .so varlık kontrolü
SO_ARM64="$JNI_LIBS/arm64-v8a/libaetheros.so"
SO_ARM="$JNI_LIBS/armeabi-v7a/libaetheros.so"

if [ -f "$SO_ARM64" ]; then
    SIZE=$(du -sh "$SO_ARM64" | cut -f1)
    ok "arm64-v8a/libaetheros.so — $SIZE"
else
    fail "libaetheros.so üretilmedi (arm64). Rust log'larını kontrol et."
fi

if [ -f "$SO_ARM" ]; then
    SIZE=$(du -sh "$SO_ARM" | cut -f1)
    ok "armeabi-v7a/libaetheros.so — $SIZE"
fi

# ── Adım 2: FRB Dart kodu üretimi ─────────────────────────
log "FRB Dart kodu üretiliyor..."
cd "$FLUTTER_APP"

if dart run flutter_rust_bridge_codegen generate 2>/dev/null; then
    ok "frb_generated.dart oluşturuldu"
else
    warn "FRB codegen atlandı (bağımlılık kurulu olmayabilir)"
    warn "Manuel çalıştır: cd flutter_app && dart run flutter_rust_bridge_codegen generate"
fi

# ── Adım 3: Flutter bağımlılıkları ────────────────────────
log "Flutter bağımlılıkları yükleniyor..."
flutter pub get
ok "pub get tamamlandı"

# ── Adım 4: APK derle ─────────────────────────────────────
log "APK derleniyor (release)..."
echo ""

flutter build apk --release \
    --split-per-abi \
    2>&1 | grep -E "Built|Error|✓" || true

echo ""

APK_PATH="$FLUTTER_APP/build/app/outputs/flutter-apk"
if ls "$APK_PATH"/*.apk >/dev/null 2>&1; then
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════╗${RESET}"
    echo -e "${GREEN}║          BUILD BAŞARILI! 🚀            ║${RESET}"
    echo -e "${GREEN}╚═══════════════════════════════════════╝${RESET}"
    echo ""
    for apk in "$APK_PATH"/*.apk; do
        SIZE=$(du -sh "$apk" | cut -f1)
        ok "$(basename "$apk") — $SIZE"
    done
    echo ""
    log "APK konumu: $APK_PATH"
    echo ""
    echo -e "  ${CYAN}Cihaza yüklemek için:${RESET}"
    echo -e "  adb install $APK_PATH/app-arm64-v8a-release.apk"
    echo ""
else
    fail "APK üretilmedi. Flutter log'larını kontrol et."
fi
