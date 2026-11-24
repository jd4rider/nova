//! Shared diagnostics, source spans and position mapping for all Nova tools.
//!
//! All offsets stored in [`Span`] are **UTF-8 byte offsets** into the original
//! source. Tools that speak LSP convert to/from UTF-16 positions using
//! [`LineIndex`].

use serde::Serialize;

/// A half-open byte range in a UTF-8 source file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }

    pub fn contains(self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

/// A related location attached to a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

/// A diagnostic with a stable code, a primary span and optional notes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable code, e.g. `E0001` for lexical errors, `E0308` for type errors.
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            span,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            message: message.into(),
            span,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Start offset of every line in a source file.
#[derive(Clone, Debug)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0usize];
        for (idx, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(idx + 1);
            }
        }
        Self {
            starts,
            len: source.len(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.starts.get(line).copied()
    }

    /// Zero-based line and Unicode-scalar column (used for human rendering).
    pub fn line_col(&self, source: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.len);
        let line = self.line_of(offset);
        let start = self.starts[line];
        let col = source
            .get(start..offset)
            .map(|s| s.chars().count())
            .unwrap_or(0);
        (line, col)
    }

    /// Zero-based line of a byte offset.
    pub fn line_of(&self, offset: usize) -> usize {
        self.starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1)
    }

    /// Zero-based line and UTF-16 character offset (LSP wire format).
    pub fn utf16_position(&self, source: &str, offset: usize) -> (u32, u32) {
        let offset = offset.min(self.len);
        let line = self.line_of(offset);
        let start = self.starts[line];
        let character = source
            .get(start..offset)
            .map(|s| s.encode_utf16().count())
            .unwrap_or(0);
        (line as u32, character as u32)
    }

    /// Byte offset for a zero-based line and UTF-16 character offset.
    ///
    /// Returns `None` for an out-of-range line or a character offset that
    /// splits a UTF-16 surrogate pair.
    pub fn offset_at_utf16(&self, source: &str, line: u32, character: u32) -> Option<usize> {
        let start = *self.starts.get(line as usize)?;
        let end = self
            .starts
            .get(line as usize + 1)
            .copied()
            .unwrap_or(self.len);
        let text = source.get(start..end)?;
        let mut units = 0u32;
        for (byte_idx, ch) in text.char_indices() {
            if units == character {
                return Some(start + byte_idx);
            }
            let width = ch.len_utf16() as u32;
            if units + width > character {
                // Position points inside a surrogate pair: reject rather than guess.
                return None;
            }
            units += width;
        }
        if units == character {
            // Positions past the last character clamp to the end of the line
            // (excluding a trailing newline, as LSP permits).
            let trimmed = text
                .strip_suffix('\n')
                .map(|t| t.strip_suffix('\r').unwrap_or(t))
                .unwrap_or(text);
            return Some(start + trimmed.len());
        }
        None
    }
}

/// Render a diagnostic with a caret and source context (plain text, no colour).
pub fn render(source_name: &str, source: &str, diag: &Diagnostic) -> String {
    let index = LineIndex::new(source);
    let (line, col) = index.line_col(source, diag.span.start);
    let mut out = String::new();
    out.push_str(&format!(
        "{}[{}]: {}\n",
        diag.severity.as_str(),
        diag.code,
        diag.message
    ));
    out.push_str(&format!("  --> {}:{}:{}\n", source_name, line + 1, col + 1));

    let line_start = index.starts[line];
    let line_end = source[line_start..]
        .find('\n')
        .map(|i| line_start + i)
        .unwrap_or(source.len());
    let line_text = &source[line_start..line_end];
    let gutter = format!("{}", line + 1);
    let pad = " ".repeat(gutter.len());
    out.push_str(&format!("{} |\n", pad));
    out.push_str(&format!(
        "{} | {}\n",
        gutter,
        line_text.replace('\t', "    ")
    ));

    // Caret range, clamped to the first line of a multi-line span.
    let end = diag.span.end.min(line_end).max(diag.span.start);
    let caret_start = source
        .get(line_start..diag.span.start)
        .map(|s| s.replace('\t', "    ").chars().count())
        .unwrap_or(0);
    let caret_len = source
        .get(diag.span.start..end)
        .map(|s| s.replace('\t', "    ").chars().count())
        .unwrap_or(0)
        .max(1);
    out.push_str(&format!(
        "{} | {}{}\n",
        pad,
        " ".repeat(caret_start),
        "^".repeat(caret_len)
    ));

    for label in &diag.labels {
        let (lline, lcol) = index.line_col(source, label.span.start);
        if lline != line {
            out.push_str(&format!(
                "{} = note: {} (at {}:{}:{})\n",
                pad,
                label.message,
                source_name,
                lline + 1,
                lcol + 1
            ));
        } else {
            out.push_str(&format!("{} = note: {}\n", pad, label.message));
        }
    }
    for note in &diag.notes {
        out.push_str(&format!("{} = note: {}\n", pad, note));
    }
    out
}

/// Render every diagnostic in order.
pub fn render_all(source_name: &str, source: &str, diags: &[Diagnostic]) -> String {
    let mut out = String::new();
    for diag in diags {
        out.push_str(&render(source_name, source, diag));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_roundtrip_with_emoji() {
        let src = "let s = \"😀\";\nlet x = 1\n";
        let index = LineIndex::new(src);
        let offset = src.find(';').unwrap();
        let (line, ch) = index.utf16_position(src, offset);
        assert_eq!(line, 0);
        assert_eq!(index.offset_at_utf16(src, line, ch), Some(offset));
    }

    #[test]
    fn rejects_position_inside_surrogate_pair() {
        let src = "\"😀\"";
        let index = LineIndex::new(src);
        // The emoji starts at byte 1 and occupies UTF-16 units 1 and 2;
        // character 2 falls inside the surrogate pair.
        assert_eq!(index.offset_at_utf16(src, 0, 1), Some(1));
        assert_eq!(index.offset_at_utf16(src, 0, 2), None);
        assert_eq!(index.offset_at_utf16(src, 0, 3), Some(5));
    }

    #[test]
    fn render_has_caret() {
        let src = "fn main() {\n    leet x = 1\n}\n";
        let start = src.find("leet").unwrap();
        let diag = Diagnostic::error("E0001", "unknown keyword", Span::new(start, start + 4));
        let text = render("main.nova", src, &diag);
        assert!(text.contains("main.nova:2:5"));
        assert!(text.contains("^^^^"));
    }

    #[test]
    fn spans_merge() {
        assert_eq!(Span::new(2, 3).merge(Span::new(7, 9)), Span::new(2, 9));
    }
}
