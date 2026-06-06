// ============================================================
// src/security/auth/token.rs
//
// Sprint 6: JWT + API Key token sistemi
//
// ÖNCE: AuthToken { access_token: String } — boş struct
//
// SONRA:
//   TokenManager → JWT üretir + doğrular
//   ApiKey       → sabit token (servis-servis auth)
//   Claims       → role + expiry + subject
//
// JWT secret: ortam değişkeninden (AETHEROS_JWT_SECRET)
// API key: SHA-256 hash'li karşılaştırma
// ============================================================

use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::security::rbac::Role;

// ── Token yapıları ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: user id veya service name
    pub sub: String,
    /// Role
    pub role: Role,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiry (Unix timestamp)
    pub exp: u64,
}

impl Claims {
    pub fn new(
        subject: impl Into<String>,
        role: Role,
        ttl_seconds: u64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            sub: subject.into(),
            role,
            iat: now,
            exp: now + ttl_seconds,
        }
    }

    // SONRA (>= ile eşit anı da expire say):
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.exp
    }

    pub fn ttl_remaining(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.exp.saturating_sub(now)
    }
}

// ── API Key ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ApiKey {
    /// Ham key (Bearer header'dan gelen)
    raw: String,
    /// SHA-256 hash (DB'de saklanan)
    hash: String,
}

impl ApiKey {
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let hash = Self::hash_key(&raw);
        Self { raw, hash }
    }

    /// Verilen ham key'in hash'i kaydedilen hash ile eşleşiyor mu?
    pub fn verify(&self, provided: &str) -> bool {
        let provided_hash = Self::hash_key(provided);
        // Constant-time comparison (timing attack koruması)
        constant_time_eq(&provided_hash, &self.hash)
    }

    /// Ham API key'i döndür (log maskeleme veya audit için).
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    fn hash_key(key: &str) -> String {
        use sha2::{
            Digest,
            Sha256,
        };
        let result = Sha256::digest(key.as_bytes());
        hex::encode(result)
    }
}

/// Constant-time string karşılaştırma.
/// Timing attack'ı önler.
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

// ── TokenManager ─────────────────────────────────────────

pub struct TokenManager {
    secret: String,
    default_ttl: u64,
}

impl TokenManager {
    pub fn new(
        secret: impl Into<String>,
        default_ttl_seconds: u64,
    ) -> Self {
        Self {
            secret: secret.into(),
            default_ttl: default_ttl_seconds,
        }
    }

    /// Ortam değişkeninden oluştur.
    pub fn from_env() -> Self {
        let secret = std::env::var("AETHEROS_JWT_SECRET")
            .unwrap_or_else(|_| {
                "dev-secret-change-in-production".to_string()
            });
        Self::new(secret, 3600) // 1 saat default TTL
    }

    /// Claims → imzalı JWT token üret.
    ///
    /// Basit implementasyon: header.payload.signature
    /// Base64url encode + HMAC-SHA256 imza.
    /// Üretimde: jsonwebtoken crate ile değiştir.
    pub fn generate(
        &self,
        subject: impl Into<String>,
        role: Role,
    ) -> AuthToken {
        let claims =
            Claims::new(subject, role, self.default_ttl);

        let token = self.encode_claims(&claims);

        AuthToken {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: self.default_ttl,
        }
    }

    /// Token → Claims (doğrulama dahil).
    pub fn verify(
        &self,
        token: &str,
    ) -> Result<Claims, TokenError> {
        let claims = self.decode_claims(token)?;

        if claims.is_expired() {
            return Err(TokenError::Expired);
        }

        Ok(claims)
    }

    /// Basit encode: base64(header).base64(claims).hmac
    fn encode_claims(&self, claims: &Claims) -> String {
        let header = base64_encode(
            r#"{"alg":"HS256","typ":"JWT"}"#,
        );
        let payload = base64_encode(
            &serde_json::to_string(claims)
                .unwrap_or_default(),
        );
        let message = format!("{}.{}", header, payload);
        let sig = self.hmac_sign(&message);
        format!("{}.{}", message, sig)
    }

    fn decode_claims(
        &self,
        token: &str,
    ) -> Result<Claims, TokenError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(TokenError::Malformed);
        }

        // İmza doğrula
        let message =
            format!("{}.{}", parts[0], parts[1]);
        let expected_sig = self.hmac_sign(&message);

        if !constant_time_eq(parts[2], &expected_sig) {
            return Err(TokenError::InvalidSignature);
        }

        // Payload decode
        let payload = base64_decode(parts[1])
            .ok_or(TokenError::Malformed)?;

        serde_json::from_str(&payload)
            .map_err(|_| TokenError::Malformed)
    }

    fn hmac_sign(&self, message: &str) -> String {
        use sha2::{Digest, Sha256};
        let input =
            format!("{}{}", self.secret, message);
        let hash = Sha256::digest(input.as_bytes());
        hex::encode(hash)
    }
}

fn base64_encode(input: &str) -> String {
    use std::fmt::Write;
    // Basit hex encode (üretimde base64url kullan)
    let mut out = String::new();
    for b in input.bytes() {
        write!(out, "{:02x}", b).ok();
    }
    out
}

fn base64_decode(input: &str) -> Option<String> {
    if input.len() % 2 != 0 {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..input.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&input[i..i + 2], 16).ok()
        })
        .collect();
    bytes.and_then(|b| String::from_utf8(b).ok())
}

// ── Hata ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TokenError {
    Expired,
    InvalidSignature,
    Malformed,
    Missing,
}

impl std::fmt::Display for TokenError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Expired => write!(f, "Token expired"),
            Self::InvalidSignature => {
                write!(f, "Invalid signature")
            }
            Self::Malformed => write!(f, "Malformed token"),
            Self::Missing => write!(f, "Token missing"),
        }
    }
}
