//! Conservative Nova source formatter.
//!
//! This milestone normalises indentation, trailing whitespace, blank-line runs
//! and the final newline while preserving every token, comment and string
//! exactly. It deliberately does not reflow expressions yet: a lossless
//! token-aware pretty-printer is planned. This makes `nova fmt` safe and
//! idempotent on the whole language.

pub fn format_source(source: &str) -> String {
    let mut state = State::default();
    let mut depth: i32 = 0;
    let mut out: Vec<String> = Vec::new();
    let mut blank_run = false;

    for raw_line in source.split('\n') {
        let raw_line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if state.in_string || state.in_block_comment {
            out.push(raw_line.to_string());
            scan(raw_line, &mut state, &mut depth);
            blank_run = raw_line.trim().is_empty();
            continue;
        }
        let trimmed_end = raw_line.trim_end();
        let content = trimmed_end.trim_start();
        if content.is_empty() {
            if !blank_run && !out.is_empty() {
                out.push(String::new());
            }
            blank_run = true;
            continue;
        }
        blank_run = false;
        let leading_closes = content.chars().take_while(|c| *c == '}').count() as i32;
        let indent = (depth - leading_closes).max(0) as usize;
        out.push(format!("{}{}", "    ".repeat(indent), content));
        scan(content, &mut state, &mut depth);
    }

    while out.last().map(|l| l.is_empty()).unwrap_or(false) {
        out.pop();
    }
    if out.is_empty() {
        String::new()
    } else {
        let mut result = out.join("\n");
        result.push('\n');
        result
    }
}

pub fn is_formatted(source: &str) -> bool {
    format_source(source) == source
}

#[derive(Default)]
struct State {
    in_block_comment: bool,
    block_depth: usize,
    in_string: bool,
    in_char: bool,
    escape: bool,
}

fn scan(line: &str, state: &mut State, depth: &mut i32) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if state.block_depth > 0 {
            if c == '/' && next == Some('*') {
                state.block_depth += 1;
                i += 2;
                continue;
            }
            if c == '*' && next == Some('/') {
                state.block_depth -= 1;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if state.in_string {
            if state.escape {
                state.escape = false;
            } else if c == '\\' {
                state.escape = true;
            } else if c == '"' {
                state.in_string = false;
            }
            i += 1;
            continue;
        }
        if state.in_char {
            if state.escape {
                state.escape = false;
            } else if c == '\\' {
                state.escape = true;
            } else if c == '\'' {
                state.in_char = false;
            }
            i += 1;
            continue;
        }
        match c {
            '/' if next == Some('/') => break,
            '/' if next == Some('*') => {
                state.block_depth = 1;
                i += 2;
            }
            '"' => {
                state.in_string = true;
                i += 1;
            }
            '\'' => {
                state.in_char = true;
                i += 1;
            }
            '{' => {
                *depth += 1;
                i += 1;
            }
            '}' => {
                *depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    state.in_block_comment = state.block_depth > 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reindents_and_keeps_content() {
        let src = "fn main() {\nlet x = 1\nif x == 1 {\nprintln(\"hi\")\n}\n}\n";
        let formatted = format_source(src);
        assert_eq!(
            formatted,
            "fn main() {\n    let x = 1\n    if x == 1 {\n        println(\"hi\")\n    }\n}\n"
        );
        assert_eq!(format_source(&formatted), formatted);
    }

    #[test]
    fn preserves_comments() {
        let src = "fn main() {\n// line comment\nlet x = 1 /* block */\n}\n";
        let formatted = format_source(src);
        assert!(formatted.contains("// line comment"));
        assert!(formatted.contains("/* block */"));
    }

    #[test]
    fn ignores_braces_in_strings_and_comments() {
        let src = "fn main() {\nlet s = \"{ not a block }\"\n/* } */\nprintln(s)\n}\n";
        let formatted = format_source(src);
        assert!(formatted.contains("let s = \"{ not a block }\""));
        assert!(formatted.contains("    /* } */"));
        assert_eq!(format_source(&formatted), formatted);
    }

    #[test]
    fn multiline_strings_are_untouched() {
        let src = "fn main() {\nlet s = \"line one\n    line two {\"\nprintln(s)\n}\n";
        let formatted = format_source(src);
        assert!(formatted.contains("let s = \"line one\n    line two {\""));
    }

    #[test]
    fn collapses_blank_lines_and_adds_final_newline() {
        let src = "fn main() {\n\n\n\nprintln(\"x\")\n}";
        let formatted = format_source(src);
        assert!(!formatted.contains("\n\n\n"));
        assert!(formatted.ends_with("}\n"));
    }

    #[test]
    fn empty_source_stays_empty() {
        assert_eq!(format_source(""), "");
        assert_eq!(format_source("\n\n"), "");
    }
}
