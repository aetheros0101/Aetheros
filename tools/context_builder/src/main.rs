// main.rs — Faz 1 + Faz 2 + Faz 3 + Faz 4 + Faz 5 + Faz 6 (Flutter/Dart)

use anyhow::Result;
use regex::Regex;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

use syn::{
    visit::{self, Visit},
    Attribute, Expr, ExprLit, File as SynFile, ImplItem, Lit, Meta, ReturnType, Type,
};
use quote::ToTokens;

// ── Faz 1 tipleri ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
struct ModuleInfo { path: String, files: usize }

// ── Faz 2 tipleri ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct BundleMeta { files: usize, bytes: usize, includes: Vec<String> }

// ── Faz 3 tipleri ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct Architecture {
    layers:  BTreeMap<String, usize>,
    modules: BTreeMap<String, ModuleStats>,
    cycles:  Vec<Vec<String>>,
    summary: Vec<ModuleSummary>,
}

#[derive(Debug, Serialize, Clone)]
struct ModuleStats {
    depends_on:  Vec<String>,
    depended_by: Vec<String>,
    transitive:  usize,
    loc:         usize,
    files:       usize,
}

#[derive(Debug, Serialize, Clone)]
struct ModuleSummary {
    name:    String,
    layer:   usize,
    loc:     usize,
    fan_in:  usize,
    fan_out: usize,
}

// ── Faz 4 tipleri (Rust AST) ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AstIndex {
    files:           BTreeMap<String, AstFileSummary>,
    total_pub_items: usize,
}

