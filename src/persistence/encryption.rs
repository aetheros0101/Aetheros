// Authenticated at-rest encryption for sled values.
//
// AETHEROS_ENCRYPTION_KEY: exactly 32 bytes encoded as 64 hex characters.
// Stored format: 12-byte random nonce || AES-256-GCM ciphertext+tag.
// If the key is absent, plaintext compatibility is retained for local/dev
// use, with an explicit warning. Production must configure the key.

use ring::aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};

use crate::errors::persistence::PersistenceError;

pub trait AtRestCipher: Send + Sync {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, PersistenceError>;
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, PersistenceError>;
}

pub struct NoopCipher;

impl AtRestCipher for NoopCipher {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        Ok(plaintext.to_vec())
    }
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        Ok(ciphertext.to_vec())
    }
}

pub struct Aes256GcmCipher {
    key: LessSafeKey,
    rng: SystemRandom,
}

impl Aes256GcmCipher {
    pub fn from_hex(hex_key: &str) -> Result<Self, PersistenceError> {
        let key_bytes = hex::decode(hex_key).map_err(|_| PersistenceError::CryptoFailure)?;
        if key_bytes.len() != 32 {
            return Err(PersistenceError::CryptoFailure);
        }
        let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
            .map_err(|_| PersistenceError::CryptoFailure)?;
        Ok(Self {
            key: LessSafeKey::new(unbound),
            rng: SystemRandom::new(),
        })
    }
}

impl AtRestCipher for Aes256GcmCipher {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        let mut nonce_bytes = [0u8; 12];
        self.rng
            .fill(&mut nonce_bytes)
            .map_err(|_| PersistenceError::CryptoFailure)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let mut buf = plaintext.to_vec();
        self.key
            .seal_in_place_append_tag(nonce, Aad::empty(), &mut buf)
            .map_err(|_| PersistenceError::CryptoFailure)?;
        let mut result = Vec::with_capacity(12 + buf.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&buf);
        Ok(result)
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        if ciphertext.len() < 28 {
            return Err(PersistenceError::CryptoFailure);
        }
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(&ciphertext[..12]);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let mut buf = ciphertext[12..].to_vec();
        let plaintext = self
            .key
            .open_in_place(nonce, Aad::empty(), &mut buf)
            .map_err(|_| PersistenceError::CryptoFailure)?;
        Ok(plaintext.to_vec())
    }
}

pub fn cipher_from_env() -> Box<dyn AtRestCipher> {
    match std::env::var("AETHEROS_ENCRYPTION_KEY") {
        Ok(key) if !key.is_empty() => match Aes256GcmCipher::from_hex(&key) {
            Ok(cipher) => {
                tracing::info!("sled at-rest AES-256-GCM encryption enabled");
                Box::new(cipher)
            }
            Err(_) => {
                panic!("AETHEROS_ENCRYPTION_KEY must be exactly 32 bytes / 64 hex characters")
            }
        },
        _ => {
            tracing::warn!(
                "AETHEROS_ENCRYPTION_KEY unset — sled values are NOT encrypted. \
                 Configure a 32-byte hex key for production."
            );
            Box::new(NoopCipher)
        }
    }
}
