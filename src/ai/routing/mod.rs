// ============================================================
// src/ai/routing/mod.rs
//
// Haziran 2026 temizliği:
//   1) Bu dosyada daha önce `routing::router::ProviderRouter` ile
//      isim çakışan, kullanılmayan ikinci bir `ProviderRouter` +
//      `AiProvider` trait çifti vardı (hiçbir somut provider bu
//      trait'i implemente etmiyordu). Kaldırıldı.
//   2) `routing::policy::RoutingPolicy` de tamamen kullanılmıyordu
//      (ayrıca `ai::policies::RoutingPolicy` ile aynı isimde başka
//      bir kopyası daha vardı — o da kaldırıldı). Provider seçimi
//      artık otomatik policy değil, kullanıcının Ayarlar'daki
//      seçimi (bkz. ProviderRouter::set_active).
//
// Tek geçerli router: routing::router::ProviderRouter.
// ============================================================

pub mod router;