#[derive(Debug, Serialize)]
struct AstFileSummary {
    module_doc:  Option<String>,
    pub_fns:     Vec<FnSignature>,
    pub_types:   Vec<TypeDef>,
    impls:       Vec<ImplBlock>,
    crate_uses:  Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct FnSignature {
    name:      String,
    is_async:  bool,
    params:    Vec<String>,
    ret:       Option<String>,
    is_method: bool,
    doc:       Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct TypeDef {
    kind:     String,
    name:     String,
    generics: Vec<String>,
    doc:      Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct ImplBlock {
    self_ty:   String,
    trait_for: Option<String>,
    methods:   Vec<FnSignature>,
}

// ── Faz 5 tipleri (dile bağımsız semantik özet) ──────────────────────────────

#[derive(Debug, Serialize)]
struct SemanticIndex {
    modules: BTreeMap<String, SemanticModule>,
}

#[derive(Debug, Serialize)]
struct SemanticModule {
    purpose:     Option<String>,
    layer:       usize,
    loc:         usize,
    depends_on:  Vec<String>,
    depended_by: Vec<String>,
    api:         Vec<SemanticItem>,
}

#[derive(Debug, Serialize, Clone)]
struct SemanticItem {
    kind:      String,
    signature: String,
    doc:       Option<String>,
}

// ── Faz 6 tipleri (Dart/Flutter) ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct DartIndex {
    files:       BTreeMap<String, DartFileSummary>,
    total_types: usize,
}

#[derive(Debug, Serialize)]
struct DartFileSummary {
    module_doc: Option<String>,
    types:      Vec<DartTypeDef>,
}

#[derive(Debug, Serialize, Clone)]
struct DartTypeDef {
    kind:      String,   // "class" | "mixin" | "enum" | "extension"
    name:      String,   // anonim extension için "(anonim)"
    signature: String,
    doc:       Option<String>,
    is_public: bool,
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);

    let ai_dir = root.join(".ai");
    fs::create_dir_all(&ai_dir)?;

    let src_root = root.join("src");

    // ── Faz 1 (Rust) ─────────────────────────────────────────────────────────
    let modules = build_modules_ext(&src_root, "rs")?;
    let symbols = build_symbols_regex(&src_root, &root)?;
    let deps    = build_dependencies(&root, &src_root)?;

    write_json(ai_dir.join("modules.json"),      &modules)?;
    write_json(ai_dir.join("symbols.json"),      &symbols)?;
    write_json(ai_dir.join("dependencies.json"), &deps)?;
    println!("✓ Faz 1: modules / symbols / dependencies (Rust)");

    // ── Faz 2 (Rust bundles) ────────────────────────────────────────────────
    let bundles_dir = ai_dir.join("bundles");
    fs::create_dir_all(&bundles_dir)?;
    let bundle_manifest = build_bundles(&root, &src_root, &deps, &bundles_dir, "rs")?;
    write_json(ai_dir.join("bundles.json"), &bundle_manifest)?;
    println!("✓ Faz 2: bundles ({} adet)", bundle_manifest.len());

    // ── Faz 3 (Rust mimari) ─────────────────────────────────────────────────
    let arch = build_architecture(&src_root, &deps, "rs")?;
    write_json(ai_dir.join("architecture.json"), &arch)?;
    println!(
        "✓ Faz 3: architecture ({} modül, {} döngü)",
        arch.modules.len(), arch.cycles.len()
    );

    // ── Faz 4 (Rust AST) ─────────────────────────────────────────────────────
    let ast_index = build_ast_index(&src_root, &root)?;
    write_json(ai_dir.join("ast.json"), &ast_index)?;
    println!(
        "✓ Faz 4: AST ({} dosya, {} pub item)",
        ast_index.files.len(), ast_index.total_pub_items
    );

    // ── Faz 5 (Rust semantik) ───────────────────────────────────────────────
    let semantic = build_semantic_index(&root, &src_root, &ast_index, &arch)?;
    write_json(ai_dir.join("semantic.json"), &semantic)?;
    println!("✓ Faz 5: semantic bundle (Rust, {} modül)", semantic.modules.len());

    // ── Faz 6 (Flutter / Dart) ──────────────────────────────────────────────
    let flutter_root = root.join("flutter_app");
    let lib_root      = flutter_root.join("lib");

    let mut dart_section = String::new();

    if lib_root.exists() {
        let package_name = read_pubspec_package_name(&flutter_root)
            .unwrap_or_else(|| "app".to_string());

        let dart_modules = build_modules_ext(&lib_root, "dart")?;
        let dart_index   = build_dart_index(&lib_root, &root)?;
        let dart_symbols = dart_symbols_from_index(&dart_index);
        let dart_deps    = build_dart_dependencies(&lib_root, &package_name)?;

        let mut dart_arch = build_architecture(&lib_root, &dart_deps, "dart")?;
        patch_dart_root_module(&mut dart_arch, &lib_root);

        let dart_semantic = build_dart_semantic_index(&lib_root, &root, &dart_index, &dart_arch)?;

        write_json(ai_dir.join("dart_modules.json"),      &dart_modules)?;
        write_json(ai_dir.join("dart_symbols.json"),      &dart_symbols)?;
        write_json(ai_dir.join("dart_dependencies.json"), &dart_deps)?;
        write_json(ai_dir.join("dart_architecture.json"), &dart_arch)?;
        write_json(ai_dir.join("dart_ast.json"),          &dart_index)?;
        write_json(ai_dir.join("dart_semantic.json"),     &dart_semantic)?;

        dart_section = render_project_context(
            &dart_semantic, &dart_arch, "Flutter Önyüz (flutter_app/lib/)"
        );

        println!(
            "✓ Faz 6: Flutter/Dart ({} modül, {} dosya, {} tip, paket: {package_name})",
            dart_arch.modules.len(), dart_index.files.len(), dart_index.total_types
        );
    } else {
        println!("ℹ Faz 6: flutter_app/lib bulunamadı, Dart taraması atlandı");
    }

    // ── PROJECT_CONTEXT.md (birleşik) ───────────────────────────────────────
    let rust_section = render_project_context(&semantic, &arch, "Rust Backend (src/)");

    let full_context = if dart_section.is_empty() {
        rust_section
    } else {
        format!("{rust_section}\n---\n\n{dart_section}")
    };

    fs::write(ai_dir.join("PROJECT_CONTEXT.md"), &full_context)?;
    println!("✓ PROJECT_CONTEXT.md yazıldı ({} bayt)", full_context.len());

    Ok(())
}

// ── Faz 5: Semantic Bundle (Rust) ────────────────────────────────────────────

fn build_semantic_index(
    root:      &Path,
    base_dir:  &Path,
    ast_index: &AstIndex,
    arch:      &Architecture,
) -> Result<SemanticIndex> {
    let mut modules: BTreeMap<String, SemanticModule> = BTreeMap::new();

    for (mod_name, stats) in &arch.modules {
        let mod_dir = base_dir.join(mod_name);
        if !mod_dir.exists() {
            continue;
        }

        let mut purpose: Option<String> = None;
        let mut api: Vec<SemanticItem> = Vec::new();

        for file in rs_files_in(&mod_dir) {
            let rel = relative(root, &file);
            let Some(summary) = ast_index.files.get(&rel) else { continue };

            if purpose.is_none() {
                if let Some(doc) = &summary.module_doc {
                    purpose = Some(doc.clone());
                }
            }

            for f in &summary.pub_fns {
                api.push(SemanticItem {
                    kind:      "fn".into(),
                    signature: render_fn_signature(f),
                    doc:       f.doc.clone(),
                });
            }

            for t in &summary.pub_types {
                api.push(SemanticItem {
                    kind:      t.kind.clone(),
                    signature: render_type_signature(t),
                    doc:       t.doc.clone(),
                });
            }

            for imp in &summary.impls {
                for m in &imp.methods {
                    api.push(SemanticItem {
                        kind:      "method".into(),
                        signature: render_method_signature(imp, m),
                        doc:       m.doc.clone(),
                    });
                }
            }
        }

        modules.insert(mod_name.clone(), SemanticModule {
            purpose,
            layer:       *arch.layers.get(mod_name).unwrap_or(&0),
            loc:         stats.loc,
            depends_on:  stats.depends_on.clone(),
            depended_by: stats.depended_by.clone(),
            api,
        });
    }

    Ok(SemanticIndex { modules })
}

fn render_fn_signature(f: &FnSignature) -> String {
    let asy = if f.is_async { "async " } else { "" };
    let params = f.params.join(", ");
    let ret = f.ret.as_ref().map(|r| format!(" -> {r}")).unwrap_or_default();
    format!("pub {asy}fn {}({params}){ret}", f.name)
}

fn render_method_signature(imp: &ImplBlock, m: &FnSignature) -> String {
    let asy = if m.is_async { "async " } else { "" };
    let params = m.params.join(", ");
    let ret = m.ret.as_ref().map(|r| format!(" -> {r}")).unwrap_or_default();
    let owner = match &imp.trait_for {
        Some(tr) => format!("{} for {}", tr, imp.self_ty),
        None     => imp.self_ty.clone(),
    };
    format!("impl {owner} :: {asy}fn {}({params}){ret}", m.name)
}

fn render_type_signature(t: &TypeDef) -> String {
    if t.generics.is_empty() {
        format!("pub {} {}", t.kind, t.name)
    } else {
        format!("pub {} {}<{}>", t.kind, t.name, t.generics.join(", "))
    }
}

/// AI'ye doğrudan yapıştırılacak markdown bölümü.
/// Tam kaynak yok — sadece mimari + doc + imza. `title` Rust/Dart ayrımı için.
fn render_project_context(semantic: &SemanticIndex, arch: &Architecture, title: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {title}\n\n"));
    out.push_str("> Bu dosya implementasyon gövdesi içermez. Sadece mimari, ");
    out.push_str("modül amacı (doc comment) ve public API imzaları yer alır.\n\n");

    out.push_str("## Mimari\n\n");
    out.push_str("| Katman | Modül | LOC | Fan-in | Fan-out |\n");
    out.push_str("|---|---|---|---|---|\n");
    for s in &arch.summary {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            s.layer, s.name, s.loc, s.fan_in, s.fan_out
        ));
    }

    if !arch.cycles.is_empty() {
        out.push_str("\n**⚠ Döngüsel bağımlılıklar tespit edildi:**\n\n");
        for c in &arch.cycles {
            out.push_str(&format!("- {}\n", c.join(" → ")));
        }
    }

    out.push_str("\n## Modüller\n\n");

    for (name, m) in &semantic.modules {
        out.push_str(&format!("### `{name}` (katman {}, {} LOC)\n\n", m.layer, m.loc));

        if let Some(p) = &m.purpose {
            out.push_str(&format!("**Amaç:** {p}\n\n"));
        }

        if !m.depends_on.is_empty() {
            out.push_str(&format!("**Bağımlı olduğu:** {}\n\n", m.depends_on.join(", ")));
        }
        if !m.depended_by.is_empty() {
            out.push_str(&format!("**Kendisine bağımlı olanlar:** {}\n\n", m.depended_by.join(", ")));
        }

        if m.api.is_empty() {
            out.push_str("_Public API yok._\n\n");
            continue;
        }

        out.push_str("**Public API:**\n\n");
        for item in &m.api {
            match &item.doc {
                Some(d) => out.push_str(&format!("- `{}` — {}\n", item.signature, d)),
                None    => out.push_str(&format!("- `{}`\n", item.signature)),
            }
        }
        out.push('\n');
    }

    out
}

