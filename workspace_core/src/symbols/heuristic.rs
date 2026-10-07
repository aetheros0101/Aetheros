use crate::models::{Symbol, SymbolKind};

/// Improved multi-language heuristic symbol extraction.
pub fn extract_heuristic(path: &str, text: &str, language: Option<&str>) -> Vec<Symbol> {
    let lang = language.unwrap_or_else(|| guess_lang(path));
    match lang {
        "rust" => extract_rust(path, text),
        "typescript" | "javascript" => extract_js_ts(path, text),
        "python" => extract_python(path, text),
        "go" => extract_go(path, text),
        "c" | "cpp" => extract_c_family(path, text),
        "java" | "kotlin" => extract_java_like(path, text),
        _ => Vec::new(),
    }
}

fn guess_lang(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "java" => "java",
        "kt" => "kotlin",
        _ => "",
    }
}

fn push_sym(
    out: &mut Vec<Symbol>,
    path: &str,
    line_no: usize,
    line: &str,
    kind: SymbolKind,
    name: &str,
    container: Option<String>,
) {
    if name.is_empty() || !is_ident_start(name.chars().next().unwrap_or('\0')) {
        return;
    }
    let col = line.find(name).unwrap_or(0) as u32 + 1;
    out.push(Symbol {
        name: name.to_string(),
        kind,
        path: path.to_string(),
        line: line_no as u32 + 1,
        column: col,
        end_line: line_no as u32 + 1,
        end_column: col + name.chars().count() as u32,
        container,
    });
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn take_ident(s: &str) -> &str {
    let end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_alphanumeric() && *c != '_')
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    &s[..end]
}

fn strip_vis(s: &str) -> &str {
    let s = s.trim_start();
    for p in [
        "pub(crate) ",
        "pub(super) ",
        "pub(self) ",
        "pub ",
        "async ",
        "unsafe ",
        "const ",
        "default ",
        "static ",
    ] {
        if let Some(rest) = s.strip_prefix(p) {
            return strip_vis(rest);
        }
    }
    s
}

fn extract_rust(path: &str, text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut container_stack: Vec<(usize, String)> = Vec::new(); // (indent_hint, name)

    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[") {
            continue;
        }
        // Pop containers when a bare closing brace appears (approx).
        if trimmed == "}" {
            container_stack.pop();
            continue;
        }
        let body = strip_vis(trimmed);

        // Rough container tracking via impl / mod blocks
        if let Some(rest) = body.strip_prefix("impl ") {
            let name = rest
                .split(|c: char| c == '<' || c == '{' || c == ' ')
                .find(|s| !s.is_empty() && *s != "for")
                .unwrap_or("")
                .trim();
            // handle `impl Trait for Type`
            let name = if rest.contains(" for ") {
                rest.split(" for ")
                    .nth(1)
                    .and_then(|s| s.split(|c: char| c == '<' || c == '{').next())
                    .unwrap_or(name)
                    .trim()
            } else {
                name
            };
            if !name.is_empty() {
                container_stack.push((line_no, name.to_string()));
            }
        }

        let container = container_stack.last().map(|(_, n)| n.clone());

        if let Some(rest) = body.strip_prefix("fn ") {
            let name = take_ident(rest.trim_start());
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                if container.is_some() {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                },
                name,
                container,
            );
        } else if let Some(rest) = body.strip_prefix("struct ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Struct,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = body.strip_prefix("enum ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Enum,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = body.strip_prefix("trait ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Interface,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = body.strip_prefix("mod ") {
            let name = take_ident(rest.trim_start());
            push_sym(&mut out, path, line_no, line, SymbolKind::Module, name, None);
            if trimmed.ends_with('{') {
                container_stack.push((line_no, name.to_string()));
            }
        } else if let Some(rest) = body.strip_prefix("const ") {
            let name = take_ident(rest.trim_start());
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Constant,
                name,
                container,
            );
        } else if let Some(rest) = body.strip_prefix("type ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Class,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = body.strip_prefix("macro_rules! ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Function,
                take_ident(rest.trim_start()),
                None,
            );
        }
    }
    out
}

