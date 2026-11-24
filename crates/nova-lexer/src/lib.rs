//! Nova lexer: converts UTF-8 source into tokens with byte spans.
//!
//! Newlines are preserved as [`TokenKind::Newline`] tokens so that the parser
//! can implement newline-aware statement termination. Comments (including
//! nested block comments) are skipped.
//!
//! String literals are stored **raw** (escape sequences are not interpreted
//! here). The parser splits `${...}` interpolation and decodes escapes, which
//! keeps the lexer independent of expression syntax.

use nova_diag::{Diagnostic, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int(i64),
    Float(f64),
    /// Raw contents between the quotes, escapes untouched.
    Str(String),
    Char(char),

    // Keywords
    KwFn,
    KwLet,
    KwVar,
    KwReturn,
    KwIf,
    KwElse,
    KwWhile,
    KwFor,
    KwIn,
    KwMatch,
    KwStruct,
    KwEnum,
    KwImpl,
    KwTrait,
    KwDyn,
    KwImport,
    KwAs,
    KwPub,
    KwTrue,
    KwFalse,
    KwBreak,
    KwContinue,
    KwDefer,
    KwUnsafe,
    KwExtern,
    KwAsync,
    KwAwait,
    KwMacro,
    KwConst,

    // Punctuation and operators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semi,
    Dot,
    DotDot,
    DotDotEq,
    Arrow,
    FatArrow,
    Question,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Eq,
    EqEq,
    Bang,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    Caret,
    Tilde,
    At,
    Underscore,

    Newline,
    Eof,
    Error(String),
}

impl TokenKind {
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(name) => format!("identifier `{name}`"),
            TokenKind::Int(_) => "integer literal".into(),
            TokenKind::Float(_) => "float literal".into(),
            TokenKind::Str(_) => "string literal".into(),
            TokenKind::Char(_) => "character literal".into(),
            TokenKind::Newline => "newline".into(),
            TokenKind::Eof => "end of file".into(),
            TokenKind::Error(msg) => format!("invalid token ({msg})"),
            other => format!("`{}`", other.punct_text()),
        }
    }

    pub fn punct_text(&self) -> &'static str {
        match self {
            TokenKind::KwFn => "fn",
            TokenKind::KwLet => "let",
            TokenKind::KwVar => "var",
            TokenKind::KwReturn => "return",
            TokenKind::KwIf => "if",
            TokenKind::KwElse => "else",
            TokenKind::KwWhile => "while",
            TokenKind::KwFor => "for",
            TokenKind::KwIn => "in",
            TokenKind::KwMatch => "match",
            TokenKind::KwStruct => "struct",
            TokenKind::KwEnum => "enum",
            TokenKind::KwImpl => "impl",
            TokenKind::KwTrait => "trait",
            TokenKind::KwDyn => "dyn",
            TokenKind::KwImport => "import",
            TokenKind::KwAs => "as",
            TokenKind::KwPub => "pub",
            TokenKind::KwTrue => "true",
            TokenKind::KwFalse => "false",
            TokenKind::KwBreak => "break",
            TokenKind::KwContinue => "continue",
            TokenKind::KwDefer => "defer",
            TokenKind::KwUnsafe => "unsafe",
            TokenKind::KwExtern => "extern",
            TokenKind::KwAsync => "async",
            TokenKind::KwAwait => "await",
            TokenKind::KwMacro => "macro",
            TokenKind::KwConst => "const",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::Comma => ",",
            TokenKind::Colon => ":",
            TokenKind::Semi => ";",
            TokenKind::Dot => ".",
            TokenKind::DotDot => "..",
            TokenKind::DotDotEq => "..=",
            TokenKind::Arrow => "->",
            TokenKind::FatArrow => "=>",
            TokenKind::Question => "?",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::PlusEq => "+=",
            TokenKind::MinusEq => "-=",
            TokenKind::StarEq => "*=",
            TokenKind::SlashEq => "/=",
            TokenKind::PercentEq => "%=",
            TokenKind::Eq => "=",
            TokenKind::EqEq => "==",
            TokenKind::Bang => "!",
            TokenKind::BangEq => "!=",
            TokenKind::Lt => "<",
            TokenKind::LtEq => "<=",
            TokenKind::Gt => ">",
            TokenKind::GtEq => ">=",
            TokenKind::AmpAmp => "&&",
            TokenKind::PipePipe => "||",
            TokenKind::Amp => "&",
            TokenKind::Pipe => "|",
            TokenKind::Caret => "^",
            TokenKind::Tilde => "~",
            TokenKind::At => "@",
            TokenKind::Underscore => "_",
            TokenKind::Newline => "newline",
            TokenKind::Eof => "end of file",
            TokenKind::Error(_) => "invalid token",
            _ => "token",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }
}

