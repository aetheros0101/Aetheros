# AetherOS Flutter İnceleme — v8 → hardened review

Tarih: 2026-09-09

## Yapılan kritik düzeltmeler

1. **Task Retry gerçek retry yapıyor.**
   - Eski davranış `SubmitTaskScreen` açıp kullanıcıyı yeni/boş task formuna gönderiyordu.
   - Yeni davranış `resubmitTask(taskId)` çağırarak backend'in mevcut task ayarlarıyla yeni UUID üretmesini kullanıyor.

2. **Eski/çift AI ayar ekranı kaldırıldı.**
   - `SettingsScreen` içinde API key'ler SharedPreferences'a düz metin yazılıyordu.
   - Aynı zamanda gerçek `AiProviderService` zaten `flutter_secure_storage` kullanıyordu.
   - Artık genel Ayarlar yalnızca güvenli `AiSettingsScreen`'e yönlendiriyor.

3. **AI provider kayıt sırası düzeltildi.**
   - Önce Rust router'a uygulanıyor, sonra kalıcı storage'a yazılıyor.
   - Bridge başarısız olduğunda cihazda `enabled=true` fakat çalışmayan provider bırakılması önlendi.
   - Aktif provider seçimi de önce Rust tarafında başarılı olup sonra preference'a yazılıyor.

4. **Unmounted async state güncellendi.**
   - Task submit, Agent, Workflow, Remote, AI Chat, AI Settings, Script Editor, WASM ve Backup akışlarında await sonrası `setState` güvenliği güçlendirildi.

5. **Workflow bağımlılık bug'ı düzeltildi.**
   - Bir step silindiğinde sonraki step'lerde kalan `dependsOn` referansları temizleniyor.
   - Böylece backend'e olmayan step ID'leri gönderilmiyor.

6. **Workflow JSON artık kaydetmeden önce parse ediliyor.**
   - Bozuk JSON kalıcı olarak kaydedilmiyor.

7. **WASM metadata dayanıklılığı artırıldı.**
   - Bozuk SharedPreferences kaydı tüm ekranı düşürmüyor.
   - Runtime'da artık bulunmayan modül metadata'sı listeden temizleniyor.

8. **Log araması debounce edildi.**
   - Her tuş vuruşunda bridge çağrısı yerine 350 ms bekleniyor.

9. **Backup mesajları gerçek kapsamı yansıtıyor.**
   - JSON yedeğinin WASM binary içermediği ve AI API key'lerinin yeniden girilmesi gerektiği açıkça belirtiliyor.
   - Backup version doğrulaması eklendi.

10. **Dart analyzer konfigürasyonu eklendi.**
    - strict casts/inference/raw types ve kritik analyzer hataları etkin.

## Henüz çözülmemiş, release öncesi önemli konular

### P0 / Release blocker

- Flutter toolchain bu çalışma ortamında kurulu olmadığı için `flutter analyze`, `flutter test` ve `flutter build apk --release` çalıştırılamadı.
- Android `release` build'i hâlâ `signingConfigs.debug` kullanıyor. Production/Play Store için gerçek release keystore + secrets tabanlı signing gerekir.

### P1

- Flutter tarafında REST API ile backend'in sahip olduğu `/tasks/{id}/cancel` endpoint'i için FRB bridge fonksiyonu yok. Bu yüzden UI'da gerçek cancellation action eklenmedi. Backend REST endpoint'i var; Flutter bridge'e güvenli şekilde expose edilip codegen çalıştırılmalı.
- Backup şu an **metadata backup**. WASM binary'leri geri yüklenmiyor. Gerçek cihazlar arası tam backup için binary bundle/ZIP veya module re-upload akışı tasarlanmalı.
- `ScriptEditorScreen` WAT kaydetmeyi doğrudan compile+upload olarak yorumluyor; editör ile deployment birbirinden ayrılmalı.
- Workflow JSON editörü yalnızca taslak saklıyor; gerçek workflow submit pipeline'ına bağlanmış bir JSON parser/validator yok.
- Ekranların çoğu doğrudan generated Rust API import ediyor. `AetherApi` abstraction mevcutken tüm ekranlar tek API katmanına geçirilmeli.

### P2 / kalite ve performans

- Task/Agent/Workflow/Remote/Log ekranlarının her biri bağımsız polling yapıyor. Merkezi repository/state katmanı + lifecycle-aware polling/websocket tercih edilmeli.
- WASM listesi açılışta modülleri tek tek `checkModuleExists` ile doğruluyor; 256 modülde gereksiz seri bridge çağrısı oluşabilir. Batch `listModuleMetadata` tercih edilmeli.
- WASM picker `withData:true` ile binary'yi Dart heap'e alıyor. 64 MiB backend limitine yakın dosyalarda mobil RAM baskısı oluşabilir. Path/stream tabanlı yükleme daha iyi.
- Log polling için backoff, visibility/lifecycle kontrolü ve websocket birinci tercih olmalı.
- Custom `GestureDetector` tabanlı butonlar erişilebilirlik/keyboard/ripple semantics açısından `InkWell`, `FilledButton`, `IconButton` gibi semantic widget'larla standartlaştırılmalı.
- UI metinleri ve state string'leri (`Failed`, `Executing`, vb.) tek domain mapper üzerinden yönetilmeli; backend enum string'lerine ekranların dağınık biçimde bağımlılığı azaltılmalı.

## Backend stres testinden Flutter'a yansıyan önemli nokta

Stres testinde `completed=0`, `failed=2119` görülmesi tek başına Flutter bug'ı değildir; testlerin önemli bölümü gerçek/çalışabilir WASM execution yerine hata yollarını doğruluyor. Flutter'ın özellikle **Failed state + error_message + retry** UX'i bu nedenle kritik.

## Önerilen son aşama

1. Flutter SDK ile `flutter pub get`.
2. `dart format --set-exit-if-changed .`.
3. `flutter analyze`.
4. Unit + widget testleri.
5. Android debug cihaz smoke test.
6. WASM upload → task submit → status → failure → retry tam akışı.
7. AI provider save → restart → rehydrate → chat.
8. Ollama local/LAN bağlantı testi.
9. Backup/export/import ve eksik binary davranışı.
10. Release signing + APK/AAB build.
