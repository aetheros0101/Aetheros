// ============================================================
// src/api/middleware.rs
//
// Güvenlik sertleştirmesi (bkz. SECURITY_HARDENING.md):
//   Önceki durum: auth_middleware / api_key_middleware yazılmış ama
//   router.rs'te hiçbir route'a bağlanmamıştı — API tamamen açıktı.
//   api_key_middleware ayrıca boş olmayan HERHANGİ bir key'i kabul
//   ediyordu (gerçek bir karşılaştırma yapmıyordu).
//
// Şimdiki durum:
//   - authenticate(): X-Api-Key (ApiKeyStore, SHA-256 + constant-time)
//     veya Authorization: Bearer <JWT-lite> (TokenManager) doğrular,
//     her ikisi de bir AuthContext { subject, role } üretir.
//   - require_auth: herhangi bir geçerli kimlik ister, AuthContext'i
//     request extensions'a enjekte eder → handler'lar
//     `Extension<AuthContext>` ile role bilgisine erişebilir.
//   - require_admin: aynı doğrulamayı yapar + Role::Admin şartı koşar.
//   - issue_token_handler: AETHEROS_ADMIN_KEY ile korunan, JWT-lite
//     token üreten bootstrap endpoint'i (/auth/token).
//
// router.rs bu middleware'leri route_layer olarak gerçekten uyguluyor.
// ============================================================

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::security::auth::token::{
    constant_time_eq, ApiKeyStore, TokenError, TokenManager,
};
use crate::security::rbac::Role;

/// Bir isteğin kimliği doğrulandıktan sonra taşınan bağlam.
/// Handler'lar `axum::Extension<AuthContext>` ile erişebilir.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub subject: String,
    pub role: Role,
    /// Hangi mekanizma ile doğrulandığı (log/audit için).
    pub method: &'static str,
}

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

fn forbidden(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

/// X-Api-Key ile doğrulanan isteklere atanacak varsayılan rol.
/// AETHEROS_API_KEY_ROLE ile override edilebilir (admin/operator/viewer/agent).
/// Bilinmeyen/parse edilemeyen değerde en az yetkili role (Viewer) düşülür —
/// asla sessizce Admin'e yükseltilmez.
fn api_key_role() -> Role {
    let setting = std::env::var("AETHEROS_API_KEY_ROLE").ok();
    let role = role_from_setting(setting.as_deref());
    if setting.is_none() {
        tracing::warn!(
            "AETHEROS_API_KEY_ROLE ayarlanmamış — X-Api-Key istekleri geriye uyumluluk \
             için Operator rolüyle çalışıyor. Üretimde açıkça 'viewer' ya da \
             gereken en düşük rolü ayarlayın."
        );
    }
    role
}

/// Saf karar mantığı (test edilebilir):
///   - ayar YOK          → Operator (geriye uyumlu; uyarı loglanır)
///   - ayar var, GEÇERSİZ → Viewer (en az yetki; yazım hatası yetki YÜKSELTMEZ)
///   - ayar var, geçerli  → o rol
fn role_from_setting(setting: Option<&str>) -> Role {
    match setting {
        None => Role::Operator,
        Some(s) => Role::parse(s).unwrap_or(Role::Viewer),
    }
}

/// Bir isteği X-Cluster-Token, X-Api-Key veya Authorization: Bearer
/// header'ına göre doğrular. Hiçbiri yoksa/başarısızsa uygun bir 401
/// Response döner.
pub fn authenticate(headers: &header::HeaderMap) -> Result<AuthContext, Response> {
    // 0) X-Cluster-Token — node-to-node (cluster) çağrıları için paylaşımlı
    // sır. Bkz. src/remote/transport.rs (HttpTransport bu header'ı ekler).
    // Not: Bu, taşımayı ŞİFRELEMEZ — sadece göndereni doğrular. Gerçek
    // mTLS/TLS için nodlar arasına bir ters proxy (nginx/envoy) veya
    // rustls tabanlı bir istemci/sunucu kurulumu eklenmelidir.
    if let Some(cluster_token) = headers.get("X-Cluster-Token").and_then(|v| v.to_str().ok()) {
        let configured = std::env::var("AETHEROS_CLUSTER_TOKEN").unwrap_or_default();
        if configured.is_empty() {
            return Err(unauthorized(
                "Cluster auth is not configured on this server (AETHEROS_CLUSTER_TOKEN unset)",
            ));
        }
        if constant_time_eq(cluster_token, &configured) {
            return Ok(AuthContext {
                subject: "cluster-node".to_string(),
                role: Role::Admin,
                method: "cluster-token",
            });
        }
        return Err(unauthorized("Invalid X-Cluster-Token"));
    }

    // 1) X-Api-Key — servis-servis / basit entegrasyonlar için.
    if let Some(key) = headers.get("X-Api-Key").and_then(|v| v.to_str().ok()) {
        let store = ApiKeyStore::from_env();
        if store.is_empty() {
            // AETHEROS_API_KEYS hiç ayarlanmamışsa bu mekanizma tamamen
            // devre dışıdır — sessizce geçmek yerine net bir hata veriyoruz.
            return Err(unauthorized(
                "X-Api-Key auth is not configured on this server",
            ));
        }
        if store.is_valid(key) {
            return Ok(AuthContext {
                subject: "api-key".to_string(),
                role: api_key_role(),
                method: "api-key",
            });
        }
        return Err(unauthorized("Invalid X-Api-Key"));
    }

    // 2) Authorization: Bearer <token>
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h["Bearer ".len()..],
        Some(_) => {
            return Err(unauthorized(
                "Invalid authorization format. Use: Bearer <token>",
            ))
        }
        None => {
            return Err(unauthorized(
                "Authentication required: provide X-Api-Key or Authorization: Bearer <token>",
            ))
        }
    };

    let manager = TokenManager::from_env();
    match manager.verify(token) {
        Ok(claims) => Ok(AuthContext {
            subject: claims.sub,
            role: claims.role,
            method: "jwt",
        }),
        Err(TokenError::Expired) => Err(unauthorized("Token expired")),
        Err(_) => Err(unauthorized("Invalid token")),
    }
}