#[derive(Debug, Default)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    base: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            base: 0,
            diagnostics: Vec::new(),
        }
    }

    /// Lex a fragment whose spans should be reported relative to a larger file.
    pub fn with_base(src: &'a str, base: usize) -> Self {
        Self {
            src,
            pos: 0,
            base,
            diagnostics: Vec::new(),
        }
    }

    pub fn run(mut self) -> LexOutput {
        let mut tokens = Vec::new();
        loop {
            if self.pos >= self.src.len() {
                let span = self.span(self.pos, self.pos);
                tokens.push(Token::new(TokenKind::Eof, span));
                break;
            }
            if let Some(token) = self.next_token() {
                tokens.push(token);
            }
        }
        LexOutput {
            tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(start + self.base, end + self.base)
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn peek_char(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_second_char(&self) -> Option<char> {
        let mut it = self.rest().chars();
        it.next();
        it.next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn next_token(&mut self) -> Option<Token> {
        let ch = self.peek_char()?;
        if ch == ' ' || ch == '\t' || ch == '\u{feff}' {
            self.bump();
            return None;
        }
        if ch == '\n' || ch == '\r' {
            return Some(self.lex_newline());
        }
        if ch == '/' && self.peek_second_char() == Some('/') {
            self.skip_line_comment();
            return None;
        }
        if ch == '/' && self.peek_second_char() == Some('*') {
            self.skip_block_comment();
            return None;
        }
        if is_ident_start(ch) {
            return Some(self.lex_ident());
        }
        if ch.is_ascii_digit() {
            return Some(self.lex_number());
        }
        if ch == '"' {
            return Some(self.lex_string());
        }
        if ch == '\'' {
            return Some(self.lex_char());
        }
        Some(self.lex_punct())
    }

    fn lex_newline(&mut self) -> Token {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == '\n' || ch == '\r' || ch == ' ' || ch == '\t' || ch == '\u{feff}' {
                self.bump();
            } else {
                break;
            }
        }
        Token::new(TokenKind::Newline, self.span(start, self.pos))
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            self.bump();
        }
    }

    fn skip_block_comment(&mut self) {
        let start = self.pos;
        self.bump();
        self.bump();
        let mut depth = 1usize;
        while depth > 0 {
            match (self.peek_char(), self.peek_second_char()) {
                (Some('/'), Some('*')) => {
                    self.bump();
                    self.bump();
                    depth += 1;
                }
                (Some('*'), Some('/')) => {
                    self.bump();
                    self.bump();
                    depth -= 1;
                }
                (Some(_), _) => {
                    self.bump();
                }
                (None, _) => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0002",
                            "unterminated block comment",
                            self.span(start, self.src.len()),
                        )
                        .with_note("block comments nest in Nova; every `/*` needs a matching `*/`"),
                    );
                    return;
                }
            }
        }
    }

    fn lex_ident(&mut self) -> Token {
        let start = self.pos;
        let mut chars = String::new();
        while let Some(ch) = self.peek_char() {
            if is_ident_continue(ch) {
                chars.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        let span = self.span(start, self.pos);
        let kind = match chars.as_str() {
            "fn" => TokenKind::KwFn,
            "let" => TokenKind::KwLet,
            "var" => TokenKind::KwVar,
            "return" => TokenKind::KwReturn,
            "if" => TokenKind::KwIf,
            "else" => TokenKind::KwElse,
            "while" => TokenKind::KwWhile,
            "for" => TokenKind::KwFor,
            "in" => TokenKind::KwIn,
            "match" => TokenKind::KwMatch,
            "struct" => TokenKind::KwStruct,
            "enum" => TokenKind::KwEnum,
            "impl" => TokenKind::KwImpl,
            "trait" => TokenKind::KwTrait,
            "dyn" => TokenKind::KwDyn,
            "import" => TokenKind::KwImport,
            "as" => TokenKind::KwAs,
            "pub" => TokenKind::KwPub,
            "true" => TokenKind::KwTrue,
            "false" => TokenKind::KwFalse,
            "break" => TokenKind::KwBreak,
            "continue" => TokenKind::KwContinue,
            "defer" => TokenKind::KwDefer,
            "unsafe" => TokenKind::KwUnsafe,
            "extern" => TokenKind::KwExtern,
            "async" => TokenKind::KwAsync,
            "await" => TokenKind::KwAwait,
            "macro" => TokenKind::KwMacro,
            "const" => TokenKind::KwConst,
            "_" => TokenKind::Underscore,
            _ => TokenKind::Ident(chars),
        };
        Token::new(kind, span)
    }

    fn lex_number(&mut self) -> Token {
        let start = self.pos;
        let mut radix = 10u32;
        let mut is_float = false;

        if self.peek_char() == Some('0') {
            match self.peek_second_char() {
                Some('x') | Some('X') => {
                    radix = 16;
                    self.bump();
                    self.bump();
                }
                Some('b') | Some('B') => {
                    radix = 2;
                    self.bump();
                    self.bump();
                }
                Some('o') | Some('O') => {
                    radix = 8;
                    self.bump();
                    self.bump();
                }
                _ => {}
            }
        }

        let digits_start = self.pos;
        while let Some(ch) = self.peek_char() {
            let valid = match radix {
                16 => ch.is_ascii_hexdigit() || ch == '_',
                2 => ch == '0' || ch == '1' || ch == '_',
                8 => ('0'..='7').contains(&ch) || ch == '_',
                _ => ch.is_ascii_digit() || ch == '_',
            };
            if valid {
                self.bump();
            } else {
                break;
            }
        }
        let mut raw = self.src[digits_start..self.pos].to_string();

        if radix == 10 {
            // A fraction only counts if a digit follows the dot, so `0..5` and
            // `1.foo()` remain integer/method syntax.
            if self.peek_char() == Some('.')
                && self
                    .peek_second_char()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            {
                is_float = true;
                self.bump();
                while let Some(ch) = self.peek_char() {
                    if ch.is_ascii_digit() || ch == '_' {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            if let Some('e') | Some('E') = self.peek_char() {
                let saved = self.pos;
                self.bump();
                if let Some('+') | Some('-') = self.peek_char() {
                    self.bump();
                }
                let exp_start = self.pos;
                while let Some(ch) = self.peek_char() {
                    if ch.is_ascii_digit() {
                        self.bump();
                    } else {
                        break;
                    }
                }
                if self.pos == exp_start {
                    // Not an exponent; restore and let it become an identifier.
                    self.pos = saved;
                } else {
                    is_float = true;
                }
            }
            raw = self.src[digits_start..self.pos].to_string();
        }

        let span = self.span(start, self.pos);
        let clean = raw.replace('_', "");
        if clean.is_empty() {
            self.diagnostics.push(Diagnostic::error(
                "E0003",
                "number literal has no digits",
                span,
            ));
            return Token::new(TokenKind::Error("empty number".into()), span);
        }
        if is_float {
            match clean.parse::<f64>() {
                Ok(value) if value.is_finite() => Token::new(TokenKind::Float(value), span),
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        "E0004",
                        format!("invalid float literal `{raw}`"),
                        span,
                    ));
                    Token::new(TokenKind::Error("invalid float".into()), span)
                }
            }
        } else {
            match i64::from_str_radix(&clean, radix) {
                Ok(value) => Token::new(TokenKind::Int(value), span),
                Err(_) => {
                    self.diagnostics.push(Diagnostic::error(
                        "E0005",
                        format!("integer literal `{raw}` does not fit in Int (i64)"),
                        span,
                    ));
                    Token::new(TokenKind::Error("integer overflow".into()), span)
                }
            }
        }
    }

    fn lex_string(&mut self) -> Token {
        let start = self.pos;
        self.bump(); // opening quote
        let content_start = self.pos;
        loop {
            match self.peek_char() {
                None => {
                    let span = self.span(start, self.pos);
                    self.diagnostics.push(
                        Diagnostic::error("E0006", "unterminated string literal", span)
                            .with_note("add a closing `\"`"),
                    );
                    return Token::new(TokenKind::Error("unterminated string".into()), span);
                }
                Some('\\') => {
                    self.bump();
                    self.bump();
                }
                Some('"') => {
                    let raw = self.src[content_start..self.pos].to_string();
                    self.bump();
                    return Token::new(TokenKind::Str(raw), self.span(start, self.pos));
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    fn lex_char(&mut self) -> Token {
        let start = self.pos;
        self.bump(); // opening quote
        let value = match self.peek_char() {
            None => None,
            Some('\\') => {
                self.bump();
                self.lex_escape()
            }
            Some('\'') => {
                // Empty char literal.
                None
            }
            Some(ch) => {
                self.bump();
                Some(ch)
            }
        };
        let closed = if self.peek_char() == Some('\'') {
            self.bump();
            true
        } else {
            false
        };
        let span = self.span(start, self.pos);
        match value {
            Some(ch) if closed => Token::new(TokenKind::Char(ch), span),
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "E0007",
                    "invalid character literal",
                    span,
                ));
                Token::new(TokenKind::Error("invalid char".into()), span)
            }
        }
    }

    fn lex_escape(&mut self) -> Option<char> {
        let ch = self.bump()?;
        match ch {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            '0' => Some('\0'),
            '\\' => Some('\\'),
            '\'' => Some('\''),
            '"' => Some('"'),
            '$' => Some('$'),
            'u' => {
                if self.peek_char() != Some('{') {
                    return None;
                }
                self.bump();
                let mut hex = String::new();
                while let Some(c) = self.peek_char() {
                    if c == '}' {
                        break;
                    }
                    hex.push(c);
                    self.bump();
                }
                if self.peek_char() != Some('}') {
                    return None;
                }
                self.bump();
                u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
            }
            _ => None,
        }
    }

    fn lex_punct(&mut self) -> Token {
        let start = self.pos;
        let ch = self.bump().unwrap();
        let kind = match ch {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semi,
            '?' => TokenKind::Question,
            '+' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.peek_char() == Some('>') {
                    self.bump();
                    TokenKind::Arrow
                } else if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }
            '=' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::EqEq
                } else if self.peek_char() == Some('>') {
                    self.bump();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                if self.peek_char() == Some('&') {
                    self.bump();
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Amp
                }
            }
            '|' => {
                if self.peek_char() == Some('|') {
                    self.bump();
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            '@' => TokenKind::At,
            '.' => {
                if self.peek_char() == Some('.') {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            _ => {
                let span = self.span(start, self.pos);
                self.diagnostics.push(Diagnostic::error(
                    "E0008",
                    format!("unexpected character `{ch}`"),
                    span,
                ));
                TokenKind::Error(ch.to_string())
            }
        };
        Token::new(kind, self.span(start, self.pos))
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

pub fn lex(source: &str) -> LexOutput {
    Lexer::new(source).run()
}

pub fn lex_with_base(source: &str, base: usize) -> LexOutput {
    Lexer::with_base(source, base).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src)
            .tokens
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Newline | TokenKind::Eof))
            .collect()
    }

    #[test]
    fn lexes_keywords_identifiers_and_punctuation() {
        let tokens = kinds("fn main() { let x = 1 + 2 }");
        assert_eq!(tokens[0], TokenKind::KwFn);
        assert_eq!(tokens[1], TokenKind::Ident("main".into()));
        assert_eq!(tokens[2], TokenKind::LParen);
        assert_eq!(tokens[5], TokenKind::KwLet);
        assert_eq!(tokens[7], TokenKind::Eq);
        assert_eq!(tokens[8], TokenKind::Int(1));
        assert_eq!(tokens[9], TokenKind::Plus);
    }

    #[test]
    fn numbers_underscores_and_radices() {
        assert_eq!(kinds("1_000"), vec![TokenKind::Int(1000)]);
        assert_eq!(kinds("0xff"), vec![TokenKind::Int(255)]);
        assert_eq!(kinds("0b1010"), vec![TokenKind::Int(10)]);
        assert_eq!(kinds("1.5e3"), vec![TokenKind::Float(1500.0)]);
        // Range must not be eaten as a float.
        assert_eq!(
            kinds("1..5"),
            vec![TokenKind::Int(1), TokenKind::DotDot, TokenKind::Int(5)]
        );
    }

    #[test]
    fn comments_are_skipped_and_nested() {
        let src = "1 /* outer /* inner */ done */ 2 // trailing\n3";
        assert_eq!(
            kinds(src),
            vec![TokenKind::Int(1), TokenKind::Int(2), TokenKind::Int(3)]
        );
    }

    #[test]
    fn string_keeps_raw_escape_sequences() {
        let tokens = kinds(r#""a\n${name}!""#);
        assert_eq!(tokens, vec![TokenKind::Str("a\\n${name}!".into())]);
    }

    #[test]
    fn unicode_identifiers() {
        assert_eq!(
            kinds("let café = 1"),
            vec![
                TokenKind::KwLet,
                TokenKind::Ident("café".into()),
                TokenKind::Eq,
                TokenKind::Int(1)
            ]
        );
    }

    #[test]
    fn reports_unterminated_comment() {
        let out = lex("/* nope");
        assert!(out.diagnostics.iter().any(|d| d.code == "E0002"));
    }

    #[test]
    fn reports_unterminated_string() {
        let out = lex("\"nope");
        assert!(out.diagnostics.iter().any(|d| d.code == "E0006"));
    }

    #[test]
    fn spans_include_base_offset() {
        let out = lex_with_base("let x", 100);
        assert_eq!(out.tokens[0].span.start, 100);
        assert_eq!(out.tokens[1].span.start, 104);
    }

    #[test]
    fn newline_tokens_are_preserved() {
        let out = lex("a\nb");
        assert!(out
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Newline)));
    }
}
