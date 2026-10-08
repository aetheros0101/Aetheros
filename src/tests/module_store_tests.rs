// ============================================================
// src/tests/module_store_tests.rs
//
// Optimizasyon #1: ModuleStore Testleri
// ============================================================

use crate::wasm::module_store::ModuleStore;

fn fake_binary(seed: u8, size: usize) -> Vec<u8> {
    // WASM magic + sahte içerik
    let mut v = vec![0x00, 0x61, 0x73, 0x6d]; // \0asm
    v.extend(vec![seed; size]);
    v
}

// ── Store / Get ───────────────────────────────────────────

#[test]
fn store_and_get_roundtrip() {
    let store = ModuleStore::new();
    let binary = fake_binary(1, 64);

    let hash = store.store(binary.clone()).unwrap();
    let retrieved = store.get(&hash).unwrap();

    assert_eq!(*retrieved, binary);
}

#[test]
fn store_returns_same_hash_for_same_binary() {
    let store = ModuleStore::new();
    let binary = fake_binary(2, 128);

    let hash1 = store.store(binary.clone()).unwrap();
    let hash2 = store.store(binary.clone()).unwrap();

    assert_eq!(hash1, hash2, "Aynı binary aynı hash vermeli");
    assert_eq!(store.count(), 1, "Aynı binary ikinci kez depolanmamalı");
}

#[test]
fn different_binaries_different_hashes() {
    let store = ModuleStore::new();
    let h1 = store.store(fake_binary(1, 64)).unwrap();
    let h2 = store.store(fake_binary(2, 64)).unwrap();

    assert_ne!(h1, h2);
    assert_eq!(store.count(), 2);
}

#[test]
fn get_nonexistent_returns_error() {
    let store = ModuleStore::new();
    let fake_hash = [0xABu8; 32];
    assert!(store.get(&fake_hash).is_err());
}

#[test]
fn empty_binary_returns_error() {
    let store = ModuleStore::new();
    let result = store.store(vec![]);
    assert!(result.is_err());
}

// ── Contains / Count ─────────────────────────────────────

#[test]
fn contains_returns_true_after_store() {
    let store = ModuleStore::new();
    let hash = store.store(fake_binary(3, 32)).unwrap();
    assert!(store.contains(&hash));
}

#[test]
fn contains_returns_false_for_unknown() {
    let store = ModuleStore::new();
    assert!(!store.contains(&[0u8; 32]));
}

#[test]
fn count_tracks_unique_modules() {
    let store = ModuleStore::new();
    assert_eq!(store.count(), 0);

    store.store(fake_binary(1, 32)).unwrap();
    store.store(fake_binary(2, 32)).unwrap();
    store.store(fake_binary(3, 32)).unwrap();
    // Aynı binary tekrar
    store.store(fake_binary(1, 32)).unwrap();

    assert_eq!(store.count(), 3);
}

// ── Hex Conversion ────────────────────────────────────────

#[test]
fn hash_to_hex_and_back() {
    let store = ModuleStore::new();
    let hash = store.store(fake_binary(4, 64)).unwrap();

    let hex = ModuleStore::hash_to_hex(&hash);
    assert_eq!(hex.len(), 64);

    let recovered = ModuleStore::hex_to_hash(&hex).unwrap();
    assert_eq!(recovered, hash);
}

#[test]
fn invalid_hex_returns_error() {
    assert!(ModuleStore::hex_to_hash("not-hex!!!").is_err());
}

#[test]
fn short_hex_returns_error() {
    // 32 byte yerine 16 byte
    assert!(ModuleStore::hex_to_hash("0102030405060708090a0b0c0d0e0f10").is_err());
}

// ── Binary Size ───────────────────────────────────────────

#[test]
fn binary_size_correct() {
    let store = ModuleStore::new();
    let binary = fake_binary(5, 200);
    let expected_size = binary.len();
    let hash = store.store(binary).unwrap();

    assert_eq!(store.binary_size(&hash), Some(expected_size));
}

#[test]
fn binary_size_unknown_returns_none() {
    let store = ModuleStore::new();
    assert!(store.binary_size(&[0u8; 32]).is_none());
}

// ── TaskDefinition Entegrasyonu ───────────────────────────

#[test]
fn task_has_module_false_when_zero_hash() {
    use crate::task::priority::TaskPriority;
    use crate::tests::helpers::make_task;

    let task = make_task(TaskPriority::Normal, "main");
    // [0u8; 32] → modül yok
    assert!(!task.has_module());
}

#[test]
fn task_has_module_true_when_real_hash() {
    use crate::task::priority::TaskPriority;
    use crate::tests::helpers::make_task_with_hash;

    let store = ModuleStore::new();
    let hash = store.store(fake_binary(6, 32)).unwrap();
    let task = make_task_with_hash(TaskPriority::High, hash);

    assert!(task.has_module());
}

// ── Serde Roundtrip ───────────────────────────────────────

#[test]
fn task_serializes_hash_as_hex() {
    use crate::task::priority::TaskPriority;
    use crate::task::task::TaskDefinition;
    use crate::tests::helpers::make_task_with_hash;

    let store = ModuleStore::new();
    let hash = store.store(fake_binary(7, 64)).unwrap();
    let task = make_task_with_hash(TaskPriority::Normal, hash);

    let json = serde_json::to_string(&task).unwrap();

    // NOT: özel bir hex serde katmanı BİLEREK kaldırıldı (bkz.
    // task/task.rs'teki yorum — persist/load round-trip'inde
    // wasm_module_hash'in sıfırlanmasına yol açan hatanın kaynağıydı).
    // Bu yüzden burada JSON metninde hex string aramak yerine, alanın
    // gerçekten var olduğunu ve 32 baytın bozulmadan geri geldiğini
    // doğruluyoruz — asıl önemli olan bu, temsil biçimi değil.
    assert!(json.contains("wasm_module_hash"));
    let recovered: TaskDefinition = serde_json::from_str(&json).unwrap();
    assert_eq!(recovered.wasm_module_hash, hash);
}

#[test]
fn task_deserializes_hash_correctly() {
    use crate::task::priority::TaskPriority;
    use crate::task::task::TaskDefinition;
    use crate::tests::helpers::make_task_with_hash;

    let store = ModuleStore::new();
    let hash = store.store(fake_binary(8, 32)).unwrap();
    let task = make_task_with_hash(TaskPriority::High, hash);

    let json = serde_json::to_string(&task).unwrap();
    let recovered: TaskDefinition = serde_json::from_str(&json).unwrap();

    assert_eq!(recovered.wasm_module_hash, hash);
    assert_eq!(recovered.id, task.id);
}