fn extract_js_ts(path: &str, text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('*') {
            continue;
        }
        let t = trimmed
            .trim_start_matches("export ")
            .trim_start_matches("default ")
            .trim_start_matches("async ")
            .trim_start_matches("declare ");

        if let Some(rest) = t.strip_prefix("function ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Function,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("class ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Class,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("interface ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Interface,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("type ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Class,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("enum ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Enum,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("const ") {
            let name = take_ident(rest.trim_start());
            let kind = if trimmed.contains("=>") || trimmed.contains("function") {
                SymbolKind::Function
            } else {
                SymbolKind::Constant
            };
            push_sym(&mut out, path, line_no, line, kind, name, None);
        } else if let Some(rest) = t.strip_prefix("let ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Variable,
                take_ident(rest.trim_start()),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("var ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Variable,
                take_ident(rest.trim_start()),
                None,
            );
        }
        // methods:  name(
        else if let Some(idx) = trimmed.find('(') {
            let before = trimmed[..idx].trim();
            if before.starts_with("get ") || before.starts_with("set ") {
                let name = take_ident(before.split_whitespace().nth(1).unwrap_or(""));
                push_sym(
                    &mut out,
                    path,
                    line_no,
                    line,
                    SymbolKind::Method,
                    name,
                    None,
                );
            }
        }
    }
    out
}

fn extract_python(path: &str, text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut class_stack: Vec<(usize, String)> = Vec::new(); // (indent, name)

    for (line_no, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();

        while class_stack
            .last()
            .map(|(i, _)| *i >= indent)
            .unwrap_or(false)
        {
            class_stack.pop();
        }

        if let Some(rest) = trimmed.strip_prefix("class ") {
            let name = take_ident(rest);
            push_sym(&mut out, path, line_no, line, SymbolKind::Class, name, None);
            class_stack.push((indent, name.to_string()));
        } else if let Some(rest) = trimmed
            .strip_prefix("def ")
            .or_else(|| trimmed.strip_prefix("async def "))
        {
            let name = take_ident(rest.trim_start());
            let container = class_stack.last().map(|(_, n)| n.clone());
            let kind = if container.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            push_sym(&mut out, path, line_no, line, kind, name, container);
        }
    }
    out
}

fn extract_go(path: &str, text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("func ") {
            // func (r *T) Name(  or  func Name(
            let name = if rest.trim_start().starts_with('(') {
                rest.split(')')
                    .nth(1)
                    .map(|s| take_ident(s.trim_start()))
                    .unwrap_or("")
            } else {
                take_ident(rest.trim_start())
            };
            let kind = if rest.trim_start().starts_with('(') {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            push_sym(&mut out, path, line_no, line, kind, name, None);
        } else if let Some(rest) = trimmed.strip_prefix("type ") {
            let name = take_ident(rest.trim_start());
            let kind = if rest.contains("struct") {
                SymbolKind::Struct
            } else if rest.contains("interface") {
                SymbolKind::Interface
            } else {
                SymbolKind::Class
            };
            push_sym(&mut out, path, line_no, line, kind, name, None);
        } else if let Some(rest) = trimmed.strip_prefix("const ") {
            // const Name = or const (
            let name = take_ident(rest.trim_start());
            if !name.is_empty() && name != "(" {
                push_sym(
                    &mut out,
                    path,
                    line_no,
                    line,
                    SymbolKind::Constant,
                    name,
                    None,
                );
            }
        }
    }
    out
}

fn extract_c_family(path: &str, text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("class ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Class,
                take_ident(rest),
                None,
            );
        } else if let Some(rest) = trimmed.strip_prefix("struct ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Struct,
                take_ident(rest),
                None,
            );
        } else if let Some(rest) = trimmed.strip_prefix("enum ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Enum,
                take_ident(rest),
                None,
            );
        } else if let Some(rest) = trimmed.strip_prefix("namespace ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Namespace,
                take_ident(rest),
                None,
            );
        }
        // rough function: type name(
        else if trimmed.contains('(') && trimmed.ends_with('{') || trimmed.ends_with(')') {
            if let Some(paren) = trimmed.find('(') {
                let before = trimmed[..paren].trim();
                let name = before.split_whitespace().last().unwrap_or("");
                let name = name.trim_start_matches('*');
                if is_ident_start(name.chars().next().unwrap_or('\0'))
                    && !matches!(
                        name,
                        "if" | "for" | "while" | "switch" | "return" | "sizeof"
                    )
                {
                    push_sym(
                        &mut out,
                        path,
                        line_no,
                        line,
                        SymbolKind::Function,
                        name,
                        None,
                    );
                }
            }
        }
    }
    out
}

fn extract_java_like(path: &str, text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('@') {
            continue;
        }
        let t = trimmed
            .trim_start_matches("public ")
            .trim_start_matches("private ")
            .trim_start_matches("protected ")
            .trim_start_matches("static ")
            .trim_start_matches("final ")
            .trim_start_matches("abstract ")
            .trim_start_matches("open ")
            .trim_start_matches("data ")
            .trim_start_matches("inner ");

        if let Some(rest) = t.strip_prefix("class ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Class,
                take_ident(rest),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("interface ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Interface,
                take_ident(rest),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("enum ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Enum,
                take_ident(rest),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("fun ") {
            // Kotlin
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Function,
                take_ident(rest),
                None,
            );
        } else if let Some(rest) = t.strip_prefix("object ") {
            push_sym(
                &mut out,
                path,
                line_no,
                line,
                SymbolKind::Class,
                take_ident(rest),
                None,
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_extracts_fn_struct_impl_method() {
        let src = r#"
pub struct Foo;
impl Foo {
    pub fn bar(&self) {}
}
fn free() {}
"#;
        let syms = extract_rust("a.rs", src);
        assert!(syms.iter().any(|s| s.name == "Foo" && s.kind == SymbolKind::Struct));
        assert!(syms.iter().any(|s| s.name == "bar" && s.kind == SymbolKind::Method));
        assert!(syms.iter().any(|s| s.name == "free" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn python_class_methods() {
        let src = "class A:\n    def meth(self):\n        pass\n\ndef top():\n    pass\n";
        let syms = extract_python("a.py", src);
        assert!(syms.iter().any(|s| s.name == "A"));
        assert!(syms.iter().any(|s| s.name == "meth" && s.container.as_deref() == Some("A")));
        assert!(syms.iter().any(|s| s.name == "top" && s.kind == SymbolKind::Function));
    }
}
