//! Minimal but real Nova language server over stdio JSON-RPC.
//!
//! Implemented: full-document sync, syntax and type diagnostics, completion,
//! hover, go-to-definition, document symbols and whole-document formatting.
//! Not implemented/advertised: incremental sync, rename, semantic tokens,
//! code actions, references, inlay hints, workspace indexing and cancellation.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};

use nova_diag::{Diagnostic, LineIndex, Severity, Span};
use nova_parser::ast::{self, Item, TypeExpr, TypeExprKind};
use nova_sema::{check, TypedProgram};
use serde_json::{json, Value};

#[derive(Default)]
struct Document {
    text: String,
    version: Option<i64>,
}

#[derive(Default)]
pub struct Server {
    documents: HashMap<String, Document>,
    shutdown: bool,
}

struct Analysis {
    program: ast::Program,
    #[allow(dead_code)]
    typed: TypedProgram,
    diagnostics: Vec<Diagnostic>,
}

pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut server = Server::default();
    loop {
        let Some(body) = read_message(&mut reader)? else {
            break;
        };
        let message: Value = match serde_json::from_str(&body) {
            Ok(message) => message,
            Err(_) => continue,
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if method == "exit" {
            break;
        }
        let id = message.get("id").cloned();
        let params = message.get("params");
        match id {
            None => server.notification(&method, params),
            Some(id) => {
                let response = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    server.request(&method, params, &id)
                })) {
                    Ok(response) => response,
                    Err(_) => error_response(id, -32603, "internal language server error"),
                };
                send(&response)?;
            }
        }
    }
    Ok(())
}

fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(length) = content_length else {
        return Ok(None);
    };
    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer)?;
    Ok(Some(String::from_utf8_lossy(&buffer).into_owned()))
}

fn send(value: &Value) -> io::Result<()> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    write!(lock, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    lock.flush()
}

fn result_response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

impl Server {
    fn notification(&mut self, method: &str, params: Option<&Value>) {
        match method {
            "textDocument/didOpen" => {
                let Some(document) = params.and_then(|p| p.get("textDocument")) else {
                    return;
                };
                let Some(uri) = document.get("uri").and_then(Value::as_str) else {
                    return;
                };
                let text = document
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let version = document.get("version").and_then(Value::as_i64);
                self.documents
                    .insert(uri.to_string(), Document { text, version });
                self.publish(uri);
            }
            "textDocument/didChange" => {
                let Some(params) = params else { return };
                let Some(uri) = params.pointer("/textDocument/uri").and_then(Value::as_str) else {
                    return;
                };
                let version = params
                    .pointer("/textDocument/version")
                    .and_then(Value::as_i64);
                let change = params
                    .get("contentChanges")
                    .and_then(Value::as_array)
                    .and_then(|changes| changes.last())
                    .and_then(|change| change.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                self.documents.insert(
                    uri.to_string(),
                    Document {
                        text: change,
                        version,
                    },
                );
                self.publish(uri);
            }
            "textDocument/didSave" => {
                if let Some(uri) = params
                    .and_then(|p| p.pointer("/textDocument/uri"))
                    .and_then(Value::as_str)
                {
                    self.publish(uri);
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = params
                    .and_then(|p| p.pointer("/textDocument/uri"))
                    .and_then(Value::as_str)
                {
                    self.documents.remove(uri);
                    let _ = send(&json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/publishDiagnostics",
                        "params": { "uri": uri, "diagnostics": [] }
                    }));
                }
            }
            _ => {}
        }
    }

