# AetherOS Flutter — Professionalization Pass

Tarih: 2026-09-13

Bu sürüm Termux üzerinde geliştirilen ve GitHub Actions üzerinden build edilip fiziksel Android cihazda manuel smoke test edilen AetherOS akışına göre hazırlanmıştır. Termux kullanımı burada bir kalite eksiği olarak değerlendirilmemiştir; kalite kapısı CI + statik analiz + otomatik test + cihaz smoke testidir.

## Uygulanan P1

### 1. Rust bridge sınırı
`AetherApi` uygulamanın Rust runtime'a tek uygulama-facing facade'ıdır. Ekranlarda generated FRB method çağrısı yerine facade kullanımı korunmuştur. Generated Rust tipleri yalnızca veri sözleşmesi olarak import edilir.

### 2. Lifecycle-aware polling
Task, Agent, Workflow, Log ve Remote ekranlarındaki periyodik sorgular `LifecyclePoller` ile merkezi hale getirildi.
- Uygulama background olduğunda timer durur.
- Foreground'a dönünce tekrar sorgular.
- Aynı anda iki polling isteği uçuşta tutulmaz.
- `dispose()` observer/timer temizliğini garanti eder.

### 3. Workflow retry persistence
Workflow adımları yalnızca RAM'de tutulmuyor; `SharedPreferences` içine workflow ID ile persist ediliyor. Uygulama yeniden açıldıktan sonra retry için gerekli step tanımı geri yüklenebiliyor.

### 4. Workflow JSON validation
Script Editor'daki Workflow JSON artık yalnızca `jsonDecode` edilmiyor. Validator:
- root object kontrolü
- name/version kontrolü
- steps listesi
- step id/type
- duplicate step id
- depends_on tipi
- bilinmeyen dependency
kontrollerini yapıyor.

### 5. Script Editor sorumluluk ayrımı
WAT için artık açıkça:
- Kaydet
- Derle & Yükle
aksiyonları var.

Workflow JSON yalnızca doğrulanıp kaydediliyor. Derleme/deploy aksiyonu JSON'a karışmıyor.

### 6. Gerçek WASM backup/restore
Backup v2 `.aetheros` ZIP formatına geçirildi:
- `manifest.json`
- mevcut lokal WASM binary'leri
- script/WAT/Workflow JSON

Import sırasında mevcut binary'ler Rust ModuleStore'a tekrar upload edilir. API key hiçbir zaman yedeğe dahil edilmez. Eski v1 JSON backup'ları metadata/editor seviyesinde geriye dönük okunabilir.

> Not: Uygulamanın eski oturumlarında yalnızca hash metadata bulunuyorsa o eski binary'ler otomatik üretilemez. Yeni yüklenen WASM'ler lokal binary cache'e alınır.

## Uygulanan P2

### Design system
`AetherColors`, `AetherSpacing` ve `buildAetherTheme()` eklendi. Ortak Material 3 input/button/card/snackbar davranışları merkezileştirildi; ekranlardaki ana renkler ortak token'lara taşındı.

### Semantic interaction
Dokunulabilir kart/aksiyonların `GestureDetector` kullanımı `InkWell` tabanına taşındı. Böylece Material interaction feedback ve semantic button davranışı güçlendirildi.

### Global kullanıcı hatası
`userFacingError()` ile timeout, validation, backend ve genel hatalar için tek kullanıcı-facing hata katmanı eklendi. API/bridge kaynaklı ham exception metninin doğrudan UI'a sızması azaltıldı.

### Localization foundation
`flutter_localizations` eklendi; `tr` ve `en` locale desteği için Material/Widgets/Cupertino delegate'leri hazırlandı. Mevcut ürün dili Türkçe olarak korunuyor. Sonraki adım UI string'lerini ARB dosyalarına taşımaktır.

### Runtime model registry
`AiModelOption` artık yalnızca `id + label` değil:
- provider
- capabilities
- input/output modalities
- context window
- max output tokens
- lifecycle
bilgilerini taşıyabilecek provider-neutral sözleşmedir.

Cloud model listeleri runtime discovery ile gelir; OpenAI tarafında yeni model ID'lerini yakalamak için pozitif sabit allowlist kaldırıldı ve yalnızca bilinen non-chat aileleri filtreleniyor. Gemini model listesinde `generateContent` desteklemeyen modeller filtreleniyor.

Gemini API key URL query parametresi yerine `x-goog-api-key` header'ı ile gönderiliyor.

## CI / test kapısı
`.github/workflows/flutter-ci.yml` eklendi:
1. Flutter stable kurulumu
2. `flutter pub get`
3. `dart format --set-exit-if-changed lib test`
4. `flutter analyze`
5. `flutter test`
6. `flutter build apk --debug`
7. APK artifact upload

Yerel çalışma ortamında Flutter/Dart SDK bulunmadığından bu oturumda `flutter analyze`, `flutter test` ve APK build komutları fiilen çalıştırılamadı. Bu nedenle son doğrulama GitHub Actions tarafından yapılmalıdır.

## Kalan P0 / sonraki profesyonel aşama

- Release signing hâlâ CI secret/keystore ile yapılandırılmalı; mevcut debug signing üretim release'i için kullanılmamalı.
- Cancellation için Rust/FRB tarafında gerçek cancel API gerekiyorsa backend'e eklenmeli.
- AI chat geçmişi provider-neutral `Message` listesine taşınmalı; prompt string birleştirme kaldırılmalı.
- Model registry bir sonraki aşamada capability-based router'a dönüştürülmeli: `best`, `fastest`, `cheapest`, `vision`, `tools` vb.
- WASM büyük dosyaları için Dart heap yerine path/stream tabanlı upload değerlendirilmeli.
- Localization string'leri ARB + generated localizations yapısına taşınmalı.