// ── Faz 4: AST Analizi (Rust) ─────────────────────────────────────────────────

fn build_ast_index(scan_dir: &Path, root: &Path) -> Result<AstIndex> {
    let mut files_map: BTreeMap<String, AstFileSummary> = BTreeMap::new();

    for file_path in rust_files(scan_dir) {
        let content = fs::read_to_string(&file_path).unwrap_or_default();

        let summary = match syn::parse_file(&content) {
            Ok(ast) => analyse_file(&ast),
            Err(e) => {
                eprintln!("⚠ parse hatası {}: {e}", file_path.display());
                continue;
            }
        };

        files_map.insert(relative(root, &file_path), summary);
    }

    let total_pub_items = files_map.values()
        .map(|s| s.pub_fns.len() + s.pub_types.len())
        .sum();

    Ok(AstIndex { files: files_map, total_pub_items })
}

fn analyse_file(ast: &SynFile) -> AstFileSummary {
    let module_doc = extract_doc(&ast.attrs);

    let mut visitor = FileVisitor::default();
    visitor.visit_file(ast);

    AstFileSummary {
        module_doc,
        pub_fns:    visitor.pub_fns,
        pub_types:  visitor.pub_types,
        impls:      visitor.impls,
        crate_uses: visitor.crate_uses,
    }
}

