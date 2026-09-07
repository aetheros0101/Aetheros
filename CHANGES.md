# Değişiklik Özeti — Güvenlik Sertleştirmesi ve Kalite İyileştirmeleri

Bu doküman, ilk kod incelemesinde tespit edilen eksikler ile bunlara
karşılık yapılan değişiklikleri önceliklendirilmiş sırayla listeler.

**ÖNEMLİ — DERLENMEDİ:** Bu değişiklikler, ağ erişimi ve Rust
derleyicisi olmayan bir ortamda, statik kod incelemesiyle yazıldı.
`cargo build` / `cargo test` ile doğrulanmadı. Merge etmeden önce
mutlaka yerel ortamda derleyip test suite'ini çalıştırın.

## 1. API kimlik doğrulaması (Kritik → Çözüldü)

**Önce:** `auth_middleware` ve `api_key_middleware` yazılmıştı ama
`router.rs`'te hiçbir route'a bağlanmamıştı — tüm API (`0.0.0.0:8080`)
kimliksiz olarak açıktı. `api_key_middleware` ayrıca boş olmayan
herhangi bir `X-Api-Key` değerini geçerli sayıyordu.

**Sonra** (`src/api/middleware.rs`, `src/api/rest/router.rs`):
- `authenticate()`: `X-Cluster-Token` → `X-Api-Key` → `Authorization: Bearer`
  sırasıyla gerçek doğrulama yapar, `AuthContext { subject, role, method }`
  üretir.
- `require_auth` / `require_admin` middleware'leri artık router'da
  gerçekten `route_layer` olarak uygulanıyor. Route'lar üç gruba
  ayrıldı: `public_routes` (health, `/auth/token`), `protected_routes`
  (kimlik ister), `admin_routes` (Role::Admin ister: cluster
  yönetimi, uzak komut, modül yükleme).
- `AuthContext` request extensions'a enjekte ediliyor — handler'lar
  `Extension<AuthContext>` ile role bilgisine erişebilir (önceki koddaki
  "Claims extension'a enjekte edilebilir" yorumu artık gerçek).
- `/ws`: tarayıcı WebSocket API'si custom header gönderemediği için
  `?token=`/`?api_key=` query param ile doğrulanıyor.
- `/auth/token`: `AETHEROS_ADMIN_KEY` ile korunan bootstrap endpoint'i
  eklendi (önceden token üretmenin hiçbir yolu yoktu).

## 2. Gerçek API key deposu (Kritik → Çözüldü)

`src/security/auth/token.rs`'e `ApiKeyStore` eklendi: `AETHEROS_API_KEYS`
(virgülle ayrılmış) ortam değişkeninden yüklenir, SHA-256 + constant-time
karşılaştırma yapar (bu primitifler zaten `ApiKey` içinde vardı, sadece
hiç kullanılmıyordu).

## 3. RBAC / yetkilendirme tutarlılığı (Yüksek → Kısmen çözüldü)

- Duplicate `SecurityPolicy` tanımı kaldırıldı (`security/policies.rs`
  silindi, `security/policy.rs`'teki — `audit_required` alanına sahip
  — üstün versiyon tutuldu; hiçbir yerde referans edilmediği
  doğrulandı).
- `security/authorization.rs`'teki `AuthorizationPolicy`, gerçek RBAC
  sistemi olan `security::rbac::RbacGuard`'a işaret eden bir
  deprecation notuyla işaretlendi (kaldırılmadı, geriye dönük uyumluluk
  için).
- Not: `security::rbac` zaten tam ve iyi tasarlanmış bir `Role`/`Action`/
  `RbacGuard` sistemine sahipti — sorun eksik kod değil, hiç
  çağrılmamasıydı. Artık `api_key_role()` ve `/auth/token` üzerinden
  gerçek isteklere bağlanıyor.
- Basit audit logging eklendi: `require_auth`/`require_admin` her
  izin/red kararını `tracing::info!/warn!(target: "audit", ...)` ile
  loglar (subject, role, method, path, outcome).

**Yapılmadı:** Her REST handler'ının kendi `Action`'ını `RbacGuard`
üzerinden tek tek kontrol etmesi (ör. `TaskCancel` sadece Operator+).
Şu an sadece route-grubu seviyesinde (auth var/yok, admin var/yok)
ayrım var. İnce taneli (per-action) kontrol bir sonraki adım.

## 4. At-rest şifreleme (AES-256-GCM)

`src/persistence/encryption.rs` artık `ring` üzerinden AES-256-GCM kullanıyor.
`AETHEROS_ENCRYPTION_KEY` 64 hex karakter (32 byte) olmalıdır. Her kayıt için
rastgele 96-bit nonce üretilir ve nonce ciphertext'in başına yazılır.

Anahtar yoksa yerel/geliştirme uyumluluğu için plaintext modu korunur ve açık
uyarı loglanır. Production ortamında anahtar zorunlu kabul edilmelidir.
Mevcut plaintext sled veritabanlarının şifreli moda geçişi için ayrıca migration
çalıştırılmalıdır.

## 5. Cluster/remote transport güvenliği (Orta → Kısmen çözüldü)