    fn request(&mut self, method: &str, params: Option<&Value>, id: &Value) -> Value {
        if self.shutdown {
            return error_response(id.clone(), -32600, "server is shutting down");
        }
        match method {
            "initialize" => result_response(
                id,
                json!({
                    "capabilities": {
                        "textDocumentSync": 1,
                        "completionProvider": { "triggerCharacters": [".", "$"], "resolveProvider": false },
                        "hoverProvider": true,
                        "definitionProvider": true,
                        "documentSymbolProvider": true,
                        "documentFormattingProvider": true
                    },
                    "serverInfo": { "name": "nova-lsp", "version": env!("CARGO_PKG_VERSION") }
                }),
            ),
            "shutdown" => {
                self.shutdown = true;
                result_response(id, Value::Null)
            }
            "textDocument/completion" => {
                let Some(uri) = uri_from(params) else {
                    return result_response(id, json!({ "isIncomplete": false, "items": [] }));
                };
                let Some(doc) = self.documents.get(uri) else {
                    return result_response(id, json!({ "isIncomplete": false, "items": [] }));
                };
                let offset = position_offset(&doc.text, params);
                let items = completion(&doc.text, offset);
                result_response(id, json!({ "isIncomplete": false, "items": items }))
            }
            "textDocument/hover" => {
                let Some(doc) = self.document_for(params) else {
                    return result_response(id, Value::Null);
                };
                let Some(offset) = position_offset(&doc.text, params) else {
                    return result_response(id, Value::Null);
                };
                result_response(id, hover(&doc.text, offset))
            }
            "textDocument/definition" => {
                let Some(uri) = uri_from(params) else {
                    return result_response(id, Value::Null);
                };
                let Some(doc) = self.documents.get(uri) else {
                    return result_response(id, Value::Null);
                };
                let Some(offset) = position_offset(&doc.text, params) else {
                    return result_response(id, Value::Null);
                };
                result_response(id, definition(uri, &doc.text, offset))
            }
            "textDocument/documentSymbol" => {
                let Some(doc) = self.document_for(params) else {
                    return result_response(id, json!([]));
                };
                let analysis = analyze(&doc.text);
                result_response(id, document_symbols(&analysis.program, &doc.text))
            }
            "textDocument/formatting" => {
                let Some(doc) = self.document_for(params) else {
                    return result_response(id, json!([]));
                };
                let formatted = nova_fmt::format_source(&doc.text);
                if formatted == doc.text {
                    return result_response(id, json!([]));
                }
                let index = LineIndex::new(&doc.text);
                let (line, character) = index.utf16_position(&doc.text, doc.text.len());
                result_response(
                    id,
                    json!([{
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": { "line": line, "character": character }
                        },
                        "newText": formatted
                    }]),
                )
            }
            _ => error_response(id.clone(), -32601, "method not implemented"),
        }
    }

    fn document_for(&self, params: Option<&Value>) -> Option<&Document> {
        let uri = uri_from(params)?;
        self.documents.get(uri)
    }

    fn publish(&self, uri: &str) {
        let Some(doc) = self.documents.get(uri) else {
            return;
        };
        let analysis = analyze(&doc.text);
        let index = LineIndex::new(&doc.text);
        let diagnostics: Vec<Value> = analysis
            .diagnostics
            .iter()
            .map(|d| {
                let start = index.utf16_position(&doc.text, d.span.start);
                let end = index.utf16_position(&doc.text, d.span.end);
                json!({
                    "range": {
                        "start": { "line": start.0, "character": start.1 },
                        "end": { "line": end.0, "character": end.1 }
                    },
                    "severity": match d.severity {
                        Severity::Error => 1,
                        Severity::Warning => 2,
                        Severity::Note => 3,
                    },
                    "code": d.code,
                    "source": "nova",
                    "message": d.message
                })
            })
            .collect();
        let _ = send(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "version": doc.version,
                "diagnostics": diagnostics
            }
        }));
    }
}

fn uri_from(params: Option<&Value>) -> Option<&str> {
    params?.pointer("/textDocument/uri")?.as_str()
}

fn position_offset(text: &str, params: Option<&Value>) -> Option<usize> {
    let position = params?.get("position")?;
    let line = position.get("line")?.as_u64()? as u32;
    let character = position.get("character")?.as_u64()? as u32;
    LineIndex::new(text).offset_at_utf16(text, line, character)
}

fn analyze(text: &str) -> Analysis {
    let parsed = nova_parser::parse(text);
    let checked = check(&parsed.program);
    let mut diagnostics = parsed.diagnostics;
    diagnostics.extend(checked.diagnostics);
    Analysis {
        program: parsed.program,
        typed: checked.program,
        diagnostics,
    }
}

fn span_range(text: &str, span: Span) -> Value {
    let index = LineIndex::new(text);
    let start = index.utf16_position(text, span.start);
    let end = index.utf16_position(text, span.end);
    json!({
        "start": { "line": start.0, "character": start.1 },
        "end": { "line": end.0, "character": end.1 }
    })
}