#[derive(Default)]
struct FileVisitor {
    pub_fns:    Vec<FnSignature>,
    pub_types:  Vec<TypeDef>,
    impls:      Vec<ImplBlock>,
    crate_uses: Vec<String>,
}

impl<'ast> Visit<'ast> for FileVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if is_pub(&node.vis) {
            self.pub_fns.push(fn_sig_from_item(node));
        }
        visit::visit_item_fn(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if is_pub(&node.vis) {
            self.pub_types.push(TypeDef {
                kind:     "struct".into(),
                name:     node.ident.to_string(),
                generics: generic_params(&node.generics),
                doc:      extract_doc(&node.attrs),
            });
        }
        visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if is_pub(&node.vis) {
            self.pub_types.push(TypeDef {
                kind:     "enum".into(),
                name:     node.ident.to_string(),
                generics: generic_params(&node.generics),
                doc:      extract_doc(&node.attrs),
            });
        }
        visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if is_pub(&node.vis) {
            self.pub_types.push(TypeDef {
                kind:     "trait".into(),
                name:     node.ident.to_string(),
                generics: generic_params(&node.generics),
                doc:      extract_doc(&node.attrs),
            });
        }
        visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let self_ty   = type_to_string(&node.self_ty);
        let trait_for = node.trait_.as_ref().map(|(_, path, _)| path.to_token_stream().to_string());

        let methods: Vec<FnSignature> = node.items.iter().filter_map(|item| {
            if let ImplItem::Fn(m) = item {
                if is_pub(&m.vis) {
                    Some(fn_sig_from_method(m))
                } else {
                    None
                }
            } else {
                None
            }
        }).collect();

        self.impls.push(ImplBlock { self_ty, trait_for, methods });
        visit::visit_item_impl(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let s = node.to_token_stream().to_string();
        if s.contains("crate ::") || s.contains("crate::") {
            let clean = s.replace("crate ::", "crate::").replace(" :: ", "::").trim_end_matches(';').to_string();
            self.crate_uses.push(clean);
        }
        visit::visit_item_use(self, node);
    }
}

// ── Doc comment çıkarma (Rust) ────────────────────────────────────────────────

fn extract_doc(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs.iter().filter_map(|attr| {
        if !attr.path().is_ident("doc") { return None; }
        if let Meta::NameValue(nv) = &attr.meta {
            if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                let trimmed = s.value().trim().to_string();
                if !trimmed.is_empty() { return Some(trimmed); }
            }
        }
        None
    }).collect();

    if lines.is_empty() { None } else { Some(lines.join(" ")) }
}

// ── AST yardımcıları (Rust) ──────────────────────────────────────────────────

fn is_pub(vis: &syn::Visibility) -> bool { matches!(vis, syn::Visibility::Public(_)) }

fn generic_params(generics: &syn::Generics) -> Vec<String> {
    generics.params.iter().map(|p| p.to_token_stream().to_string()).collect()
}

fn type_to_string(ty: &Type) -> String {
    ty.to_token_stream().to_string().replace(" :: ", "::").replace("< ", "<").replace(" >", ">")
}

fn ret_to_string(ret: &ReturnType) -> Option<String> {
    match ret {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(type_to_string(ty)),
    }
}

fn fn_sig_from_item(node: &syn::ItemFn) -> FnSignature {
    FnSignature {
        name:      node.sig.ident.to_string(),
        is_async:  node.sig.asyncness.is_some(),
        params:    node.sig.inputs.iter().map(|a| a.to_token_stream().to_string()).collect(),
        ret:       ret_to_string(&node.sig.output),
        is_method: false,
        doc:       extract_doc(&node.attrs),
    }
}

fn fn_sig_from_method(node: &syn::ImplItemFn) -> FnSignature {
    FnSignature {
        name:      node.sig.ident.to_string(),
        is_async:  node.sig.asyncness.is_some(),
        params:    node.sig.inputs.iter().map(|a| a.to_token_stream().to_string()).collect(),
        ret:       ret_to_string(&node.sig.output),
        is_method: true,
        doc:       extract_doc(&node.attrs),
    }
}

// ── Faz 3: Architecture (dil-bağımsız) ───────────────────────────────────────

