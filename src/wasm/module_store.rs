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

use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

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
    SizeLimitExceeded,
    StoreLimitExceeded,
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
            Self::InvalidBinary => write!(f, "Invalid WASM binary"),
            Self::SizeLimitExceeded => write!(f, "WASM module exceeds configured size limit"),
            Self::StoreLimitExceeded => write!(f, "WASM module store resource limit exceeded"),
        }
    }
}

pub struct ModuleStore {
    /// hash → Arc<Vec<u8>>
    ///
    /// Arc: binary birden fazla engine thread'inde
    /// paylaşılabilir — kopya yok.
    modules: DashMap<ModuleHash, Arc<Vec<u8>>>,
    total_bytes: AtomicUsize,
    max_module_bytes: usize,
    max_modules: usize,
    max_total_bytes: usize,
}

impl ModuleStore {
    pub fn new() -> Self {
        Self {
            modules: DashMap::new(),
            total_bytes: AtomicUsize::new(0),
            max_module_bytes: env_limit("AETHEROS_MAX_MODULE_SIZE_BYTES", 64 * 1024 * 1024),
            max_modules: env_limit("AETHEROS_MAX_MODULE_COUNT", 256),
            max_total_bytes: env_limit("AETHEROS_MAX_MODULE_TOTAL_BYTES", 512 * 1024 * 1024),
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
        if binary.len() > self.max_module_bytes {
            return Err(ModuleStoreError::SizeLimitExceeded);
        }

        let hash: ModuleHash = Sha256::digest(&binary).into();
        if self.modules.contains_key(&hash) {
            return Ok(hash);
        }
        if self.modules.len() >= self.max_modules {
            return Err(ModuleStoreError::StoreLimitExceeded);
        }

        let size = binary.len();
        loop {
            let current = self.total_bytes.load(Ordering::Acquire);
            let new_total = current.saturating_add(size);
            if new_total > self.max_total_bytes {
                return Err(ModuleStoreError::StoreLimitExceeded);
            }
            if self.total_bytes.compare_exchange(
                current, new_total, Ordering::AcqRel, Ordering::Acquire
            ).is_ok() {
                break;
            }
        }

        match self.modules.entry(hash) {
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(Arc::new(binary));
            }
            dashmap::mapref::entry::Entry::Occupied(_) => {
                self.total_bytes.fetch_sub(size, Ordering::AcqRel);
            }
        }

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
        self.total_bytes.store(0, Ordering::Release);
    }
}

fn env_limit(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}
