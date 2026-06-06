// ============================================================
// src/bridge/mod.rs
//
// Flutter ↔ Rust köprüsü modül tanımları.
//
// FRB v2 kod üretici bu modülü tarar:
//   flutter_rust_bridge_codegen generate
//
// Çıktı: flutter_app/lib/src/rust/
//   ├── frb_generated.dart        (FRB altyapısı)
//   └── api/
//       └── aetheros.dart         (bu modülden üretilen Dart API)
// ============================================================

pub mod api;
pub mod state;
pub mod types;