fn build_architecture(base_dir: &Path, deps: &BTreeMap<String, Vec<String>>, ext: &str) -> Result<Architecture> {
    let all_modules = collect_all_modules(base_dir, deps);
    let reverse     = build_reverse(&all_modules, deps);
    let layers      = assign_layers(&all_modules, deps);
    let cycles      = find_cycles(&all_modules, deps);
    let loc_map     = build_loc_map(base_dir, &all_modules, ext);
    let file_map    = build_file_map(base_dir, &all_modules, ext);

    let mut module_stats: BTreeMap<String, ModuleStats> = BTreeMap::new();
    for m in &all_modules {
        let depends_on:  Vec<String> = deps.get(m).cloned().unwrap_or_default();
        let depended_by: Vec<String> = reverse.get(m).map(|s| s.iter().cloned().collect()).unwrap_or_default();
        let transitive = transitive_deps(m, deps).len().saturating_sub(1);
        module_stats.insert(m.clone(), ModuleStats {
            depends_on, depended_by, transitive,
            loc:   *loc_map.get(m).unwrap_or(&0),
            files: *file_map.get(m).unwrap_or(&0),
        });
    }

    let mut summary: Vec<ModuleSummary> = module_stats.iter().map(|(name, s)| ModuleSummary {
        name:    name.clone(),
        layer:   *layers.get(name).unwrap_or(&0),
        loc:     s.loc,
        fan_in:  s.depended_by.len(),
        fan_out: s.depends_on.len(),
    }).collect();
    summary.sort_by(|a, b| b.fan_in.cmp(&a.fan_in).then(a.name.cmp(&b.name)));

    Ok(Architecture { layers, modules: module_stats, cycles, summary })
}

fn collect_all_modules(base_dir: &Path, deps: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    if base_dir.exists() {
        if let Ok(rd) = fs::read_dir(base_dir) {
            for e in rd.filter_map(|x| x.ok()) {
                if e.path().is_dir() {
                    if let Some(n) = e.path().file_name().and_then(|s| s.to_str()) {
                        if !is_excluded_dir(n) {
                            set.insert(n.to_string());
                        }
                    }
                }
            }
        }
    }
    for (k, vs) in deps { set.insert(k.clone()); set.extend(vs.iter().cloned()); }
    set
}

fn build_reverse(all: &BTreeSet<String>, deps: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, BTreeSet<String>> {
    let mut rev: BTreeMap<String, BTreeSet<String>> = all.iter().map(|m| (m.clone(), BTreeSet::new())).collect();
    for (m, children) in deps {
        for c in children { rev.entry(c.clone()).or_default().insert(m.clone()); }
    }
    rev
}

fn assign_layers(all: &BTreeSet<String>, deps: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, usize> {
    let mut in_degree: BTreeMap<String, usize> = all.iter().map(|m| (m.clone(), 0)).collect();
    for children in deps.values() {
        for c in children { *in_degree.entry(c.clone()).or_default() += 1; }
    }
    let mut layer: BTreeMap<String, usize> = BTreeMap::new();
    let mut queue: Vec<String> = in_degree.iter().filter(|(_, &d)| d == 0).map(|(m, _)| m.clone()).collect();
    for m in &queue { layer.insert(m.clone(), 0); }

    while !queue.is_empty() {
        let cur = queue.clone(); queue.clear();
        for m in &cur {
            if let Some(children) = deps.get(m) {
                for c in children {
                    let nl = layer.get(m).copied().unwrap_or(0) + 1;
                    let e  = layer.entry(c.clone()).or_insert(0);
                    if nl > *e { *e = nl; }
                    let deg = in_degree.entry(c.clone()).or_default();
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 { queue.push(c.clone()); }
                }
            }
        }
    }
    for m in all { layer.entry(m.clone()).or_insert(0); }
    layer
}

fn find_cycles(all: &BTreeSet<String>, deps: &BTreeMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut cycles  = Vec::new();
    let mut visited = BTreeSet::new();
    for start in all {
        let mut path     = vec![start.clone()];
        let mut on_stack = BTreeSet::from([start.clone()]);
        dfs_cycles(start, deps, &mut path, &mut on_stack, &mut visited, &mut cycles);
    }
    cycles.sort(); cycles.dedup(); cycles
}

fn dfs_cycles(
    node: &str, deps: &BTreeMap<String, Vec<String>>,
    path: &mut Vec<String>, on_stack: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>, cycles: &mut Vec<Vec<String>>,
) {
    if let Some(children) = deps.get(node) {
        for c in children {
            if on_stack.contains(c) {
                if let Some(idx) = path.iter().position(|x| x == c) {
                    let mut cycle = path[idx..].to_vec();
                    cycle.push(c.clone());
                    if let Some(mp) = cycle.iter().enumerate().min_by_key(|(_, v)| v.as_str()).map(|(i, _)| i) {
                        cycle.rotate_left(mp);
                    }
                    cycles.push(cycle);
                }
            } else if !visited.contains(c) {
                path.push(c.clone()); on_stack.insert(c.clone());
                dfs_cycles(c, deps, path, on_stack, visited, cycles);
                path.pop(); on_stack.remove(c);
            }
        }
    }
    visited.insert(node.to_string());
}

fn build_loc_map(base_dir: &Path, all: &BTreeSet<String>, ext: &str) -> BTreeMap<String, usize> {
    all.iter().map(|m| {
        let dir = base_dir.join(m);
        let loc = if dir.exists() {
            files_with_ext(&dir, ext).into_iter()
                .map(|f| fs::read_to_string(&f).unwrap_or_default().lines().count())
                .sum()
        } else { 0 };
        (m.clone(), loc)
    }).collect()
}

fn build_file_map(base_dir: &Path, all: &BTreeSet<String>, ext: &str) -> BTreeMap<String, usize> {
    all.iter().map(|m| {
        let dir = base_dir.join(m);
        (m.clone(), if dir.exists() { files_with_ext(&dir, ext).len() } else { 0 })
    }).collect()
}

// ── Faz 2: Bundle Generator (dil-bağımsız) ───────────────────────────────────

fn build_bundles(
    root: &Path, base_dir: &Path, deps: &BTreeMap<String, Vec<String>>,
    bundles_dir: &Path, ext: &str,
) -> Result<BTreeMap<String, BundleMeta>> {
    let mut manifest = BTreeMap::new();
    let all_modules: BTreeSet<String> = {
        if base_dir.exists() {
            fs::read_dir(base_dir)?.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.path().file_name().and_then(|s| s.to_str()).map(str::to_string))
                .filter(|n| !is_excluded_dir(n))
                .collect()
        } else { BTreeSet::new() }
    };

    for module in &all_modules {
        let includes = transitive_deps(module, deps);
        let mut sources: Vec<(String, String)> = Vec::new();
        for m in &includes {
            let mod_path = base_dir.join(m);
            if !mod_path.exists() { continue; }
            for file in files_with_ext(&mod_path, ext) {
                sources.push((relative(root, &file), fs::read_to_string(&file).unwrap_or_default()));
            }
        }
        let bundle = build_bundle_text(module, &includes, &sources);
        let bytes  = bundle.len();
        let files  = sources.len();
        fs::write(bundles_dir.join(format!("{module}.txt")), &bundle)?;
        manifest.insert(module.clone(), BundleMeta { files, bytes, includes: includes.into_iter().collect() });
    }
    Ok(manifest)
}

fn transitive_deps(start: &str, deps: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    let mut visited = BTreeSet::new();
    let mut stack   = vec![start.to_string()];
    while let Some(m) = stack.pop() {
        if !visited.insert(m.clone()) { continue; }
        if let Some(children) = deps.get(&m) { stack.extend(children.iter().cloned()); }
    }
    visited
}

fn build_bundle_text(module: &str, includes: &BTreeSet<String>, sources: &[(String, String)]) -> String {
    let mut out = format!("# BUNDLE: {module}\n# Includes: {}\n\n", includes.iter().cloned().collect::<Vec<_>>().join(", "));
    for (path, content) in sources {
        out.push_str(&format!("// ── {path} ──\n"));
        out.push_str(content);
        if !content.ends_with('\n') { out.push('\n'); }
        out.push('\n');
    }
    out
}

// ── Faz 1: ortak yardımcılar ──────────────────────────────────────────────────

fn write_json<T: Serialize>(path: PathBuf, data: &T) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(data)?)?;
    Ok(())
}

