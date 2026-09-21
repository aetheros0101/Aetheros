use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::security::rbac::Role;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: Role,
    pub iat: u64,
    pub exp: u64,
}

impl Claims {
    pub fn new(subject: impl Into<String>, role: Role, ttl_seconds: u64) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        Self { sub: subject.into(), role, iat: now, exp: now + ttl_seconds }
    }
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        now >= self.exp
    }
    pub fn ttl_remaining(&self) -> u64 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        self.exp.saturating_sub(now)
    }
}

#[derive(Debug, Clone)]
pub struct ApiKey {
    hash: String,
}

impl ApiKey {
    pub fn new(raw: impl Into<String>) -> Self {
        Self { hash: Self::hash_key(&raw.into()) }
    }
    pub fn verify(&self, provided: &str) -> bool {
        constant_time_eq(&Self::hash_key(provided), &self.hash)
    }
    pub fn hash(&self) -> &str { &self.hash }
    fn hash_key(key: &str) -> String {
        hex::encode(Sha256::digest(key.as_bytes()))
    }
}

pub struct ApiKeyStore { keys: Vec<ApiKey> }

impl ApiKeyStore {
    pub fn new(raw_keys: Vec<String>) -> Self {
        Self { keys: raw_keys.into_iter().map(ApiKey::new).collect() }
    }
    pub fn from_env() -> Self {
        let raw = std::env::var("AETHEROS_API_KEYS").unwrap_or_default();
        let keys = raw.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
        Self::new(keys)
    }
    pub fn is_empty(&self) -> bool { self.keys.is_empty() }
    pub fn is_valid(&self, provided: &str) -> bool {
        !provided.is_empty() && self.keys.iter().any(|k| k.verify(provided))
    }
}

pub(crate) fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() { return false; }
    a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub struct TokenManager {
    secret: String,
    default_ttl: u64,
}

impl TokenManager {
    pub fn new(secret: impl Into<String>, default_ttl_seconds: u64) -> Self {
        Self { secret: secret.into(), default_ttl: default_ttl_seconds }
    }

    pub fn from_env() -> Self {
        static EPHEMERAL_SECRET: OnceLock<String> = OnceLock::new();
        let secret = match std::env::var("AETHEROS_JWT_SECRET") {
            Ok(s) if !s.is_empty() => s,
            _ => EPHEMERAL_SECRET.get_or_init(|| {
                let bytes: [u8; 32] = rand::random();
                tracing::error!(
                    "AETHEROS_JWT_SECRET ayarlanmamış — süreç başına rastgele JWT secret kullanılıyor. \
                     Production'da kalıcı bir secret tanımlayın."
                );
                hex::encode(bytes)
            }).clone(),
        };
        let ttl = std::env::var("AETHEROS_JWT_TTL_SECONDS")
            .ok().and_then(|v| v.parse().ok()).filter(|v: &u64| *v > 0).unwrap_or(3600);
        Self::new(secret, ttl)
    }

    pub fn generate(&self, subject: impl Into<String>, role: Role) -> AuthToken {
        let claims = Claims::new(subject, role, self.default_ttl);
        AuthToken {
            access_token: self.encode_claims(&claims),
            token_type: "Bearer".to_string(),
            expires_in: self.default_ttl,
        }
    }

    pub fn verify(&self, token: &str) -> Result<Claims, TokenError> {
        let claims = self.decode_claims(token)?;
        if claims.is_expired() { return Err(TokenError::Expired); }
        Ok(claims)
    }

    fn encode_claims(&self, claims: &Claims) -> String {
        let header = base64url_encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64url_encode(&serde_json::to_vec(claims).unwrap_or_default());
        let message = format!("{}.{}", header, payload);
        format!("{}.{}", message, self.hmac_sign(&message))
    }

    fn decode_claims(&self, token: &str) -> Result<Claims, TokenError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 { return Err(TokenError::Malformed); }

        let message = format!("{}.{}", parts[0], parts[1]);
        let expected_sig = self.hmac_sign(&message);
        if !constant_time_eq(parts[2], &expected_sig) {
            return Err(TokenError::InvalidSignature);
        }

        // Algoritma confusion riskini kapat: header gerçekten HS256/JWT olmalı.
        let header = base64url_decode(parts[0]).ok_or(TokenError::Malformed)?;
        if header != br#"{"alg":"HS256","typ":"JWT"}"# {
            return Err(TokenError::Malformed);
        }

        let payload = base64url_decode(parts[1]).ok_or(TokenError::Malformed)?;
        serde_json::from_slice(&payload).map_err(|_| TokenError::Malformed)
    }

    fn hmac_sign(&self, message: &str) -> String {
        // RFC 2104 HMAC-SHA256; SHA256(secret || message) DEĞİL.
        let mut key = self.secret.as_bytes().to_vec();
        if key.len() > 64 { key = Sha256::digest(&key).to_vec(); }
        key.resize(64, 0);

        let mut ipad = [0x36u8; 64];
        let mut opad = [0x5cu8; 64];
        for i in 0..64 {
            ipad[i] ^= key[i];
            opad[i] ^= key[i];
        }

        let mut inner = Sha256::new();
        inner.update(ipad);
        inner.update(message.as_bytes());
        let inner_hash = inner.finalize();

        let mut outer = Sha256::new();
        outer.update(opad);
        outer.update(inner_hash);
        hex::encode(outer.finalize())
    }
}

fn base64url_encode(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((input.len() * 4 + 2) / 3);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i+1] as u32) << 8) | input[i+2] as u32;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        1 => {
            let n = (input[i] as u32) << 16;
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
        }
        2 => {
            let n = ((input[i] as u32) << 16) | ((input[i+1] as u32) << 8);
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push(T[((n >> 6) & 63) as usize] as char);
        }
        _ => {}
    }
    out
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    fn v(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'-' => Some(62), b'_' => Some(63), _ => None,
        }
    }
    let b = input.as_bytes();
    if b.len() % 4 == 1 { return None; }
    let mut out = Vec::with_capacity(b.len() * 3 / 4);
    let mut i = 0;
    while i + 4 <= b.len() {
        let n = ((v(b[i])? as u32) << 18)
            | ((v(b[i+1])? as u32) << 12)
            | ((v(b[i+2])? as u32) << 6)
            | v(b[i+3])? as u32;
        out.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8]);
        i += 4;
    }
    match b.len() - i {
        0 => {}
        2 => {
            let n = ((v(b[i])? as u32) << 18) | ((v(b[i+1])? as u32) << 12);
            out.push((n >> 16) as u8);
        }
        3 => {
            let n = ((v(b[i])? as u32) << 18) | ((v(b[i+1])? as u32) << 12) | ((v(b[i+2])? as u32) << 6);
            out.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8]);
        }
        _ => unreachable!(),
    }
    Some(out)
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenError { Expired, InvalidSignature, Malformed, Missing }

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expired => write!(f, "Token expired"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::Malformed => write!(f, "Malformed token"),
            Self::Missing => write!(f, "Token missing"),
        }
    }
}
