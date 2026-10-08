// ============================================================
// src/tests/scripting_tests.rs
//
// SPRINT 3 — Script Engine Testleri
//
// ScriptDefinition, ScriptRegistry, ScriptResult
// unit test edilir. ScriptEngine testi gerçek WASM
// execution gerektirdiği için integration klasöründe
// ayrıca yer alır (wat binary ile).
// ============================================================

use crate::scripting::definition::ScriptDefinition;
use crate::scripting::registry::ScriptRegistry;
use crate::scripting::result::ScriptResult;

// ── ScriptDefinition Testleri ─────────────────────────────

#[test]
fn from_binary_creates_definition() {
    let binary = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic
    let script = ScriptDefinition::from_binary("test-script", binary.clone(), "main", 5000);

    assert_eq!(script.name, "test-script");
    assert_eq!(script.entrypoint, "main");
    assert_eq!(script.timeout_ms, 5000);
    assert_eq!(script.wasm_binary, binary);
    assert!(script.description.is_none());
}

#[test]
fn with_description_sets_field() {
    let script = ScriptDefinition::from_binary("my-script", vec![], "run", 1000)
        .with_description("Does something useful");

    assert_eq!(script.description.unwrap(), "Does something useful");
}

#[test]
fn from_hex_valid_hex_parses() {
    // WASM magic bytes: \0asm
    let script = ScriptDefinition::from_hex("hex-script", "0061736d", "main", 3000);
    assert!(script.is_ok());
    let s = script.unwrap();
    assert_eq!(s.wasm_binary, vec![0x00, 0x61, 0x73, 0x6d]);
}

#[test]
fn from_hex_invalid_returns_error() {
    let result = ScriptDefinition::from_hex("bad", "not-valid-hex!!!", "main", 1000);
    assert!(result.is_err());
}

// ── ScriptRegistry Testleri ───────────────────────────────

#[test]
fn registry_register_and_get() {
    let registry = ScriptRegistry::new();
    let script = ScriptDefinition::from_binary("my-script", vec![], "main", 1000);

    registry.register(script);

    let found = registry.get("my-script");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "my-script");
}

#[test]
fn registry_get_nonexistent_returns_none() {
    let registry = ScriptRegistry::new();
    assert!(registry.get("missing").is_none());
}

#[test]
fn registry_remove_deletes_entry() {
    let registry = ScriptRegistry::new();
    registry.register(ScriptDefinition::from_binary(
        "to-delete",
        vec![],
        "main",
        1000,
    ));

    assert!(registry.contains("to-delete"));
    registry.remove("to-delete");
    assert!(!registry.contains("to-delete"));
}

#[test]
fn registry_count_accurate() {
    let registry = ScriptRegistry::new();
    assert_eq!(registry.count(), 0);

    for i in 0..5 {
        registry.register(ScriptDefinition::from_binary(
            format!("script-{}", i),
            vec![],
            "main",
            1000,
        ));
    }
    assert_eq!(registry.count(), 5);
}

#[test]
fn registry_list_returns_all() {
    let registry = ScriptRegistry::new();
    registry.register(ScriptDefinition::from_binary("a", vec![], "main", 1000));
    registry.register(ScriptDefinition::from_binary("b", vec![], "main", 1000));

    let list = registry.list();
    assert_eq!(list.len(), 2);
}

#[test]
fn registry_overwrite_same_name() {
    let registry = ScriptRegistry::new();

    registry.register(ScriptDefinition::from_binary(
        "script",
        vec![1, 2, 3],
        "old",
        1000,
    ));
    registry.register(ScriptDefinition::from_binary(
        "script",
        vec![4, 5, 6],
        "new",
        2000,
    ));

    // Son kayıt kazanır
    let found = registry.get("script").unwrap();
    assert_eq!(found.entrypoint, "new");
    assert_eq!(found.timeout_ms, 2000);
    assert_eq!(registry.count(), 1); // hâlâ 1 kayıt
}

// ── ScriptResult Testleri ─────────────────────────────────

#[test]
fn result_output_as_string() {
    let result = ScriptResult {
        script_name: "test".to_string(),
        output: b"hello world".to_vec(),
        elapsed_ms: 42,
        executed_at: chrono::Utc::now(),
    };

    assert_eq!(result.output_as_string(), "hello world");
}

#[test]
fn result_empty_output() {
    let result = ScriptResult {
        script_name: "empty".to_string(),
        output: vec![],
        elapsed_ms: 1,
        executed_at: chrono::Utc::now(),
    };

    assert!(result.is_empty());
    assert_eq!(result.output_as_string(), "");
}

#[test]
fn result_invalid_utf8_handled_lossy() {
    let result = ScriptResult {
        script_name: "binary".to_string(),
        output: vec![0xFF, 0xFE, 0x41], // geçersiz UTF-8
        elapsed_ms: 5,
        executed_at: chrono::Utc::now(),
    };

    // Panic etmemeli — lossy dönüşüm
    let s = result.output_as_string();
    assert!(s.contains('A')); // 0x41 = 'A'
}