fn build_modules_ext(base_dir: &Path, ext: &str) -> Result<BTreeMap<String, ModuleInfo>> {
    let mut map = BTreeMap::new();
    if !base_dir.exists() { return Ok(map); }
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?; let path = entry.path();
        if !path.is_dir() { continue; }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else { continue; };
        if is_excluded_dir(name) { continue; }
        map.insert(name.to_string(), ModuleInfo { path: path.display().to_string(), files: files_with_ext(&path, ext).len() });
    }
    Ok(map)
}

fn build_symbols_regex(scan_dir: &Path, root: &Path) -> Result<BTreeMap<String, String>> {
    let mut symbols = BTreeMap::new();
    let re = Regex::new(r"pub\s+(struct|enum|trait)\s+([A-Za-z0-9_]+)")?;
    for file in rust_files(scan_dir) {
        let content = fs::read_to_string(&file).unwrap_or_default();
        for cap in re.captures_iter(&content) {
            symbols.insert(cap[2].to_string(), relative(root, &file));
        }
    }
    Ok(symbols)
}

fn build_dependencies(root: &Path, scan_dir: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let re = Regex::new(r"use\s+crate::([a-zA-Z0-9_]+)")?;
    for file in rust_files(scan_dir) {
        let content = fs::read_to_string(&file).unwrap_or_default();
        let Some(module) = top_module(root, &file) else { continue; };
        for cap in re.captures_iter(&content) {
            let dep = cap[1].to_string();
            if dep != module { graph.entry(module.clone()).or_default().insert(dep); }
        }
    }
    Ok(graph.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect())
}

/// Bir klasör adı tarama dışı tutulmalı mı? (derleme çıktıları, vcs, bağımlılıklar)
fn is_excluded_dir(name: &str) -> bool {
    matches!(name, "target" | "node_modules" | "build" | "dist") || name.starts_with('.')
}

fn files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 { return true; }
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !is_excluded_dir(name);
                }
            }
            true
        })
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some(ext))
        .map(|e| e.into_path())
        .collect()
}