**Önce:** `TransportSecurity` (tls_enabled/mutual_tls/token_auth) sadece
bir config struct'tı, hiçbir yerde kullanılmıyordu; `HttpTransport`
node'lara düz `http://` ile, kimliksiz istek atıyordu.

**Sonra:** `AETHEROS_CLUSTER_TOKEN` paylaşımlı sırrı eklendi —
`HttpTransport::send_to`/`broadcast` bunu `X-Cluster-Token` header'ı
olarak ekliyor, `authenticate()` bunu doğrulayıp `Role::Admin` veriyor.
Bu, node'ları birbirine karşı doğrular.

**Yapılmadı:** Gerçek TLS/mTLS (taşımanın şifrelenmesi). Bunun için
nodlar arasına bir ters proxy (nginx/envoy TLS termination) veya
`rustls` tabanlı bir istemci/sunucu kurulumu eklenmesi gerekiyor —
kapsam ve doğrulanamama riski nedeniyle bu oturumda yapılmadı.

## 6. CI/CD (Orta → Çözüldü)

`.github/workflows/ci.yml` eklendi: `cargo fmt --check`, `cargo clippy
-- -D warnings`, `cargo test --all-targets`, `cargo audit` (bilgi
amaçlı, `continue-on-error`). Önceki tek workflow (`build_apk.yml`)
sadece Android APK build ediyordu, dokunulmadı.

## 7. `unwrap()`/`panic!` riskleri (Düşük-Orta → İncelendi)

190 `.unwrap()` çağrısının büyük kısmı `src/tests/` altında (test
kodu, risk yok). Üretim kodunda kalanlar tek tek incelendi:

- `src/api/rest/router.rs` (2 adet, `get_agent_handler`/
  `get_workflow_handler`): `serde_json::to_value(...).unwrap()` →
  `match` ile `500` dönecek şekilde düzeltildi. Bunlar kullanıcı
  isteğiyle tetiklenebilen tek üretim unwrap'leriydi.
- `src/logging/buffer.rs` (5 adet): `Mutex::lock().unwrap()` —
  standart Rust idiomu, sadece poisoned mutex'te panikler (zaten
  başka bir thread çökmüş demektir). Değiştirilmedi.
- `src/main.rs` (2 adet): sinyal handler kurulumu, sadece
  başlangıçta çalışır, başarısızlık zaten fatal olmalı. Değiştirilmedi.
- `src/workflows/compiler.rs` (1 adet): `id_map.get(&step.id).unwrap()`
  — incelendi, `id_map` aynı `dsl.steps` listesinden hemen önce
  dolduruluyor, yani bu değer her zaman mevcut (invariant ile
  korunuyor). Güvenli, değiştirilmedi.
- `src/frb_generated.rs`: flutter_rust_bridge tarafından otomatik
  üretiliyor, elle düzenlenmez.

## 8. Flutter testleri (Düşük → Başlangıç eklendi)

`flutter_app/test/ai_provider_service_test.dart`: önceden flutter_app'te
hiç test yoktu. Rust bridge'e veya widget ağacına ihtiyaç duymayan saf
Dart mantığı (`aiProviders` sabit listesi, `aiProviderById` lookup)
için ilk unit test seti eklendi.

**Yapılmadı:** `AiProviderService`'in storage/bridge'e bağımlı
metodları (save/load/rehydrateFromStorage) ve widget testleri — bunlar
için sahte (fake) `FlutterSecureStorage`/Rust bridge implementasyonları
gerekir, kapsam dışı bırakıldı.

## 9. README (Düşük → Çözüldü)

Tek satırlık placeholder, mimari diyagramı, kurulum adımları, ortam
değişkeni tablosu ve bilinen sınırlara referans içeren gerçek bir
README ile değiştirildi.

---

## Özet tablo

| # | Konu | Durum |
|---|---|---|
| 1 | API auth wiring | ✅ Çözüldü |
| 2 | Gerçek API key store | ✅ Çözüldü |
| 3 | RBAC/authz tutarlılığı | 🟡 Kısmen (route-grubu seviyesinde) |
| 4 | At-rest şifreleme | 🟢 AES-256-GCM etkin (key verildiğinde) |
| 5 | Cluster transport güvenliği | 🟡 Node auth var, TLS/mTLS yok |
| 6 | CI (test/clippy/audit) | ✅ Çözüldü |
| 7 | unwrap() riskleri | ✅ İncelendi, gerçek riskler düzeltildi |
| 8 | Flutter testleri | 🟡 Başlangıç seti eklendi |
| 9 | README | ✅ Çözüldü |


## Security hardening pass

- JWT signing changed to RFC 2104 HMAC-SHA256 with RFC 4648 base64url encoding.
- Removed the predictable JWT development secret.
- API keys are no longer retained as raw strings.
- WASM module size/count/total-store limits are enforced.
- Wasmtime store enforces linear-memory/resource limits.
- Wasmtime store memory/resource limits are now enforced; the Wasmi backend remains on its existing timeout-based compatibility path until its dependency is upgraded.
- REST task/agent/workflow/script inputs have explicit resource limits.
- `cargo audit` is a blocking CI gate.
- Android Rust/Flutter build failures are no longer hidden by `grep ... || true`.
- AES-256-GCM at-rest encryption is available when `AETHEROS_ENCRYPTION_KEY` is configured.
