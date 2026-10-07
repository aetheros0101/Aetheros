//! Minimal LSP stdio JSON-RPC client.
//!
//! Spawns a language server process and speaks LSP over stdin/stdout.
//! Designed for host-driven use via [`StdioLanguageServer`].

use super::{CompletionItem, LanguageServer, Position};
use crate::error::{Result, WorkspaceError};
use crate::models::{CodeAction, Diagnostic, TextEdit, TextRange};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub struct StdioLanguageServer {
    language_ids: Vec<&'static str>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    reader: Option<Mutex<BufReader<ChildStdout>>>,
    next_id: AtomicU64,
    root_uri: String,
    /// path → version for didOpen tracking
    open_docs: HashMap<String, u64>,
    cmd: String,
    args: Vec<String>,
}

impl StdioLanguageServer {
    pub fn new(language_ids: &[&'static str], cmd: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            language_ids: language_ids.to_vec(),
            child: None,
            stdin: None,
            reader: None,
            next_id: AtomicU64::new(1),
            root_uri: String::new(),
            open_docs: HashMap::new(),
            cmd: cmd.into(),
            args,
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_message(&msg)
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(&msg)?;
        self.read_response(id)
    }

    fn write_message(&mut self, msg: &Value) -> Result<()> {
        let body = serde_json::to_string(msg)
            .map_err(|e| WorkspaceError::Lsp(e.to_string()))?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| WorkspaceError::Lsp("server not started".into()))?;
        stdin
            .write_all(header.as_bytes())
            .and_then(|_| stdin.write_all(body.as_bytes()))
            .and_then(|_| stdin.flush())
            .map_err(|e| WorkspaceError::Lsp(e.to_string()))?;
        Ok(())
    }

    fn read_response(&mut self, expect_id: u64) -> Result<Value> {
        // Read messages until we get a response with matching id.
        // Notifications (no id) are skipped (diagnostics handled via pull).
        for _ in 0..50 {
            let msg = self.read_message()?;
            if msg.get("id").and_then(|v| v.as_u64()) == Some(expect_id) {
                if let Some(err) = msg.get("error") {
                    return Err(WorkspaceError::Lsp(err.to_string()));
                }
                return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
            }
            // stash publishDiagnostics if needed — for now ignore
        }
        Err(WorkspaceError::Lsp("timeout waiting for LSP response".into()))
    }

    fn read_message(&mut self) -> Result<Value> {
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| WorkspaceError::Lsp("server not started".into()))?;
        let mut guard = reader
            .lock()
            .map_err(|_| WorkspaceError::Lsp("reader lock poisoned".into()))?;

        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            guard
                .read_line(&mut line)
                .map_err(|e| WorkspaceError::Lsp(e.to_string()))?;
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
            if let Some(v) = line
                .to_lowercase()
                .strip_prefix("content-length:")
            {
                content_length = v.trim().parse().ok();
            }
        }
        let len = content_length
            .ok_or_else(|| WorkspaceError::Lsp("missing Content-Length".into()))?;
        let mut buf = vec![0u8; len];
        guard
            .read_exact(&mut buf)
            .map_err(|e| WorkspaceError::Lsp(e.to_string()))?;
        serde_json::from_slice(&buf).map_err(|e| WorkspaceError::Lsp(e.to_string()))
    }

    fn path_to_uri(root: &Path, path: &str) -> String {
        let full = root.join(path);
        format!("file://{}", full.to_string_lossy().replace('\\', "/"))
    }
}

impl LanguageServer for StdioLanguageServer {
    fn language_ids(&self) -> &[&str] {
        &self.language_ids
    }

    fn initialize(&mut self, root: &Path) -> Result<()> {
        let mut child = Command::new(&self.cmd)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| WorkspaceError::Lsp(format!("spawn {}: {e}", self.cmd)))?;