fn word_at(text: &str, offset: usize) -> Option<(String, Span)> {
    if text.is_empty() {
        return None;
    }
    let offset = offset.min(text.len());
    let is_word = |c: char| c == '_' || c.is_alphanumeric();
    let mut start = offset;
    for (idx, ch) in text[..offset].char_indices().rev() {
        if is_word(ch) {
            start = idx;
        } else {
            break;
        }
    }
    let mut end = offset;
    for (idx, ch) in text[offset..].char_indices() {
        if is_word(ch) {
            end = offset + idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if start == end {
        None
    } else {
        Some((text[start..end].to_string(), Span::new(start, end)))
    }
}

fn completion(text: &str, offset: Option<usize>) -> Vec<Value> {
    let Some(offset) = offset else {
        return Vec::new();
    };
    let prefix = &text[..offset.min(text.len())];
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let current = prefix[line_start..].trim_end();
    if current.ends_with('.') {
        return [
            ("len", "number of elements or UTF-8 bytes"),
            ("is_empty", "true when empty"),
            ("get", "checked element access (Option)"),
            ("push", "append to a mutable List"),
            ("to_string", "convert a scalar or String"),
            ("unwrap_or", "Option/Result fallback"),
            ("is_some", "Option check"),
            ("is_none", "Option check"),
            ("is_ok", "Result check"),
            ("is_err", "Result check"),
        ]
        .iter()
        .map(|(label, detail)| {
            json!({ "label": label, "kind": 2, "detail": detail, "sortText": format!("0{label}") })
        })
        .collect();
    }
    let mut items = Vec::new();
    for keyword in [
        "fn", "let", "var", "return", "if", "else", "while", "for", "in", "match", "struct",
        "enum", "import", "true", "false", "break", "continue",
    ] {
        items.push(json!({ "label": keyword, "kind": 14, "detail": "keyword" }));
    }
    for (label, detail, body) in [
        (
            "fn",
            "function declaration",
            "fn ${1:name}(${2:param: Type}) -> ${3:Type} {\n    ${0}\n}",
        ),
        ("for", "for loop", "for ${1:i} in ${2:0..10} {\n    ${0}\n}"),
        (
            "match",
            "match expression",
            "match ${1:value} {\n    ${2:pattern} => ${3:body}\n}",
        ),
        ("if", "if expression", "if ${1:condition} {\n    ${0}\n}"),
    ] {
        items.push(json!({
            "label": label, "kind": 15, "detail": detail,
            "insertText": body, "insertTextFormat": 2
        }));
    }
    for (name, detail) in [
        ("println", "print a value with a newline"),
        ("print", "print a value"),
        ("eprintln", "print to stderr"),
        ("assert", "abort if a Bool is false"),
        ("assert_eq", "abort if two values differ"),
        ("panic", "abort with a message"),
    ] {
        items.push(json!({ "label": name, "kind": 3, "detail": detail }));
    }
    let parsed = nova_parser::parse(text);
    for item in &parsed.program.items {
        match item {
            Item::Fn(function) => items.push(json!({
                "label": function.name,
                "kind": 3,
                "detail": signature(function),
            })),
            Item::Struct(decl) => items.push(json!({
                "label": decl.name, "kind": 22,
                "detail": format!("struct with {} field(s)", decl.fields.len())
            })),
            Item::Enum(decl) => items.push(json!({
                "label": decl.name, "kind": 10, "detail": "enum (not implemented)"
            })),
            Item::Impl(decl) => {
                for method in &decl.methods {
                    items.push(json!({
                        "label": method.name,
                        "kind": 2,
                        "detail": format!("{}.{}", decl.type_name, signature(method)),
                    }));
                }
            }
            Item::Trait(decl) => {
                for method in &decl.methods {
                    items.push(json!({
                        "label": method.name,
                        "kind": 2,
                        "detail": format!("{}.{}", decl.name, method.name),
                    }));
                }
            }
            Item::Extern(decl) => items.push(json!({
                "label": decl.name,
                "kind": 3,
                "detail": format!("extern \"{}\" function", decl.abi),
            })),
            Item::Macro(decl) => items.push(json!({
                "label": decl.name,
                "kind": 14,
                "detail": "expression macro",
            })),
            Item::Import(_) => {}
        }
    }
    items
}

fn hover(text: &str, offset: usize) -> Value {
    let Some((word, span)) = word_at(text, offset) else {
        return Value::Null;
    };
    let parsed = nova_parser::parse(text);
    let mut markdown = None;
    for item in &parsed.program.items {
        match item {
            Item::Fn(function) if function.name == word => {
                let mut value = format!("```nova\n{}\n```", signature(function));
                if function.is_test {
                    value.push_str("\n\n`@test` function");
                }
                markdown = Some(value);
            }
            Item::Struct(decl) if decl.name == word => {
                let mut value = format!("```nova\nstruct {} {{\n", decl.name);
                for field in &decl.fields {
                    value.push_str(&format!("    {}: {}\n", field.name, type_expr(&field.ty)));
                }
                value.push_str("}\n```");
                markdown = Some(value);
            }
            _ => {}
        }
    }
    if markdown.is_none() {
        markdown = builtin_doc(&word).map(str::to_string);
    }
    let Some(markdown) = markdown else {
        return Value::Null;
    };
    json!({
        "contents": { "kind": "markdown", "value": markdown },
        "range": span_range(text, span)
    })
}

fn definition(uri: &str, text: &str, offset: usize) -> Value {
    let Some((word, _)) = word_at(text, offset) else {
        return Value::Null;
    };
    let parsed = nova_parser::parse(text);
    for item in &parsed.program.items {
        match item {
            Item::Fn(function) if function.name == word => {
                return json!({
                    "uri": uri,
                    "range": span_range(text, function.name_span)
                });
            }
            Item::Struct(decl) if decl.name == word => {
                return json!({
                    "uri": uri,
                    "range": span_range(text, decl.name_span)
                });
            }
            _ => {}
        }
    }
    Value::Null
}

fn document_symbols(program: &ast::Program, text: &str) -> Value {
    let mut symbols = Vec::new();
    for item in &program.items {
        match item {
            Item::Fn(function) => symbols.push(json!({
                "name": function.name,
                "kind": 12,
                "detail": signature(function),
                "range": span_range(text, function.span),
                "selectionRange": span_range(text, function.name_span)
            })),
            Item::Struct(decl) => {
                let children: Vec<Value> = decl
                    .fields
                    .iter()
                    .map(|field| {
                        json!({
                            "name": field.name,
                            "kind": 8,
                            "detail": type_expr(&field.ty),
                            "range": span_range(text, field.span),
                            "selectionRange": span_range(text, field.name_span)
                        })
                    })
                    .collect();
                symbols.push(json!({
                    "name": decl.name,
                    "kind": 23,
                    "range": span_range(text, decl.span),
                    "selectionRange": span_range(text, decl.name_span),
                    "children": children
                }));
            }
            Item::Enum(decl) => {
                let children: Vec<Value> = decl
                    .variants
                    .iter()
                    .map(|variant| {
                        json!({
                            "name": variant.name,
                            "kind": 22,
                            "range": span_range(text, variant.span),
                            "selectionRange": span_range(text, variant.name_span)
                        })
                    })
                    .collect();
                symbols.push(json!({
                    "name": decl.name,
                    "kind": 10,
                    "range": span_range(text, decl.span),
                    "selectionRange": span_range(text, decl.name_span),
                    "children": children
                }));
            }
            Item::Impl(decl) => {
                let children: Vec<Value> = decl
                    .methods
                    .iter()
                    .map(|method| {
                        json!({
                            "name": format!("{}.{}", decl.type_name, method.name),
                            "kind": 6,
                            "detail": signature(method),
                            "range": span_range(text, method.span),
                            "selectionRange": span_range(text, method.name_span)
                        })
                    })
                    .collect();
                symbols.push(json!({
                    "name": format!("impl {}", decl.type_name),
                    "kind": 5,
                    "range": span_range(text, decl.span),
                    "selectionRange": span_range(text, decl.type_span),
                    "children": children
                }));
            }
            Item::Trait(decl) => {
                let children: Vec<Value> = decl
                    .methods
                    .iter()
                    .map(|method| {
                        json!({
                            "name": method.name,
                            "kind": 6,
                            "range": span_range(text, method.span),
                            "selectionRange": span_range(text, method.name_span)
                        })
                    })
                    .collect();
                symbols.push(json!({
                    "name": format!("trait {}", decl.name),
                    "kind": 11,
                    "range": span_range(text, decl.span),
                    "selectionRange": span_range(text, decl.name_span),
                    "children": children
                }));
            }
            Item::Extern(decl) => symbols.push(json!({
                "name": decl.name,
                "kind": 12,
                "detail": format!("extern \"{}\"", decl.abi),
                "range": span_range(text, decl.span),
                "selectionRange": span_range(text, decl.name_span)
            })),
            Item::Macro(decl) => symbols.push(json!({
                "name": decl.name,
                "kind": 14,
                "range": span_range(text, decl.span),
                "selectionRange": span_range(text, decl.name_span)
            })),
            Item::Import(import) => symbols.push(json!({
                "name": import
                    .path
                    .clone()
                    .unwrap_or_else(|| import.module.join(".")),
                "kind": 2,
                "range": span_range(text, import.span),
                "selectionRange": span_range(text, import.span)
            })),
        }
    }
    Value::Array(symbols)
}

fn signature(function: &ast::FnDecl) -> String {
    let params: Vec<String> = function
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, type_expr(&param.ty)))
        .collect();
    let ret = function
        .ret
        .as_ref()
        .map(|ty| format!(" -> {}", type_expr(ty)))
        .unwrap_or_default();
    format!("fn {}({}){}", function.name, params.join(", "), ret)
}

