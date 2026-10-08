# AetherOS

AetherOS, bir **Agent Operating System**: Rust'ta yazılmış bir çalışma
zamanı (runtime), WASM tabanlı bir agent/görev yürütme motoru, bir
workflow motoru ve bunları mobilden yöneten bir Flutter istemcisinden
oluşur. Rust tarafı hem bağımsız bir sunucu binary'si hem de
flutter_rust_bridge (FRB) üzerinden Flutter'a gömülü bir kütüphane
olarak derlenir.

> Bu README, projeyi ilk kez klonlayan birinin ne olduğunu, nasıl
> ayağa kaldıracağını ve mevcut sınırların ne olduğunu anlaması için
> yazıldı. Güncel bilinen eksiklerin/planların tam listesi için
> `CHANGES.md`'ye bakın.

## Mimari

```
┌─────────────────────────┐        FRB (flutter_rust_bridge)
│   flutter_app/ (Dart)   │◄──────────────────────────────────┐
│  - Ekranlar, ayarlar    │                                    │
│  - flutter_secure_storage                                    │
└─────────────────────────┘                                    │
                                                                 ▼
┌───────────────────────────────────────────────────────────────────┐
│                        src/ (Rust çekirdeği)                      │
│                                                                     │
│  runtime/    → agent & workflow yürütme çekirdeği                 │
│  wasm/       → WASM sandbox (wasmtime veya wasmi backend)          │
│  agents/     → agent tanımları ve yürütme mantığı                  │
│  workflows/  → çok adımlı workflow motoru                          │
│  orchestration/ → workflow grafı, koordinasyon (küme iskeleti deneysel)│
│  api/        → axum tabanlı REST + WebSocket API (sunucu modunda)  │
│  security/   → RBAC (rbac.rs), JWT-lite + API key (auth/token.rs)  │
│  persistence/→ sled tabanlı kalıcılık (MessagePack serileştirme)   │
│  remote/     → cluster/node-to-node iletişimi                      │
│  ai/         → Anthropic/OpenAI/Gemini/Ollama provider entegrasyonu│
│  scripting/  → script motoru                                       │
│  bridge/     → FRB için Rust↔Dart köprü katmanı                    │
└───────────────────────────────────────────────────────────────────┘
```

İki farklı çalışma modu vardır:

1. **Sunucu modu** (`src/main.rs`) — bağımsız bir binary, REST/WebSocket
   API'sini axum ile `0.0.0.0:8080` üzerinde açar. AI provider'lar ortam
   değişkenlerinden okunur.
2. **Mobil modu** (`flutter_app/`) — Rust, FRB aracılığıyla bir kütüphane
   (`cdylib`/`staticlib`) olarak Flutter uygulamasına gömülür; runtime
   cihaz üzerinde, ağ olmadan çalışır.

## Gereksinimler

- Rust (stable) — `rustup` ile kurulum önerilir
- Flutter 3.24.x (mobil istemci için)
- Android NDK 27+ (Android build için — bkz. `scripts/setup_android.sh`)
- `flutter_rust_bridge_codegen` (Rust↔Dart köprüsünü yeniden üretmek için)

## Kurulum ve çalıştırma

### Rust çekirdeği (sunucu modu)

```bash
# Test
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Sunucuyu çalıştır (varsayılan backend: wasmtime)
cargo run
```

Önemli ortam değişkenleri (hepsi opsiyonel, ayarlanmazsa güvenli/kısıtlı
bir varsayılana düşer — bkz. `SECURITY_HARDENING.md`):

| Değişken | Amaç |
|---|---|
| `AETHEROS_JWT_SECRET` | JWT-lite imzalama anahtarı. **Üretimde zorunlu.** |
| `AETHEROS_API_KEYS` | Virgülle ayrılmış geçerli API key listesi (X-Api-Key header) |
| `AETHEROS_API_KEY_ROLE` | X-Api-Key ile gelen isteklere atanacak rol (`admin`/`operator`/`viewer`/`agent`) |
| `AETHEROS_ADMIN_KEY` | `/auth/token` uç noktasını açan bootstrap sırrı |
| `AETHEROS_CLUSTER_TOKEN` | Node-to-node (`/remote/command`) çağrılarını doğrulayan paylaşımlı sır |
| `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` / `GEMINI_API_KEY` / `OLLAMA_HOST` | AI provider yapılandırması |

### Flutter mobil istemci

```bash
cd flutter_app
flutter pub get

# FRB köprüsünü Rust kaynağından yeniden üret (Rust API değiştiyse)
bash ../scripts/generate_bridge.sh

# Testler
flutter test

# Android APK
bash ../scripts/build_android.sh
```

CI, her push/PR'da hem Rust tarafını (`cargo fmt`, `clippy`, `test`,
`audit` — bkz. `.github/workflows/ci.yml`) hem de Android APK build'ini
(`.github/workflows/build_apk.yml`) otomatik çalıştırır.

## Cargo feature'ları

- `backend-wasmtime` (varsayılan) — WASM sandbox için wasmtime kullanır.
- `backend-wasmi` — Android/mobil derlemelerde tercih edilen, saf Rust
  yorumlayıcı tabanlı alternatif backend.

## Bilinen sınırlar

Bu proje aktif geliştirme altında. Şu an bilinçli olarak eksik bırakılan
veya sadece entegrasyon noktası olarak hazırlanmış (henüz tam
implemente edilmemiş) alanlar için `CHANGES.md`'ye bakın — özellikle
at-rest şifreleme ve node'lar arası TLS/mTLS henüz tamamlanmadı.

## Katkı

Bu, tek bir geliştirici tarafından yürütülen ticari bir üründür; dış
katkı süreci şu an tanımlı değildir.
