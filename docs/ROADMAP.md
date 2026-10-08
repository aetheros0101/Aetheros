# AetherOS Yol Haritası

**Konumlandırma:** denetlenebilir, yönetilen (governed) agent çalışma zamanı.
Farklılaştırıcı: capability → risk → onay → audit hattı. Genel amaçlı dağıtık
orkestrasyon bu vizyonun dışında; `orchestration` altında yalnızca workflow
grafı ve koordinasyon tutulur.

## Hedef crate yapısı

| Crate | Sorumluluk |
|---|---|
| `aetheros-core` | tipler, hatalar, olaylar |
| `aetheros-runtime` | task, worker, wasm, persistence |
| `aetheros-agents` | governance, planlama, bellek, araçlar, gözlemlenebilirlik |
| `aetheros-security` | capability, risk, politika, audit, RBAC |
| `aetheros-ai` | provider'lar ve router |
| `aetheros-workspace`, `aetheros-terminal` | (mevcut) dosya/git ve PTY |
| `aetheros-app` | tek servis katmanı (use-case'ler) |
| `aetheros-api`, `aetheros-bridge`, `aetheros-server` | ince adaptörler |

## Fazlar

- **Faz 0 — Hijyen (tamam):** ölü kod, Docker, bağımlılıklar. Bkz. `CHANGES.md`.
- **Faz 1 — Birleştirme (devam ediyor):** `types → agents` ve diğer modül
  döngüleri kırıldı (bkz. `CHANGES.md`, mimari testleri korur). Kalan: `ai/{memory,planner,tools,context}` ile `agents/*`
  çakışmasını gider; `types → agents` döngüsünü kır; `unreachable_pub` uyarısını
  aç, gereksiz `pub`'ları daralt.
- **Faz 2 — Servis katmanı:** `bridge/api.rs`'i domain'lere böl; global
  `OnceLock<MobileRuntime>` yerine enjekte edilen `AppContext`; `main.rs` ve
  `bridge/state.rs` bootstrap'ini tek yerde birleştir.
- **Faz 3 — Kurumsal:** agent/workflow kayıtlarını kalıcı yap; WASM modül imza
  doğrulamasını zorunlu kıl; varsayılan roller ve API anahtarı politikasını
  sıkılaştır; tenant izolasyonunu (`IsolationScope`) uçtan uca bağla;
  `sled` yerine bakımı olan bir depolama değerlendir.
- **Faz 4 — Kalite:** `observability`, `planning`, `runtime` testleri;
  `path_guard`/`command_policy` fuzz; `cargo-deny`; MSRV kontrolü; API için
  yetkilendirme entegrasyon testleri.
