# AetherOS Flutter — Professionalization Changelog

## 2026-09-13

- Dinamik cloud model discovery modeli provider-neutral metadata ile genişletildi.
- OpenAI model discovery pozitif model prefix allowlist'inden çıkarıldı.
- Gemini API key URL'den header'a taşındı.
- Lifecycle-aware polling eklendi.
- Workflow retry step cache persist edildi.
- Workflow JSON validator eklendi.
- Script Editor'da save/compile-deploy ayrıştırıldı.
- WASM binary backup/restore için `.aetheros` ZIP formatı eklendi.
- Ortak Material 3 theme/design token katmanı eklendi.
- GestureDetector tabanlı dokunulabilir kontroller InkWell'e taşındı.
- Global kullanıcı-facing error mapper eklendi.
- Flutter localization altyapısı eklendi.
- Unit testler: model registry ve workflow validator.
- GitHub Actions: format, analyze, test ve debug APK build kapısı eklendi.

## Doğrulama notu

Bu geliştirme ortamında Flutter/Dart SDK mevcut olmadığı için gerçek Flutter test/build çalıştırılamadı. CI dosyası cihaz testinden önce otomatik kalite kapısı olarak kullanılmalıdır.
