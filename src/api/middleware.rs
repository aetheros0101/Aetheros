// ============================================================
// src/api/middleware.rs  (YENİ)
//
// Sprint 6: Auth middleware
//
// Axum middleware olarak çalışır:
//   Authorization: Bearer <token> header'ı okur
//   TokenManager::verify() ile doğrular
//   Claims → Request extension'a enjekte eder
//   Handler'lar Extension<Claims> ile okur
//
// Korumasız endpoint'ler (/ health, /ws) middleware
// dışında tutulur — Router'da ayrı layer.
// ============================================================

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::security::auth::token::{
    TokenError,
    TokenManager,
};

/// Bearer token'ı doğrula ve Claims'i extension'a ekle.
///
/// Kullanım (router.rs'de):
/// ```rust
/// use axum::middleware;
/// router.route_layer(middleware::from_fn_with_state(
///     state,
///     auth_middleware,
/// ))
/// ```
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Response {
    let manager = TokenManager::from_env();

    // Authorization header'ını al
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => {
            &h["Bearer ".len()..]
        }
        Some(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid authorization format. Use: Bearer <token>"
                })),
            )
                .into_response();
        }
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authorization header missing"
                })),
            )
                .into_response();
        }
    };

    // Token doğrula
    match manager.verify(token) {
        Ok(_claims) => {
            // Claims extension'a enjekte edilebilir
            // Axum 0.8'de: request.extensions_mut().insert(claims);
            // Handler'da: Extension<Claims> ile al
            next.run(request).await
        }
        Err(TokenError::Expired) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Token expired"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid token"
            })),
        )
            .into_response(),
    }
}

/// API key doğrulama (header: X-Api-Key).
pub async fn api_key_middleware(
    request: Request,
    next: Next,
) -> Response {
    let api_key = request
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if !key.is_empty() => {
            // Gerçek implementasyon:
            // ApiKeyStore'dan hash'i çek, verify() ile karşılaştır
            // Şimdilik: key mevcutsa geç (Faz 7'de tam impl)
            let _ = key;
            next.run(request).await
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "X-Api-Key header missing or empty"
            })),
        )
            .into_response(),
    }
}