        self.stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| WorkspaceError::Lsp("no stdout".into()))?;
        self.reader = Some(Mutex::new(BufReader::new(stdout)));
        self.child = Some(child);
        self.root_uri = format!("file://{}", root.to_string_lossy().replace('\\', "/"));

        let result = self.send_request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": self.root_uri,
                "capabilities": {
                    "textDocument": {
                        "hover": { "contentFormat": ["plaintext", "markdown"] },
                        "completion": { "completionItem": { "snippetSupport": false } },
                        "publishDiagnostics": {},
                    },
                    "workspace": {}
                }
            }),
        )?;
        let _ = result;
        self.send_notification("initialized", json!({}))?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        let _ = self.send_request("shutdown", Value::Null);
        let _ = self.send_notification("exit", Value::Null);
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stdin = None;
        self.reader = None;
        Ok(())
    }

    fn did_open(&mut self, path: &str, language_id: &str, text: &str, version: u64) -> Result<()> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        self.send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": version,
                    "text": text,
                }
            }),
        )?;
        self.open_docs.insert(path.replace('\\', "/"), version);
        Ok(())
    }

    fn did_change(&mut self, path: &str, text: &str, version: u64) -> Result<()> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        if !self.open_docs.contains_key(&path.replace('\\', "/")) {
            // open first with unknown language
            self.send_notification(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": uri,
                        "languageId": self.language_ids.first().copied().unwrap_or("plaintext"),
                        "version": version,
                        "text": text,
                    }
                }),
            )?;
        } else {
            self.send_notification(
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{ "text": text }]
                }),
            )?;
        }
        self.open_docs.insert(path.replace('\\', "/"), version);
        Ok(())
    }

    fn did_close(&mut self, path: &str) -> Result<()> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        self.send_notification(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        )?;
        self.open_docs.remove(&path.replace('\\', "/"));
        Ok(())
    }

    fn hover(&mut self, path: &str, pos: Position) -> Result<Option<String>> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        let result = self.send_request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": pos.line, "character": pos.character }
            }),
        )?;
        if result.is_null() {
            return Ok(None);
        }
        let content = result
            .pointer("/contents/value")
            .or_else(|| result.pointer("/contents"))
            .cloned();
        match content {
            Some(Value::String(s)) => Ok(Some(s)),
            Some(Value::Array(arr)) => {
                let parts: Vec<String> = arr
                    .iter()
                    .filter_map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .or_else(|| v.get("value").and_then(|x| x.as_str()).map(|s| s.to_string()))
                    })
                    .collect();
                Ok(Some(parts.join("\n")))
            }
            Some(v) => Ok(v.get("value").and_then(|x| x.as_str()).map(|s| s.to_string())),
            None => Ok(None),
        }
    }

    fn goto_definition(&mut self, path: &str, pos: Position) -> Result<Vec<TextRange>> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        let result = self.send_request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": pos.line, "character": pos.character }
            }),
        )?;
        Ok(parse_locations(&result))
    }

    fn references(&mut self, path: &str, pos: Position) -> Result<Vec<(String, TextRange)>> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        let result = self.send_request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": pos.line, "character": pos.character },
                "context": { "includeDeclaration": true }
            }),
        )?;
        Ok(parse_location_links(&result))
    }

    fn completion(&mut self, path: &str, pos: Position) -> Result<Vec<CompletionItem>> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        let result = self.send_request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": pos.line, "character": pos.character }
            }),
        )?;
        let items = result
            .get("items")
            .cloned()
            .unwrap_or(result);
        let arr = match items {
            Value::Array(a) => a,
            _ => return Ok(Vec::new()),
        };
        Ok(arr
            .iter()
            .filter_map(|v| {
                Some(CompletionItem {
                    label: v.get("label")?.as_str()?.to_string(),
                    kind: v.get("kind").map(|k| k.to_string()),
                    detail: v.get("detail").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    insert_text: v
                        .get("insertText")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string()),
                    documentation: v
                        .pointer("/documentation/value")
                        .or_else(|| v.get("documentation"))
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect())
    }

    fn code_actions(&mut self, path: &str, range: TextRange) -> Result<Vec<CodeAction>> {
        let root = PathBuf::from(self.root_uri.trim_start_matches("file://"));
        let uri = Self::path_to_uri(&root, path);
        let result = self.send_request(
            "textDocument/codeAction",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": range.start_line.saturating_sub(1), "character": range.start_column.saturating_sub(1) },
                    "end": { "line": range.end_line.saturating_sub(1), "character": range.end_column.saturating_sub(1) }
                },
                "context": { "diagnostics": [] }
            }),
        )?;
        let arr = match result {
            Value::Array(a) => a,
            _ => return Ok(Vec::new()),
        };
        Ok(arr
            .iter()
            .filter_map(|v| {
                let title = v.get("title")?.as_str()?.to_string();
                let edits = extract_edits(v);
                Some(CodeAction {
                    title,
                    kind: v.get("kind").and_then(|k| k.as_str()).map(|s| s.to_string()),
                    path: path.to_string(),
                    range: range.clone(),
                    is_preferred: v
                        .get("isPreferred")
                        .and_then(|p| p.as_bool())
                        .unwrap_or(false),
                    edits,
                })
            })
            .collect())
    }

    fn diagnostics(&mut self, _path: &str) -> Result<Vec<Diagnostic>> {
        // Pull diagnostics not universally supported; return empty.
        // Host can push via Workspace::set_diagnostics from publishDiagnostics.
        Ok(Vec::new())
    }
}

fn parse_locations(v: &Value) -> Vec<TextRange> {
    match v {
        Value::Array(arr) => arr.iter().filter_map(location_range).collect(),
        Value::Object(_) => location_range(v).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn parse_location_links(v: &Value) -> Vec<(String, TextRange)> {
    match v {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                let uri = item
                    .get("uri")
                    .or_else(|| item.get("targetUri"))
                    .and_then(|u| u.as_str())?
                    .to_string();
                let range = location_range(item)?;
                Some((uri, range))
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn location_range(v: &Value) -> Option<TextRange> {
    let range = v.get("range").or_else(|| v.get("targetRange"))?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    Some(TextRange {
        start_line: start.get("line")?.as_u64()? as u32 + 1,
        start_column: start.get("character")?.as_u64()? as u32 + 1,
        end_line: end.get("line")?.as_u64()? as u32 + 1,
        end_column: end.get("character")?.as_u64()? as u32 + 1,
    })
}

fn extract_edits(action: &Value) -> Vec<TextEdit> {
    let mut out = Vec::new();
    // edit.changes or edit.documentChanges
    if let Some(changes) = action.pointer("/edit/changes").and_then(|c| c.as_object()) {
        for (uri, edits) in changes {
            if let Some(arr) = edits.as_array() {
                for e in arr {
                    if let Some(te) = text_edit_from_json(uri, e) {
                        out.push(te);
                    }
                }
            }
        }
    }
    out
}

fn text_edit_from_json(uri: &str, e: &Value) -> Option<TextEdit> {
    let range = location_range(e)?;
    let new_text = e.get("newText")?.as_str()?.to_string();
    let path = uri
        .trim_start_matches("file://")
        .to_string();
    Some(TextEdit {
        path,
        range,
        new_text,
    })
}

impl Drop for StdioLanguageServer {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
