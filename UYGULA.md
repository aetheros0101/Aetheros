# Adım 1 — Workbench bağlama

Proje kökünde:

    unzip -o step1_wire.zip
    python3 apply_ws_dev.py
    python3 apply_wb_dart.py
    cargo fmt --all
    cargo test --workspace      # Cargo.lock güncellenir (notify)

Sonra commit + PR. CI codegen `bridge/api/workspace_dev.dart` üretecek;
flutter analyze/test adımları bilgilendirici (kırmızı yakmaz).
