// ============================================================
// src/wasm/module_store.rs  (YENİ)
//
// Optimizasyon #1: wasm_module → hash referansı
//
// SORUN:
//   TaskDefinition.wasm_module: Vec<u8>
//   - Her clone → MB'larca kopya
//   - Her persist → base64(binary) → JSON → disk
//   - Aynı modülü kullanan 100 task → 100 kopya
//
// ÇÖZÜM: ModuleStore
//   - Hash → binary KV deposu (DashMap)
//   - TaskDefinition sadece 32 byte hash taşır
//   - Binary bir kez kaydedilir, çoğu task paylaşır
//   - WasmEngine zaten SHA-256 cache'i var →
//     ModuleStore aynı hash'i anahtar olarak kullanır
//     → binary lookup + compilation lookup tek adım
//
// Yaşam döngüsü:
//   Upload:  binary → ModuleStore::store() → [u8;32] hash
//   Submit:  TaskDefinition { wasm_module_hash, .. }
//   Execute: WasmEngine → ModuleStore::get(hash) → binary
//            → get_or_compile(binary) → Module cache hit
// ============================================================

use std::sync::Arc;

use dashmap::DashMap;
use sha2::{
    Digest,
    Sha256,
};
use tracing::{
    debug,
    warn,
};

pub type ModuleHash = [u8; 32];

#[derive(Debug, Clone)]
pub enum ModuleStoreError {
    NotFound(ModuleHash),
    InvalidBinary,
}

impl std::fmt::Display for ModuleStoreError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::NotFound(h) => {
                write!(f, "Module not found: {}", hex::encode(h))
            }
            Self::InvalidBinary => {
                write!(f, "Invalid WASM binary")
            }
        }
    }
}

pub struct ModuleStore {
    /// hash → Arc<Vec<u8>>
    ///
    /// Arc: binary birden fazla engine thread'inde
    /// paylaşılabilir — kopya yok.
    modules: DashMap<ModuleHash, Arc<Vec<u8>>>,
}

impl ModuleStore {
    pub fn new() -> Self {
        Self {
            modules: DashMap::new(),
        }
    }

    /// Binary'yi depola → hash döndür.
    ///
    /// Aynı binary tekrar gönderilirse hash döner,
    /// ikinci kez depolanmaz (idempotent).
    pub fn store(
        &self,
        binary: Vec<u8>,
    ) -> Result<ModuleHash, ModuleStoreError> {
        if binary.is_empty() {
            return Err(ModuleStoreError::InvalidBinary);
        }

        let hash: ModuleHash =
            Sha256::digest(&binary).into();

        // Zaten varsa overwrite etme
        self.modules
            .entry(hash)
            .or_insert_with(|| Arc::new(binary));

        debug!(
            hash = %hex::encode(hash),
            "Module stored"
        );

        Ok(hash)
    }

    /// Hash'e göre binary'yi al.
    pub fn get(
        &self,
        hash: &ModuleHash,
    ) -> Result<Arc<Vec<u8>>, ModuleStoreError> {
        self.modules
            .get(hash)
            .map(|entry| Arc::clone(&entry))
            .ok_or(ModuleStoreError::NotFound(*hash))
    }

    /// Binary mevcutsa true.
    pub fn contains(&self, hash: &ModuleHash) -> bool {
        self.modules.contains_key(hash)
    }

    /// Depolanan modül sayısı.
    pub fn count(&self) -> usize {
        self.modules.len()
    }

    /// Tüm hash'leri hex string listesi olarak döndür.
    /// GET /modules endpoint'i için.
    pub fn list_hashes(&self) -> Vec<String> {
        self.modules
            .iter()
            .map(|entry| Self::hash_to_hex(entry.key()))
            .collect()
    }

    /// Hash'i hex string'e çevir (API response için).
    pub fn hash_to_hex(hash: &ModuleHash) -> String {
        hex::encode(hash)
    }

    /// Hex string'i hash'e çevir (API request için).
    pub fn hex_to_hash(
        hex_str: &str,
    ) -> Result<ModuleHash, ModuleStoreError> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| ModuleStoreError::InvalidBinary)?;

        if bytes.len() != 32 {
            return Err(ModuleStoreError::InvalidBinary);
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(hash)
    }

    /// Belirli bir hash için binary boyutunu döndür.
    pub fn binary_size(
        &self,
        hash: &ModuleHash,
    ) -> Option<usize> {
        self.modules
            .get(hash)
            .map(|b| b.len())
    }

    /// Kullanılmayan modülleri temizle (Faz 9: LRU eviction).
    /// Şimdilik tümünü temizler — monitoring endpoint için.
    pub fn clear(&self) {
        warn!("Clearing all modules from store");
        self.modules.clear();
    }
}
