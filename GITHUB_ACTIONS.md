# AetherOS — GitHub Actions ile Android APK Build

## Ön Koşullar (Repo'ya Eklenmesi Gerekenler)

### 1. pubspec.yaml düzeltmesi
`flutter_app/pubspec.yaml`'a `path_provider` ekle (main.dart'ta kullanılıyor):
```yaml
dependencies:
  path_provider: ^2.1.3
```

### 2. Android scaffold dosyaları
Bu dosyaları repo'ya commit et:
```
flutter_app/android/build.gradle              ← kök build
flutter_app/android/settings.gradle
flutter_app/android/gradle.properties
flutter_app/android/gradle/wrapper/gradle-wrapper.properties
.github/workflows/build_apk.yml
```

---

## Workflow Mimarisi

```
Push / PR / Manuel tetikle
          │
          ▼
┌─────────────────────────┐    ┌─────────────────────────┐
│  🦀 Rust → arm64-v8a   │    │ 🦀 Rust → armeabi-v7a  │
│  (paralel)              │    │ (paralel)               │
│  cargo ndk --release    │    │ cargo ndk --release     │
│  --features backend-    │    │ --features backend-     │
│  wasmi                  │    │ wasmi                   │
└──────────┬──────────────┘    └──────────┬──────────────┘
           │  libaetheros.so              │  libaetheros.so
           └─────────────────┬────────────┘
                             ▼
              ┌──────────────────────────┐
              │  📱 Flutter APK Build   │
              │  flutter build apk       │
              │  --release               │
              │  --split-per-abi        │
              └──────────┬───────────────┘
                         │
              ┌──────────▼───────────────┐
              │  📤 Artifacts            │
              │  app-arm64-v8a-release   │
              │  app-armeabi-v7a-release │
              │  (30 gün saklanır)       │
              └──────────────────────────┘
```

## Tetikleme Yöntemleri

| Yöntem | Açıklama |
|--------|----------|
| `git push main` | Otomatik build |
| `git push develop` | Otomatik build |
| `git push release/*` | Otomatik build |
| PR to main | Otomatik build |
| Actions → Run workflow | Manuel tetikle |

## APK'yı Nereden İndirirsin?

GitHub → Actions → İlgili workflow çalışması → **Artifacts** bölümü →
`aetheros-apk-<build_no>` zip dosyasını indir.

## Versiyonlama

APK `versionCode` otomatik olarak `github.run_number`'dan alınır.
`pubspec.yaml`'daki `versionName` korunur.

## Önemli Notlar

- **wasmi backend**: Android build'inde JIT (wasmtime/Cranelift) kullanılamaz.
  Play Store politikası gereği `backend-wasmi` zorunludur.
- **NDK 27.0.12077973**: `build.gradle`'daki ndkVersion ile eşleşmeli.
- **Min SDK 24**: Rust std + wasmi için Android 7.0+ gerekli.