fn type_expr(ty: &TypeExpr) -> String {
    match &ty.kind {
        TypeExprKind::Unit => "Unit".into(),
        TypeExprKind::Tuple(items) => {
            format!(
                "({})",
                items.iter().map(type_expr).collect::<Vec<_>>().join(", ")
            )
        }
        TypeExprKind::Array { elem, len } => format!("[{}; {len}]", type_expr(elem)),
        TypeExprKind::Fn { params, ret } => format!(
            "fn({}) -> {}",
            params.iter().map(type_expr).collect::<Vec<_>>().join(", "),
            type_expr(ret)
        ),
        TypeExprKind::Dyn(name) => format!("dyn {name}"),
        TypeExprKind::Named { path, args } => {
            let base = path.join(".");
            if args.is_empty() {
                base
            } else {
                format!(
                    "{}<{}>",
                    base,
                    args.iter().map(type_expr).collect::<Vec<_>>().join(", ")
                )
            }
        }
    }
}

fn builtin_doc(word: &str) -> Option<&'static str> {
    Some(match word {
        "println" => "```nova\nfn println(value: printable)\n```\nPrint a value followed by a newline.",
        "print" => "```nova\nfn print(value: printable)\n```\nPrint a value without a trailing newline.",
        "eprintln" => "```nova\nfn eprintln(value: printable)\n```\nPrint a value followed by a newline to stderr.",
        "assert" => "```nova\nfn assert(condition: Bool)\n```\nAbort the process when the condition is false.",
        "assert_eq" => "```nova\nfn assert_eq<T: equatable>(left: T, right: T)\n```\nAbort when the values differ.",
        "panic" => "```nova\nfn panic(message: String) -> Never\n```\nAbort the process with a message.",
        "Int" => "`Int` — signed 64-bit integer (`i64`). Arithmetic is checked.",
        "Float" => "`Float` — IEEE-754 binary64. No implicit conversion from Int.",
        "Bool" => "`Bool` — `true` or `false`.",
        "Char" => "`Char` — Unicode scalar value.",
        "String" => "`String` — immutable UTF-8 value. `.len()` returns bytes.",
        "Unit" => "`Unit` — the absence of a value.",
        "List" => "`List<T>` — growable list. Indexing is bounds-checked; `.get` returns `Option<T>`.",
        "Option" => "`Option<T>` — `Some(value)` or `None`.",
        "Result" => "`Result<T, E>` — `Ok(value)` or `Err(error)`.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_reports_errors() {
        let analysis = analyze("fn main() { let x: Int = \"no\" }");
        assert!(analysis.diagnostics.iter().any(|d| d.code == "E0308"));
    }

    #[test]
    fn word_span_is_correct() {
        let text = "fn main() { let count = 1 }";
        let offset = text.find("count").unwrap() + 2;
        let (word, span) = word_at(text, offset).unwrap();
        assert_eq!(word, "count");
        assert_eq!(&text[span.start..span.end], "count");
    }

    #[test]
    fn definition_finds_function() {
        let text = "fn helper() { }\nfn main() { helper() }";
        let offset = text.rfind("helper").unwrap();
        let value = definition("file:///main.nova", text, offset);
        assert_eq!(value["range"]["start"]["line"], 0);
    }

    #[test]
    fn hover_shows_signature() {
        let text = "fn greet(name: String) -> String { return name }\nfn main() { greet(\"x\") }";
        let offset = text.rfind("greet").unwrap();
        let value = hover(text, offset);
        assert!(value["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("fn greet"));
    }

    #[test]
    fn formatting_uses_formatter() {
        let source = "fn main() {\nlet x = 1\n}\n";
        let formatted = nova_fmt::format_source(source);
        assert!(formatted.contains("    let x = 1"));
    }

    #[test]
    fn position_mapping_handles_utf16() {
        let text = "let s = \"😀\"\nlet x = 1\n";
        let offset = text.find(';').unwrap_or(text.len());
        let _ = offset;
        let index = LineIndex::new(text);
        assert_eq!(
            index.offset_at_utf16(text, 1, 0),
            Some(text.find("let x").unwrap())
        );
    }
}