/// `audit` hedefiyle yapılandırılmış bir denetim satırı yazar. Bunu ayrı
/// bir dosyaya/backend'e yönlendirmek için tracing-subscriber tarafında
/// `target("audit")` filtreli bir katman/exporter eklemek yeterlidir —
/// bkz. src/logging/exporters/.
fn audit_log(outcome: &str, path: &str, ctx: Option<&AuthContext>, detail: &str) {
    match ctx {
        Some(c) => tracing::info!(
            target: "audit",
            outcome,
            path,
            subject = %c.subject,
            role = ?c.role,
            method = c.method,
            "auth decision"
        ),
        None => tracing::warn!(
            target: "audit",
            outcome,
            path,
            detail,
            "auth decision"
        ),
    }
}

/// Herhangi bir geçerli kimlik ister (rol farketmez).
/// Başarılı olursa `AuthContext`'i request extensions'a ekler.
pub async fn require_auth(mut request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    match authenticate(request.headers()) {
        Ok(ctx) => {
            audit_log("allow", &path, Some(&ctx), "");
            request.extensions_mut().insert(ctx);
            next.run(request).await
        }
        Err(resp) => {
            audit_log("deny", &path, None, "authentication failed");
            resp
        }
    }
}

/// Geçerli kimlik + Role::Admin ister. Cluster yönetimi, uzak komut
/// çalıştırma ve modül yükleme gibi yüksek riskli uçlar için kullanılır.
pub async fn require_admin(mut request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    match authenticate(request.headers()) {
        Ok(ctx) if ctx.role.is_admin() => {
            audit_log("allow", &path, Some(&ctx), "");
            request.extensions_mut().insert(ctx);
            next.run(request).await
        }
        Ok(ctx) => {
            audit_log("deny", &path, Some(&ctx), "admin role required");
            forbidden(&format!(
                "Role '{:?}' cannot access this endpoint (Admin required)",
                ctx.role
            ))
        }
        Err(resp) => {
            audit_log("deny", &path, None, "authentication failed");
            resp
        }
    }
}

/// WebSocket upgrade'i header set edemeyen istemciler (tarayıcı
/// `new WebSocket(url)`) için query string üzerinden doğrular:
///   /ws?token=<jwt>   veya   /ws?api_key=<key>
pub fn authenticate_ws(token: Option<&str>, api_key: Option<&str>) -> bool {
    if let Some(key) = api_key {
        let store = ApiKeyStore::from_env();
        return !store.is_empty() && store.is_valid(key);
    }
    if let Some(t) = token {
        return TokenManager::from_env().verify(t).is_ok();
    }
    false
}

// ── /auth/token — bootstrap token issuance ─────────────────

#[derive(Debug, Deserialize)]
pub struct IssueTokenRequest {
    /// AETHEROS_ADMIN_KEY ile karşılaştırılan paylaşılan sır.
    pub admin_key: String,
    pub subject: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct IssueTokenError {
    pub error: String,
}

/// POST /auth/token — AETHEROS_ADMIN_KEY bilinen taraflara JWT-lite
/// token üretir. AETHEROS_ADMIN_KEY ayarlanmamışsa uç tamamen kapalıdır
/// (boş admin_key asla eşleşmez).
pub async fn issue_token_handler(Json(req): Json<IssueTokenRequest>) -> Response {
    let configured = std::env::var("AETHEROS_ADMIN_KEY").unwrap_or_default();

    if configured.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(IssueTokenError {
                error: "Token issuance is disabled: AETHEROS_ADMIN_KEY is not set".into(),
            }),
        )
            .into_response();
    }

    if req.admin_key.is_empty() || !constant_time_eq(&req.admin_key, &configured) {
        return unauthorized("Invalid admin key");
    }

    let role = match Role::parse(&req.role) {
        Some(r) => r,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(IssueTokenError {
                    error: "role must be one of: admin, operator, viewer, agent".into(),
                }),
            )
                .into_response()
        }
    };

    let manager = TokenManager::from_env();
    let token = manager.generate(req.subject, role);
    Json(token).into_response()
}

#[cfg(test)]
mod role_setting_tests {
    use super::*;

    #[test]
    fn unset_keeps_backward_compatible_operator() {
        assert_eq!(role_from_setting(None), Role::Operator);
    }

    #[test]
    fn invalid_value_falls_to_least_privilege_viewer() {
        assert_eq!(role_from_setting(Some("adm1n")), Role::Viewer);
        assert_eq!(role_from_setting(Some("")), Role::Viewer);
    }

    #[test]
    fn valid_values_are_honored_case_insensitively() {
        assert_eq!(role_from_setting(Some("viewer")), Role::Viewer);
        assert_eq!(role_from_setting(Some("ADMIN")), Role::Admin);
        assert_eq!(role_from_setting(Some("Operator")), Role::Operator);
    }
}
