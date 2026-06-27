//pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod provider;

/// Bazı mobil operatörlerde cihaz IPv6 adresi alır ama gerçek
/// yönlendirme yoktur → "Network unreachable" (Android'de aynı
/// SocketException: errno 101 — Dart tarafında da görüldü ve
/// main.dart'ta IPv4'e zorlanarak çözüldü). Çıkış soketini IPv4'e
/// sabitleyerek Rust HTTP istemcilerini de aynı sorundan koruyoruz.
/// Builder her nasılsa başarısız olursa varsayılan client'a düşülür.
pub(crate) fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