fn rust_files(dir: &Path) -> Vec<PathBuf> { files_with_ext(dir, "rs") }
fn rs_files_in(dir: &Path) -> Vec<PathBuf> { rust_files(dir) }
fn dart_files_in(dir: &Path) -> Vec<PathBuf> { files_with_ext(dir, "dart") }

/// Bir klasördeki dart dosyalarını alt klasörlere İNMEDEN listeler.
/// `lib/` kökünde gevşek duran dosyalar (main.dart gibi) için kullanılır.
fn dart_files_direct(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir).map(|rd| {
        rd.filter_map(|e| e.ok())
          .map(|e| e.path())
          .filter(|p| p.is_file() && p.extension().and_then(|x| x.to_str()) == Some("dart"))
          .collect()
    }).unwrap_or_default()
}

fn top_module(root: &Path, file: &Path) -> Option<String> {
    let rel   = file.strip_prefix(root.join("src")).ok()?;
    let first = rel.components().next()?;
    Some(first.as_os_str().to_string_lossy().to_string())
}

fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root).unwrap_or(file).display().to_string()
}

/// `..` ve `.` bileşenlerini diskte var olma kontrolüne gerek kalmadan
/// sözel olarak çözer (relative dart import'ları için).
fn normalize_path(p: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => { out.pop(); },
            Component::CurDir => {},
            other => out.push(other),
        }
    }
    out.into_iter().collect()
}

// ── Faz 6: Flutter / Dart ────────────────────────────────────────────────────

fn read_pubspec_package_name(flutter_root: &Path) -> Option<String> {
    let pubspec = flutter_root.join("pubspec.yaml");
    let content = fs::read_to_string(pubspec).ok()?;
    let re = Regex::new(r"(?m)^name:\s*(\S+)").ok()?;
    re.captures(&content).map(|c| c[1].trim().to_string())
}

fn build_dart_index(lib_root: &Path, root: &Path) -> Result<DartIndex> {
    let mut files_map: BTreeMap<String, DartFileSummary> = BTreeMap::new();

    for file in dart_files_in(lib_root) {
        let content = fs::read_to_string(&file).unwrap_or_default();
        let summary = analyse_dart_file(&content);
        files_map.insert(relative(root, &file), summary);
    }

    let total_types = files_map.values().map(|s| s.types.len()).sum();
    Ok(DartIndex { files: files_map, total_types })
}

/// Satır-tabanlı basit Dart tarayıcı: class/mixin/enum/extension ilanlarını
/// ve üstlerindeki `///` doc yorumlarını yakalar. Tam AST değil — ama
/// implementasyon detayına girmeden API yüzeyini çıkarmak için yeterli.
fn analyse_dart_file(content: &str) -> DartFileSummary {
    let class_re     = Regex::new(r"^(?:abstract\s+)?class\s+(\w+)").unwrap();
    let mixin_re     = Regex::new(r"^mixin\s+(\w+)").unwrap();
    let enum_re      = Regex::new(r"^enum\s+(\w+)").unwrap();
    let extension_re = Regex::new(r"^extension\s*(\w+)?\s+on\s+").unwrap();

    let mut module_doc: Option<String> = None;
    let mut types: Vec<DartTypeDef> = Vec::new();
    let mut pending_doc: Vec<String> = Vec::new();
    let mut seen_first_decl = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("///") {
            pending_doc.push(rest.trim().to_string());
            continue;
        }

        if line.is_empty() || line.starts_with('@') {
            // boş satır / annotation — bekleyen doc'u bozma
            continue;
        }

        let decl = class_re.captures(line).map(|c| ("class".to_string(), c[1].to_string()))
            .or_else(|| mixin_re.captures(line).map(|c| ("mixin".to_string(), c[1].to_string())))
            .or_else(|| enum_re.captures(line).map(|c| ("enum".to_string(), c[1].to_string())))
            .or_else(|| extension_re.captures(line).map(|c| {
                let name = c.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "(anonim)".to_string());
                ("extension".to_string(), name)
            }));

        if let Some((kind, name)) = decl {
            let signature = line.trim_end_matches('{').trim().to_string();
            let doc = if pending_doc.is_empty() { None } else { Some(pending_doc.join(" ")) };
            let is_public = name == "(anonim)" || !name.starts_with('_');

            types.push(DartTypeDef { kind, name, signature, doc, is_public });
            pending_doc.clear();
            seen_first_decl = true;
            continue;
        }

        // İlk ilandan önceki bekleyen doc bloğu, dosya-seviyesi açıklama say.
        if !seen_first_decl && module_doc.is_none() && !pending_doc.is_empty() {
            module_doc = Some(pending_doc.join(" "));
        }
        pending_doc.clear();
    }

    DartFileSummary { module_doc, types }
}

fn dart_symbols_from_index(idx: &DartIndex) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (file, summary) in &idx.files {
        for t in &summary.types {
            if t.is_public && t.name != "(anonim)" {
                map.insert(t.name.clone(), file.clone());
            }
        }
    }
    map
}

