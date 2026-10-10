//! Mimari kuralları (katman bağımlılıkları).
//!
//! Üretim kodunun üst düzey modülleri (`src/<modül>/`) arasındaki `crate::…`
//! bağımlılık grafiğini kaynak koddan çıkarır ve şunları denetler:
//!
//! 1. Graf **döngüsüzdür** (eskiden `types ⇄ agents`, `agents ⇄ security`,
//!    `task ⇄ wasm` döngüleri vardı).
//! 2. Temel katmanlar (`types`, `errors`) başka hiçbir iç modüle bağlanmaz.
//! 3. `bridge` hiçbir modül tarafından import edilmez; `api` yalnızca
//!    `bridge` tarafından kullanılabilir (adaptörler en üstte durur).
//!
//! Test kodu (`#[cfg(test)]`) ve yorum satırları sayılmaz.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

type Graph = BTreeMap<String, BTreeSet<String>>;

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Yorum satırlarını ve `#[cfg(test)]` sonrasını atar.
fn production_text(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            break;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn read_ident(chars: &[char], mut i: usize) -> (String, usize) {
    let start = i;
    while i < chars.len() && is_ident(chars[i]) {
        i += 1;
    }
    (chars[start..i].iter().collect(), i)
}

/// `crate::x` ve `crate::{x, y::z}` biçimlerinden ilk segmentleri çıkarır.
fn crate_roots(text: &str) -> BTreeSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let pat: Vec<char> = "crate::".chars().collect();
    let mut roots = BTreeSet::new();
    let mut i = 0;
    while i + pat.len() <= chars.len() {
        // `my_crate::` gibi önekleri eleme: öncesi tanımlayıcı karakter olmamalı.
        let boundary = i == 0 || !is_ident(chars[i - 1]);
        if boundary && chars[i..i + pat.len()] == pat[..] {
            let mut j = i + pat.len();
            if j < chars.len() && chars[j] == '{' {
                // Gruplu import: derinlik 1'deki öğelerin ilk tanımlayıcısı.
                let mut depth = 1;
                let mut item_start = true;
                j += 1;
                while j < chars.len() && depth > 0 {
                    let c = chars[j];
                    if c == '{' {
                        depth += 1;
                        item_start = false;
                    } else if c == '}' {
                        depth -= 1;
                    } else if c == ',' {
                        item_start = depth == 1;
                    } else if c.is_whitespace() {
                        // boşlukları atla
                    } else if item_start && depth == 1 && is_ident(c) {
                        let (id, next) = read_ident(&chars, j);
                        roots.insert(id);
                        item_start = false;
                        j = next;
                        continue;
                    } else {
                        item_start = false;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
            let (id, next) = read_ident(&chars, j);
            if !id.is_empty() {
                roots.insert(id);
            }
            i = next;
            continue;
        }
        i += 1;
    }
    roots
}

fn dependency_graph() -> Graph {
    let src = src_dir();
    let modules: BTreeSet<String> = fs::read_dir(&src)
        .expect("src okunabilmeli")
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name != "tests")
        .collect();

    let mut graph: Graph = modules
        .iter()
        .map(|m| (m.clone(), BTreeSet::new()))
        .collect();

    for module in &modules {
        let mut files = Vec::new();
        rust_files(&src.join(module), &mut files);
        for file in files {
            let text = fs::read_to_string(&file).expect("kaynak okunabilmeli");
            for root in crate_roots(&production_text(&text)) {
                if &root != module && modules.contains(&root) {
                    graph.get_mut(module).unwrap().insert(root);
                }
            }
        }
    }
    graph
}

/// İlk bulunan döngüyü (varsa) yol olarak döndürür.
fn find_cycle(graph: &Graph) -> Option<Vec<String>> {
    fn visit(
        node: &str,
        graph: &Graph,
        state: &mut BTreeMap<String, u8>, // 1 = yolda, 2 = bitti
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        state.insert(node.to_string(), 1);
        path.push(node.to_string());
        if let Some(deps) = graph.get(node) {
            for dep in deps {
                match state.get(dep.as_str()) {
                    Some(1) => {
                        let start = path.iter().position(|n| n == dep).unwrap_or(0);
                        let mut cycle = path[start..].to_vec();
                        cycle.push(dep.clone());
                        return Some(cycle);
                    }
                    Some(_) => {}
                    None => {
                        if let Some(c) = visit(dep, graph, state, path) {
                            return Some(c);
                        }
                    }
                }
            }
        }
        path.pop();
        state.insert(node.to_string(), 2);
        None
    }

    let mut state = BTreeMap::new();
    for node in graph.keys() {
        if !state.contains_key(node.as_str()) {
            let mut path = Vec::new();
            if let Some(c) = visit(node, graph, &mut state, &mut path) {
                return Some(c);
            }
        }
    }
    None
}

#[test]
fn module_graph_is_acyclic() {
    let graph = dependency_graph();
    if let Some(cycle) = find_cycle(&graph) {
        panic!(
            "Modüller arası döngüsel bağımlılık: {}\n\
             Ortak tipi daha alt bir katmana (örn. `types`) taşıyın.",
            cycle.join(" → ")
        );
    }
}

#[test]
fn foundation_modules_have_no_internal_dependencies() {
    let graph = dependency_graph();
    for foundation in ["types", "errors"] {
        let deps = &graph[foundation];
        assert!(
            deps.is_empty(),
            "`{foundation}` temel katmandır ama şunlara bağlanıyor: {deps:?}"
        );
    }
}

#[test]
fn adapters_sit_at_the_top() {
    let graph = dependency_graph();
    for (module, deps) in &graph {
        assert!(
            !deps.contains("bridge"),
            "`{module}` modülü `bridge`'i import ediyor; bridge en üst adaptör katmanıdır"
        );
        if module != "bridge" {
            assert!(
                !deps.contains("api"),
                "`{module}` modülü `api`'yi import ediyor; yalnızca `bridge` kullanabilir"
            );
        }
    }
}

#[test]
fn crate_roots_parses_plain_and_grouped_imports() {
    let text = "use crate::types::ids::TaskId;\n\
                use crate::{errors::E, task::{a, b}, wasm};\n\
                use crate::{\n    persistence::x,\n    events,\n};\n\
                let _ = my_crate::nope::X;";
    let roots = crate_roots(text);
    let expected: BTreeSet<String> = ["types", "errors", "task", "wasm", "persistence", "events"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(roots, expected);
}

#[test]
fn find_cycle_detects_a_synthetic_cycle() {
    let mut g: Graph = BTreeMap::new();
    g.insert("a".into(), ["b".to_string()].into_iter().collect());
    g.insert("b".into(), ["c".to_string()].into_iter().collect());
    g.insert("c".into(), ["a".to_string()].into_iter().collect());
    let cycle = find_cycle(&g).expect("döngü bulunmalı");
    assert_eq!(cycle.first(), cycle.last());
    g.insert("c".into(), BTreeSet::new());
    assert!(find_cycle(&g).is_none());
}
