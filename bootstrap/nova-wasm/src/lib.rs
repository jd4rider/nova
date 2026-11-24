//! Browser-facing Nova front end.
//!
//! This crate intentionally does **not** include LLVM generation or the native
//! runtime. The website playground can therefore parse, type-check and format
//! real Nova source inside WebAssembly without pretending it can execute a
//! native program.

use std::alloc::{alloc, dealloc, Layout};
use std::slice;
use std::sync::Mutex;

use nova_diag::{Diagnostic, LineIndex};
use nova_parser::ast::Item;
use serde::Serialize;

#[derive(Serialize)]
struct Point {
    line: u32,
    character: u32,
}

#[derive(Serialize)]
struct LabelView {
    message: String,
    start: Point,
    end: Point,
}

#[derive(Serialize)]
struct DiagnosticView {
    code: String,
    severity: String,
    message: String,
    start: Point,
    end: Point,
    notes: Vec<String>,
    labels: Vec<LabelView>,
}

#[derive(Serialize)]
struct Stats {
    lines: usize,
    bytes: usize,
    functions: usize,
    structs: usize,
    errors: usize,
    warnings: usize,
}

#[derive(Serialize)]
struct CheckResponse {
    ok: bool,
    diagnostics: Vec<DiagnosticView>,
    stats: Stats,
    version: &'static str,
}

impl DiagnosticView {
    fn from_diagnostic(diagnostic: &Diagnostic, index: &LineIndex, source: &str) -> Self {
        let (start_line, start_character) = index.utf16_position(source, diagnostic.span.start);
        let (end_line, end_character) = index.utf16_position(source, diagnostic.span.end);
        Self {
            code: diagnostic.code.clone(),
            severity: diagnostic.severity.as_str().to_string(),
            message: diagnostic.message.clone(),
            start: Point {
                line: start_line,
                character: start_character,
            },
            end: Point {
                line: end_line,
                character: end_character,
            },
            notes: diagnostic.notes.clone(),
            labels: diagnostic
                .labels
                .iter()
                .map(|label| {
                    let (line, character) = index.utf16_position(source, label.span.start);
                    let (end_line, end_character) = index.utf16_position(source, label.span.end);
                    LabelView {
                        message: label.message.clone(),
                        start: Point { line, character },
                        end: Point {
                            line: end_line,
                            character: end_character,
                        },
                    }
                })
                .collect(),
        }
    }
}

fn check_source(source: &str) -> CheckResponse {
    let parsed = nova_parser::parse(source);
    let checked = nova_sema::check(&parsed.program);
    let mut diagnostics = parsed.diagnostics;
    diagnostics.extend(checked.diagnostics);
    let index = LineIndex::new(source);
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    let warnings = diagnostics.iter().filter(|d| !d.is_error()).count();
    let functions = parsed
        .program
        .items
        .iter()
        .filter(|item| matches!(item, Item::Fn(_)))
        .count();
    let structs = parsed
        .program
        .items
        .iter()
        .filter(|item| matches!(item, Item::Struct(_)))
        .count();

    CheckResponse {
        ok: errors == 0,
        diagnostics: diagnostics
            .iter()
            .map(|diagnostic| DiagnosticView::from_diagnostic(diagnostic, &index, source))
            .collect(),
        stats: Stats {
            lines: source.lines().count(),
            bytes: source.len(),
            functions,
            structs,
            errors,
            warnings,
        },
        version: env!("CARGO_PKG_VERSION"),
    }
}

static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Allocate a byte buffer owned by the caller.
///
/// The playground passes a UTF-8 source buffer in, calls [`nova_check`], then
/// releases the input with [`nova_free`].
#[no_mangle]
pub extern "C" fn nova_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    let Ok(layout) = Layout::from_size_align(len, 1) else {
        return std::ptr::null_mut();
    };
    // SAFETY: the caller owns the returned allocation and frees it with
    // `nova_free` using the same length.
    unsafe { alloc(layout) }
}

/// Free a buffer previously returned by [`nova_alloc`].
#[no_mangle]
pub extern "C" fn nova_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let Ok(layout) = Layout::from_size_align(len, 1) else {
        return;
    };
    // SAFETY: pointer and length are the pair returned by `nova_alloc`.
    unsafe { dealloc(ptr, layout) };
}

/// Check UTF-8 Nova source and return a pointer to the JSON response.
///
/// The response is retained until the next call. JavaScript reads
/// [`nova_result_len`] bytes from the returned pointer.
#[no_mangle]
pub extern "C" fn nova_check(ptr: *const u8, len: usize) -> *const u8 {
    // SAFETY: a zero-length input is represented by an empty slice; otherwise
    // the caller provided a valid allocation of `len` bytes.
    let input = if len == 0 {
        &[][..]
    } else {
        unsafe { slice::from_raw_parts(ptr, len) }
    };
    let source = String::from_utf8_lossy(input);
    let response = check_source(&source);
    let json = serde_json::to_vec(&response).unwrap_or_else(|_| {
        br#"{"ok":false,"diagnostics":[],"stats":{"lines":0,"bytes":0,"functions":0,"structs":0,"errors":1,"warnings":0},"version":"error"}"#.to_vec()
    });
    // The result stays alive until the next check so JavaScript can copy it
    // out of linear memory. The mutex keeps the exported ABI sound even if the
    // module is later instantiated in a threaded host.
    let mut output = OUTPUT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *output = json;
    output.as_ptr()
}

/// Length in bytes of the most recent [`nova_check`] response.
#[no_mangle]
pub extern "C" fn nova_result_len() -> usize {
    OUTPUT.lock().map(|output| output.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_source_has_no_errors() {
        let result = check_source("fn main() {\n    let x = 1\n}\n");
        assert!(result.ok);
        assert_eq!(result.stats.errors, 0);
        assert_eq!(result.stats.functions, 1);
    }

    #[test]
    fn invalid_source_reports_a_diagnostic() {
        let result = check_source("fn main() { let x: Int = \"no\" }");
        assert!(!result.ok);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0308"));
    }

    #[test]
    fn formatter_is_exposed() {
        let result = check_source("fn main() {\nlet x = 1\n}\n");
        assert!(result.formatted.contains("    let x = 1"));
    }
}