/// `import` satırlarından modül-seviyeli bağımlılık grafiği kurar.
/// `package:<paket>/...` ve relative (`../`, `./`, düz) import'ları çözer;
/// `dart:` ve başka paketlerin import'larını yok sayar.
fn build_dart_dependencies(lib_root: &Path, package_name: &str) -> Result<BTreeMap<String, Vec<String>>> {
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let import_re = Regex::new(r#"^\s*import\s+['"]([^'"]+)['"]"#)?;
    let pkg_prefix = format!("package:{package_name}/");

    for file in dart_files_in(lib_root) {
        let Some(module) = dart_top_module(lib_root, &file) else { continue };
        let content = fs::read_to_string(&file).unwrap_or_default();

        for line in content.lines() {
            let Some(cap) = import_re.captures(line) else { continue };
            let import_path = &cap[1];

            let target_module: Option<String> = if import_path.starts_with("dart:") {
                None
            } else if let Some(rest) = import_path.strip_prefix(&pkg_prefix) {
                rest.split('/').next().map(str::to_string)
            } else if import_path.starts_with("package:") {
                None // başka bir paket — dış bağımlılık, atla
            } else {
                let Some(parent) = file.parent() else { continue };
                let resolved = normalize_path(&parent.join(import_path));
                resolved.strip_prefix(lib_root).ok()
                    .and_then(|rel| rel.components().next())
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
            };

            if let Some(dep) = target_module {
                if dep != module {
                    graph.entry(module.clone()).or_default().insert(dep);
                }
            }
        }
    }

    Ok(graph.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect())
}

/// Bir dart dosyasının ait olduğu üst modülü belirler.
/// `lib/` kökünde gevşek duran dosyalar "root" sahte modülüne düşer.
fn dart_top_module(lib_root: &Path, file: &Path) -> Option<String> {
    let rel = file.strip_prefix(lib_root).ok()?;
    let mut comps = rel.components();
    let first = comps.next()?;
    if comps.next().is_some() {
        Some(first.as_os_str().to_string_lossy().to_string())
    } else {
        Some("root".to_string())
    }
}

/// "root" sahte modülü gerçek bir klasöre karşılık gelmediği için
/// generic mimari fonksiyonları onun loc/files istatistiğini hesaplayamaz.
/// Burada elle düzeltiyoruz.
fn patch_dart_root_module(arch: &mut Architecture, lib_root: &Path) {
    let direct_files = dart_files_direct(lib_root);
    let loc: usize = direct_files.iter()
        .map(|f| fs::read_to_string(f).unwrap_or_default().lines().count())
        .sum();
    let files = direct_files.len();

    let entry = arch.modules.entry("root".to_string()).or_insert_with(|| ModuleStats {
        depends_on: vec![], depended_by: vec![], transitive: 0, loc: 0, files: 0,
    });
    entry.loc = loc;
    entry.files = files;

    if let Some(s) = arch.summary.iter_mut().find(|s| s.name == "root") {
        s.loc = loc;
    } else {
        arch.summary.push(ModuleSummary { name: "root".into(), layer: 0, loc, fan_in: 0, fan_out: 0 });
    }
    arch.layers.entry("root".to_string()).or_insert(0);
}

fn build_dart_semantic_index(
    lib_root: &Path,
    root:     &Path,
    dart_index: &DartIndex,
    arch:       &Architecture,
) -> Result<SemanticIndex> {
    let mut modules: BTreeMap<String, SemanticModule> = BTreeMap::new();

    for (mod_name, stats) in &arch.modules {
        let files: Vec<PathBuf> = if mod_name == "root" {
            dart_files_direct(lib_root)
        } else {
            let dir = lib_root.join(mod_name);
            if dir.exists() { dart_files_in(&dir) } else { Vec::new() }
        };

        if files.is_empty() && mod_name != "root" {
            continue;
        }

        let mut purpose: Option<String> = None;
        let mut api: Vec<SemanticItem> = Vec::new();

        for file in &files {
            let rel = relative(root, file);
            let Some(summary) = dart_index.files.get(&rel) else { continue };

            if purpose.is_none() {
                if let Some(doc) = &summary.module_doc {
                    purpose = Some(doc.clone());
                }
            }

            for t in &summary.types {
                if !t.is_public { continue; }
                api.push(SemanticItem {
                    kind:      t.kind.clone(),
                    signature: t.signature.clone(),
                    doc:       t.doc.clone(),
                });
            }
        }

        modules.insert(mod_name.clone(), SemanticModule {
            purpose,
            layer:       *arch.layers.get(mod_name).unwrap_or(&0),
            loc:         stats.loc,
            depends_on:  stats.depends_on.clone(),
            depended_by: stats.depended_by.clone(),
            api,
        });
    }

    Ok(SemanticIndex { modules })
}
