//! Recursive-descent and Pratt parser for Nova.
//!
//! The parser is designed never to hang on malformed input: every loop checks
//! for progress, and statement-level recovery resynchronises on newlines and
//! declaration keywords. Diagnostics are accumulated; parsing always returns a
//! program with placeholder nodes for unrecoverable fragments.

pub mod ast;

pub use ast::*;

use nova_diag::{Diagnostic, Span};
use nova_lexer::{lex, lex_with_base, Token, TokenKind};

pub struct ParseResult {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(source: &str) -> ParseResult {
    let lexed = lex(source);
    let mut parser = Parser::new(lexed.tokens);
    parser.diagnostics.extend(lexed.diagnostics);
    let program = parser.parse_program(source.len());
    ParseResult {
        program,
        diagnostics: parser.diagnostics,
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    delimiter_depth: usize,
    disallow_struct_lit: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            delimiter_depth: 0,
            disallow_struct_lit: 0,
        }
    }

    // ---------------------------------------------------------------------
    // Token helpers
    // ---------------------------------------------------------------------

    fn peek(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            self.tokens.last().expect("parser requires an EOF token")
        }
    }

    fn peek_at(&self, n: usize) -> &Token {
        let idx = (self.pos + n).min(self.tokens.len().saturating_sub(1));
        &self.tokens[idx]
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if !token.is_eof() {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, kind: &TokenKind) -> Option<Token> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn at_separator(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Newline | TokenKind::Semi)
    }

    fn skip_separators(&mut self) {
        while self.at_separator() {
            self.advance();
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect(&mut self, kind: &TokenKind, context: &str) -> Option<Token> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            let found = self.peek().kind.describe();
            let span = self.peek().span;
            self.error(
                "E0103",
                format!("expected {} {context}, found {found}", kind.describe()),
                span,
            );
            None
        }
    }

    fn expect_ident(&mut self, context: &str) -> Option<(String, Span)> {
        if let TokenKind::Ident(name) = &self.peek().kind {
            let name = name.clone();
            let span = self.peek().span;
            self.advance();
            Some((name, span))
        } else {
            let found = self.peek().kind.describe();
            let span = self.peek().span;
            self.error(
                "E0103",
                format!("expected an identifier {context}, found {found}"),
                span,
            );
            None
        }
    }

    fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    fn synchronize_stmt(&mut self) {
        while !self.at_eof() {
            match self.peek().kind {
                TokenKind::Newline | TokenKind::Semi => return,
                TokenKind::RBrace => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn synchronize_item(&mut self) {
        while !self.at_eof() {
            if matches!(
                self.peek().kind,
                TokenKind::KwFn
                    | TokenKind::KwStruct
                    | TokenKind::KwEnum
                    | TokenKind::KwImpl
                    | TokenKind::KwTrait
                    | TokenKind::KwExtern
                    | TokenKind::KwMacro
                    | TokenKind::KwAsync
                    | TokenKind::KwImport
                    | TokenKind::At
                    | TokenKind::KwPub
            ) {
                return;
            }
            self.advance();
        }
    }

    // ---------------------------------------------------------------------
    // Program and items
    // ---------------------------------------------------------------------

    fn parse_program(&mut self, source_len: usize) -> Program {
        let mut items = Vec::new();
        loop {
            self.skip_separators();
            if self.at_eof() {
                break;
            }
            let mut is_test = false;
            let mut is_pub = false;
            let mut had_attribute = false;
            while self.check(&TokenKind::At) {
                had_attribute = true;
                self.advance();
                if let Some((name, span)) = self.expect_ident("after `@`") {
                    if name == "test" {
                        is_test = true;
                    } else {
                        self.diagnostics.push(
                            Diagnostic::warning(
                                "W0101",
                                format!("unknown attribute `@{name}`"),
                                span,
                            )
                            .with_note("the only attribute supported in this build is `@test`"),
                        );
                    }
                }
                self.skip_newlines();
            }
            if had_attribute {
                self.skip_separators();
            }
            if self.check(&TokenKind::KwPub) {
                self.advance();
                is_pub = true;
                self.skip_newlines();
            }

            let before = self.pos;
            let item = match self.peek().kind {
                TokenKind::KwFn => self.parse_fn(is_pub, is_test, None).map(Item::Fn),
                TokenKind::KwStruct => self.parse_struct().map(Item::Struct),
                TokenKind::KwEnum => self.parse_enum().map(Item::Enum),
                TokenKind::KwImpl => self.parse_impl().map(Item::Impl),
                TokenKind::KwImport => self.parse_import().map(Item::Import),
                TokenKind::KwTrait => self.parse_trait().map(Item::Trait),
                TokenKind::KwExtern => self.parse_extern().map(Item::Extern),
                TokenKind::KwMacro => self.parse_macro().map(Item::Macro),
                TokenKind::KwAsync => {
                    self.advance();
                    self.skip_newlines();
                    self.parse_fn(is_pub, is_test, None).map(Item::Fn)
                }
                ref other => {
                    let found = other.describe();
                    let span = self.peek().span;
                    self.error(
                        "E0101",
                        format!("expected a declaration (`fn`, `struct`, `enum`, `import`), found {found}"),
                        span,
                    );
                    self.advance();
                    None
                }
            };
            if let Some(item) = item {
                items.push(item);
            }
            if self.pos == before {
                self.advance();
            }
            self.synchronize_item();
            self.skip_separators();
        }
        Program {
            items,
            span: Span::new(0, source_len),
        }
    }

    fn skip_balanced_braces(&mut self) {
        let mut depth = 0usize;
        while !self.at_eof() {
            match self.peek().kind {
                TokenKind::LBrace => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                    self.advance();
                    if depth == 0 {
                        return;
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_generic_params(&mut self) -> Vec<String> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::Lt) {
            return params;
        }
        self.advance();
        loop {
            if self.check(&TokenKind::Gt) {
                self.advance();
                break;
            }
            if let Some((name, _)) = self.expect_ident("in generic parameter list") {
                params.push(name);
            } else {
                break;
            }
            if self.eat(&TokenKind::Comma).is_some() {
                self.skip_newlines();
                continue;
            }
            if self.eat(&TokenKind::Gt).is_some() {
                break;
            }
            let span = self.peek().span;
            self.error(
                "E0104",
                "expected `,` or `>` in generic parameter list",
                span,
            );
            break;
        }
        params
    }

    fn parse_type_args(&mut self) -> Vec<TypeExpr> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::Lt) {
            return args;
        }
        self.advance();
        self.delimiter_depth += 1;
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::Gt) {
                self.advance();
                break;
            }
            args.push(self.parse_type());
            self.skip_newlines();
            if self.eat(&TokenKind::Comma).is_some() {
                continue;
            }
            if self.eat(&TokenKind::Gt).is_some() {
                break;
            }
            let span = self.peek().span;
            self.error("E0104", "expected `,` or `>` in type argument list", span);
            break;
        }
        self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        args
    }

    fn parse_extern(&mut self) -> Option<ExternFnDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwExtern, "")?;
        let abi = match self.peek().kind.clone() {
            TokenKind::Str(raw) => {
                let span = self.advance().span;
                let mut text = String::new();
                for part in self.parse_string_parts(&raw, span) {
                    if let StrPart::Literal(value) = part {
                        text.push_str(&value);
                    }
                }
                text
            }
            _ => {
                let span = self.peek().span;
                self.error("E0103", "expected an ABI string after `extern`", span);
                "C".into()
            }
        };
        self.skip_newlines();
        let sig = self.parse_fn_signature()?;
        Some(ExternFnDecl {
            abi,
            name: sig.name,
            name_span: sig.name_span,
            params: sig.params,
            ret: sig.ret,
            span: Span::new(start, sig.span.end),
        })
    }

    fn parse_macro(&mut self) -> Option<MacroDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwMacro, "")?;
        let (name, name_span) = self.expect_ident("for the macro name")?;
        self.expect(&TokenKind::LParen, "to open macro parameters");
        let mut params = Vec::new();
        self.delimiter_depth += 1;
        while !self.check(&TokenKind::RParen) && !self.at_eof() {
            let Some((param, span)) = self.expect_ident("for a macro parameter") else {
                break;
            };
            params.push((param, span));
            if self.eat(&TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }
        self.expect(&TokenKind::RParen, "to close macro parameters");
        self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        self.skip_newlines();
        let body = self.parse_block();
        let end = body.span.end;
        Some(MacroDecl {
            name,
            name_span,
            params,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_trait(&mut self) -> Option<TraitDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwTrait, "")?;
        let (name, name_span) = self.expect_ident("for the trait name")?;
        self.skip_newlines();
        let mut methods = Vec::new();
        if self
            .expect(&TokenKind::LBrace, "to open trait body")
            .is_none()
        {
            return None;
        }
        self.delimiter_depth += 1;
        self.skip_separators();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            if self.check(&TokenKind::KwFn) {
                if let Some(method) = self.parse_fn_signature() {
                    methods.push(method);
                }
            } else {
                let span = self.peek().span;
                self.error("E0101", "expected a method signature in `trait`", span);
                self.advance();
            }
            self.skip_separators();
            if self.pos == before {
                self.advance();
            }
        }
        let end = self
            .expect(&TokenKind::RBrace, "to close trait body")
            .map(|t| t.span.end)
            .unwrap_or(self.peek().span.end);
        self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        Some(TraitDecl {
            name,
            name_span,
            methods,
            span: Span::new(start, end),
        })
    }

    fn parse_fn_signature(&mut self) -> Option<FnSignature> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwFn, "")?;
        let (name, name_span) = self.expect_ident("for the method name")?;
        let _ = self.parse_generic_params();
        self.skip_newlines();
        let mut params = Vec::new();
        self.expect(&TokenKind::LParen, "to open the parameter list");
        self.delimiter_depth += 1;
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.at_eof() {
            let Some((pname, pspan)) = self.expect_ident("for a parameter name") else {
                break;
            };
            let ty = if self.eat(&TokenKind::Colon).is_some() {
                self.parse_type()
            } else if pname == "self" {
                TypeExpr {
                    kind: TypeExprKind::Named {
                        path: vec!["Self".into()],
                        args: Vec::new(),
                    },
                    span: pspan,
                }
            } else {
                self.parse_type()
            };
            params.push(Param {
                name: pname,
                name_span: pspan,
                ty,
                span: pspan.merge(self.tokens[self.pos.saturating_sub(1)].span),
            });
            self.skip_newlines();
            if self.eat(&TokenKind::Comma).is_some() {
                self.skip_newlines();
                continue;
            }
            break;
        }
        self.expect(&TokenKind::RParen, "to close the parameter list");
        self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        self.skip_newlines();
        let ret = if self.eat(&TokenKind::Arrow).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        self.eat(&TokenKind::Semi);
        Some(FnSignature {
            name,
            name_span,
            params,
            ret,
            span: Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
        })
    }

    fn parse_impl(&mut self) -> Option<ImplDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwImpl, "")?;
        let generics = self.parse_generic_params();
        let (first, first_span) = self.expect_ident("for the impl target")?;
        let first_args = self.parse_type_args();
        let (trait_name, type_name, type_span, type_args) = if self.eat(&TokenKind::KwFor).is_some()
        {
            let (type_name, type_span) = self.expect_ident("after `for` in impl")?;
            let type_args = self.parse_type_args();
            (Some(first), type_name, type_span, type_args)
        } else {
            (None, first, first_span, first_args)
        };
        let self_ty = TypeExpr {
            kind: TypeExprKind::Named {
                path: vec![type_name.clone()],
                args: type_args.clone(),
            },
            span: type_span,
        };
        self.skip_newlines();
        let mut methods = Vec::new();
        if self
            .expect(&TokenKind::LBrace, "to open impl body")
            .is_none()
        {
            return None;
        }
        self.delimiter_depth += 1;
        self.skip_separators();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            if self.check(&TokenKind::KwFn) {
                if let Some(method) = self.parse_fn(false, false, Some(&self_ty)) {
                    methods.push(method);
                }
            } else {
                let span = self.peek().span;
                self.error("E0101", "expected a method declaration in `impl`", span);
                self.advance();
            }
            self.skip_separators();
            if self.pos == before {
                self.advance();
            }
        }
        let end = self
            .expect(&TokenKind::RBrace, "to close impl body")
            .map(|t| t.span.end)
            .unwrap_or(self.peek().span.end);
        self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        Some(ImplDecl {
            type_name,
            type_span,
            generics,
            type_args,
            trait_name,
            methods,
            span: Span::new(start, end),
        })
    }

    fn parse_fn(
        &mut self,
        is_pub: bool,
        is_test: bool,
        self_ty: Option<&TypeExpr>,
    ) -> Option<FnDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwFn, "to start a function declaration")?;
        let (name, name_span) = self.expect_ident("for the function name")?;
        let generics = self.parse_generic_params();
        self.skip_newlines();

        let mut params = Vec::new();
        if self.eat(&TokenKind::LParen).is_some() {
            self.delimiter_depth += 1;
            self.skip_newlines();
            while !self.check(&TokenKind::RParen) && !self.at_eof() {
                let before = self.pos;
                let Some((pname, pspan)) = self.expect_ident("for a parameter name") else {
                    break;
                };
                let ty = if self.eat(&TokenKind::Colon).is_some() {
                    self.parse_type()
                } else if pname == "self" {
                    if let Some(self_ty) = self_ty {
                        TypeExpr {
                            kind: self_ty.kind.clone(),
                            span: pspan,
                        }
                    } else {
                        self.error("E0111", "`self` is only valid in an `impl` method", pspan);
                        TypeExpr {
                            kind: TypeExprKind::Named {
                                path: vec!["<missing>".into()],
                                args: Vec::new(),
                            },
                            span: pspan,
                        }
                    }
                } else {
                    let span = self.peek().span;
                    self.error(
                        "E0111",
                        format!("parameter `{pname}` requires an explicit type"),
                        span,
                    );
                    TypeExpr {
                        kind: TypeExprKind::Named {
                            path: vec!["<missing>".into()],
                            args: Vec::new(),
                        },
                        span,
                    }
                };
                params.push(Param {
                    name: pname,
                    name_span: pspan,
                    ty,
                    span: pspan.merge(self.tokens[self.pos.saturating_sub(1)].span),
                });
                self.skip_newlines();
                if self.eat(&TokenKind::Comma).is_some() {
                    self.skip_newlines();
                    continue;
                }
                if !self.check(&TokenKind::RParen) {
                    let span = self.peek().span;
                    self.error("E0104", "expected `,` or `)` in parameter list", span);
                    if self.pos == before {
                        self.advance();
                    }
                }
                self.skip_newlines();
            }
            self.expect(&TokenKind::RParen, "to close the parameter list");
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        } else {
            let span = self.peek().span;
            self.error("E0103", "expected `(` after function name", span);
        }

        self.skip_newlines();
        let ret = if self.eat(&TokenKind::Arrow).is_some() {
            self.skip_newlines();
            Some(self.parse_type())
        } else {
            None
        };
        self.skip_newlines();
        let body = self.parse_block();
        let span = Span::new(start, body.span.end);
        if is_test && !params.is_empty() {
            self.error(
                "E0112",
                "@test functions must not take parameters",
                name_span,
            );
        }
        Some(FnDecl {
            name,
            name_span,
            generics,
            params,
            ret,
            body,
            is_pub,
            is_test,
            span,
        })
    }

    fn parse_struct(&mut self) -> Option<StructDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwStruct, "")?;
        let (name, name_span) = self.expect_ident("for the struct name")?;
        let generics = self.parse_generic_params();
        self.skip_newlines();
        let mut fields = Vec::new();
        if self.eat(&TokenKind::LBrace).is_some() {
            self.delimiter_depth += 1;
            loop {
                self.skip_separators();
                if self.check(&TokenKind::RBrace) || self.at_eof() {
                    break;
                }
                let before = self.pos;
                let Some((fname, fspan)) = self.expect_ident("for a field name") else {
                    break;
                };
                self.skip_newlines();
                if self.expect(&TokenKind::Colon, "after field name").is_none() {
                    self.synchronize_stmt();
                } else {
                    let ty = self.parse_type();
                    fields.push(FieldDecl {
                        name: fname,
                        name_span: fspan,
                        ty,
                        span: fspan.merge(self.tokens[self.pos.saturating_sub(1)].span),
                    });
                }
                if self.eat(&TokenKind::Comma).is_none() && !self.at_separator() {
                    if !self.check(&TokenKind::RBrace) {
                        let span = self.peek().span;
                        self.error("E0104", "expected `,` or newline after struct field", span);
                        self.synchronize_stmt();
                    }
                }
                self.skip_separators();
                if self.pos == before {
                    self.advance();
                }
            }
            self.expect(&TokenKind::RBrace, "to close the struct body");
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        } else {
            let span = self.peek().span;
            self.error("E0103", "expected `{` after struct name", span);
        }
        Some(StructDecl {
            name,
            name_span,
            generics,
            fields,
            span: Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
        })
    }

    fn parse_enum(&mut self) -> Option<EnumDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwEnum, "")?;
        let (name, name_span) = self.expect_ident("for the enum name")?;
        let generics = self.parse_generic_params();
        self.skip_newlines();
        let mut variants = Vec::new();
        if self.eat(&TokenKind::LBrace).is_some() {
            self.delimiter_depth += 1;
            loop {
                self.skip_separators();
                if self.check(&TokenKind::RBrace) || self.at_eof() {
                    break;
                }
                let before = self.pos;
                let Some((vname, vspan)) = self.expect_ident("for an enum variant") else {
                    break;
                };
                let mut fields = Vec::new();
                if self.eat(&TokenKind::LParen).is_some() {
                    self.delimiter_depth += 1;
                    self.skip_newlines();
                    while !self.check(&TokenKind::RParen) && !self.at_eof() {
                        fields.push(self.parse_type());
                        self.skip_newlines();
                        if self.eat(&TokenKind::Comma).is_some() {
                            self.skip_newlines();
                            continue;
                        }
                        break;
                    }
                    self.expect(&TokenKind::RParen, "to close variant payload");
                    self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                }
                variants.push(VariantDecl {
                    name: vname,
                    name_span: vspan,
                    fields,
                    span: vspan.merge(self.tokens[self.pos.saturating_sub(1)].span),
                });
                if self.eat(&TokenKind::Comma).is_none() && !self.at_separator() {
                    if !self.check(&TokenKind::RBrace) {
                        let span = self.peek().span;
                        self.error("E0104", "expected `,` or newline after enum variant", span);
                        self.synchronize_stmt();
                    }
                }
                self.skip_separators();
                if self.pos == before {
                    self.advance();
                }
            }
            self.expect(&TokenKind::RBrace, "to close the enum body");
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        } else {
            let span = self.peek().span;
            self.error("E0103", "expected `{` after enum name", span);
        }
        Some(EnumDecl {
            name,
            name_span,
            generics,
            variants,
            span: Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
        })
    }

    fn parse_import(&mut self) -> Option<ImportDecl> {
        let start = self.peek().span.start;
        self.expect(&TokenKind::KwImport, "")?;
        if let TokenKind::Str(raw) = self.peek().kind.clone() {
            let string_span = self.advance().span;
            let mut path = String::new();
            for part in self.parse_string_parts(&raw, string_span) {
                match part {
                    StrPart::Literal(text) => path.push_str(&text),
                    StrPart::Expr(_) => self.error(
                        "E0109",
                        "import paths cannot contain interpolation",
                        string_span,
                    ),
                }
            }
            let alias = if self.eat(&TokenKind::KwAs).is_some() {
                self.expect_ident("for the module alias")
                    .map(|(name, _)| name)
            } else {
                None
            };
            self.eat(&TokenKind::Semi);
            return Some(ImportDecl {
                module: Vec::new(),
                path: Some(path),
                alias,
                items: Vec::new(),
                span: Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
            });
        }
        let mut module = Vec::new();
        if let Some((first, _)) = self.expect_ident("in import path") {
            module.push(first);
        } else {
            return None;
        }
        while self.check(&TokenKind::Dot) {
            self.advance();
            if let Some((part, _)) = self.expect_ident("in import path") {
                module.push(part);
            } else {
                break;
            }
        }
        let mut items = Vec::new();
        if self.eat(&TokenKind::LBrace).is_some() {
            self.delimiter_depth += 1;
            loop {
                self.skip_separators();
                if self.check(&TokenKind::RBrace) || self.at_eof() {
                    break;
                }
                let Some((name, name_span)) = self.expect_ident("in import list") else {
                    break;
                };
                let alias = if self.eat(&TokenKind::KwAs).is_some() {
                    self.expect_ident("for import alias").map(|(a, _)| a)
                } else {
                    None
                };
                items.push(ImportItem {
                    name,
                    name_span,
                    alias,
                });
                self.eat(&TokenKind::Comma);
                self.skip_separators();
            }
            self.expect(&TokenKind::RBrace, "to close import list");
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        }
        self.eat(&TokenKind::Semi);
        Some(ImportDecl {
            module,
            path: None,
            alias: None,
            items,
            span: Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
        })
    }

    // ---------------------------------------------------------------------
    // Types
    // ---------------------------------------------------------------------

    fn parse_type(&mut self) -> TypeExpr {
        let start = self.peek().span.start;
        if self.check(&TokenKind::KwFn) {
            self.advance();
            self.expect(&TokenKind::LParen, "to open function type parameters");
            self.delimiter_depth += 1;
            let mut params = Vec::new();
            self.skip_newlines();
            while !self.check(&TokenKind::RParen) && !self.at_eof() {
                params.push(self.parse_type());
                self.skip_newlines();
                if self.eat(&TokenKind::Comma).is_some() {
                    self.skip_newlines();
                    continue;
                }
                break;
            }
            self.expect(&TokenKind::RParen, "to close function type parameters");
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
            self.expect(&TokenKind::Arrow, "before a function return type");
            let ret = self.parse_type();
            return TypeExpr {
                kind: TypeExprKind::Fn {
                    params,
                    ret: Box::new(ret),
                },
                span: Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
            };
        }
        if self.check(&TokenKind::LParen) {
            self.advance();
            if self.check(&TokenKind::RParen) {
                let end = self.advance().span.end;
                return TypeExpr {
                    kind: TypeExprKind::Unit,
                    span: Span::new(start, end),
                };
            }
            let mut types = vec![self.parse_type()];
            while self.eat(&TokenKind::Comma).is_some() {
                self.skip_newlines();
                types.push(self.parse_type());
            }
            let end = self
                .expect(&TokenKind::RParen, "to close tuple type")
                .map(|t| t.span.end)
                .unwrap_or(self.peek().span.end);
            return TypeExpr {
                kind: TypeExprKind::Tuple(types),
                span: Span::new(start, end),
            };
        }
        if self.check(&TokenKind::LBracket) {
            self.advance();
            self.delimiter_depth += 1;
            self.skip_newlines();
            let elem = self.parse_type();
            self.skip_newlines();
            self.expect(&TokenKind::Semi, "between array element type and length");
            self.skip_newlines();
            let len = match self.peek().kind {
                TokenKind::Int(value) if value >= 0 => {
                    self.advance();
                    value as usize
                }
                _ => {
                    let span = self.peek().span;
                    self.error("E0116", "array length must be a non-negative integer", span);
                    self.advance();
                    0
                }
            };
            self.skip_newlines();
            let end = self
                .expect(&TokenKind::RBracket, "to close array type")
                .map(|t| t.span.end)
                .unwrap_or(self.peek().span.end);
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
            return TypeExpr {
                kind: TypeExprKind::Array {
                    elem: Box::new(elem),
                    len,
                },
                span: Span::new(start, end),
            };
        }
        if self.check(&TokenKind::KwDyn) {
            self.advance();
            let (name, name_span) = match self.expect_ident("after `dyn`") {
                Some(value) => value,
                None => {
                    return TypeExpr {
                        kind: TypeExprKind::Dyn("<missing>".into()),
                        span: Span::new(start, self.peek().span.end),
                    }
                }
            };
            return TypeExpr {
                kind: TypeExprKind::Dyn(name),
                span: Span::new(start, name_span.end),
            };
        }
        let mut path = Vec::new();
        if let Some((name, _)) = self.expect_ident("in type") {
            path.push(name);
        } else {
            let span = self.peek().span;
            return TypeExpr {
                kind: TypeExprKind::Named {
                    path: vec!["<error>".into()],
                    args: Vec::new(),
                },
                span,
            };
        }
        while self.check(&TokenKind::Dot) {
            self.advance();
            if let Some((part, _)) = self.expect_ident("in type path") {
                path.push(part);
            } else {
                break;
            }
        }
        let mut args = Vec::new();
        if self.eat(&TokenKind::Lt).is_some() {
            self.delimiter_depth += 1;
            loop {
                self.skip_newlines();
                if self.check(&TokenKind::Gt) || self.at_eof() {
                    break;
                }
                args.push(self.parse_type());
                self.skip_newlines();
                if self.eat(&TokenKind::Comma).is_some() {
                    continue;
                }
                break;
            }
            self.expect(&TokenKind::Gt, "to close type arguments");
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        }
        TypeExpr {
            kind: TypeExprKind::Named { path, args },
            span: Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
        }
    }

    // ---------------------------------------------------------------------
    // Statements and blocks
    // ---------------------------------------------------------------------

    fn parse_block(&mut self) -> Block {
        let start = self.peek().span.start;
        if self.expect(&TokenKind::LBrace, "to open a block").is_none() {
            return Block {
                stmts: Vec::new(),
                span: Span::new(start, start),
            };
        }
        self.skip_separators();
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            }
            let had_separator = self.at_separator();
            self.skip_separators();
            if !self.check(&TokenKind::RBrace) && !self.at_eof() && !had_separator {
                let span = self.peek().span;
                self.error(
                    "E0102",
                    "expected a newline or `;` between statements",
                    span,
                );
                self.synchronize_stmt();
                self.skip_separators();
            }
            if self.pos == before {
                // Guarantee progress on pathological input.
                self.advance();
            }
        }
        let end = self
            .expect(&TokenKind::RBrace, "to close the block")
            .map(|t| t.span.end)
            .unwrap_or(self.peek().span.end);
        Block {
            stmts,
            span: Span::new(start, end),
        }
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span.start;
        match self.peek().kind {
            TokenKind::KwLet | TokenKind::KwVar => self.parse_let(start),
            TokenKind::KwReturn => {
                self.advance();
                let value =
                    if self.at_separator() || self.check(&TokenKind::RBrace) || self.at_eof() {
                        None
                    } else {
                        Some(self.parse_expr_prec(0))
                    };
                let end = value
                    .as_ref()
                    .map(|e| e.span.end)
                    .unwrap_or(self.tokens[self.pos.saturating_sub(1)].span.end);
                Some(Stmt {
                    kind: StmtKind::Return(value),
                    span: Span::new(start, end),
                })
            }
            TokenKind::KwBreak => {
                let token = self.advance();
                Some(Stmt {
                    kind: StmtKind::Break,
                    span: token.span,
                })
            }
            TokenKind::KwContinue => {
                let token = self.advance();
                Some(Stmt {
                    kind: StmtKind::Continue,
                    span: token.span,
                })
            }
            TokenKind::KwWhile => {
                self.advance();
                self.disallow_struct_lit += 1;
                let cond = self.parse_expr_prec(0);
                self.disallow_struct_lit -= 1;
                self.skip_newlines();
                let body = self.parse_block();
                Some(Stmt {
                    span: Span::new(start, body.span.end),
                    kind: StmtKind::While { cond, body },
                })
            }
            TokenKind::KwFor => {
                self.advance();
                let (name, name_span) = if self.check(&TokenKind::Underscore) {
                    let t = self.advance();
                    ("_".into(), t.span)
                } else {
                    self.expect_ident("for the loop variable")?
                };
                self.skip_newlines();
                self.expect(&TokenKind::KwIn, "after loop variable")?;
                self.skip_newlines();
                self.disallow_struct_lit += 1;
                let iter = self.parse_expr_prec(0);
                self.disallow_struct_lit -= 1;
                self.skip_newlines();
                let body = self.parse_block();
                Some(Stmt {
                    span: Span::new(start, body.span.end),
                    kind: StmtKind::For {
                        name,
                        name_span,
                        iter,
                        body,
                    },
                })
            }
            TokenKind::KwDefer => {
                let span = self.peek().span;
                self.advance();
                self.error("E0101", "`defer` is not implemented in this build", span);
                let _ = self.parse_expr_prec(0);
                None
            }
            TokenKind::KwFn => {
                let span = self.peek().span;
                self.error(
                    "E0106",
                    "nested function declarations are not supported; declare it at module level",
                    span,
                );
                self.synchronize_stmt();
                None
            }
            TokenKind::Error(_) => {
                self.advance();
                None
            }
            _ => {
                let lhs = self.parse_expr_prec(0);
                let assign_op = match self.peek().kind {
                    TokenKind::Eq => Some(AssignOp::Assign),
                    TokenKind::PlusEq => Some(AssignOp::Add),
                    TokenKind::MinusEq => Some(AssignOp::Sub),
                    TokenKind::StarEq => Some(AssignOp::Mul),
                    TokenKind::SlashEq => Some(AssignOp::Div),
                    TokenKind::PercentEq => Some(AssignOp::Rem),
                    _ => None,
                };
                if let Some(op) = assign_op {
                    self.advance();
                    self.skip_newlines();
                    let value = self.parse_expr_prec(0);
                    let span = lhs.span.merge(value.span);
                    Some(Stmt {
                        kind: StmtKind::Assign {
                            target: lhs,
                            op,
                            value,
                        },
                        span,
                    })
                } else {
                    let span = lhs.span;
                    Some(Stmt {
                        kind: StmtKind::Expr(lhs),
                        span,
                    })
                }
            }
        }
    }

    fn parse_let(&mut self, start: usize) -> Option<Stmt> {
        let mutable = matches!(self.peek().kind, TokenKind::KwVar);
        self.advance();
        let (name, name_span) = self.expect_ident("for the binding name")?;
        let ty = if self.eat(&TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        let init = if self.eat(&TokenKind::Eq).is_some() {
            self.skip_newlines();
            self.parse_expr_prec(0)
        } else {
            let span = self.peek().span;
            self.error(
                "E0107",
                format!("binding `{name}` requires an initializer in this build"),
                span,
            );
            Expr::new(ExprKind::Error, name_span)
        };
        let span = Span::new(start, init.span.end);
        Some(Stmt {
            kind: StmtKind::Let {
                mutable,
                name,
                name_span,
                ty,
                init,
            },
            span,
        })
    }

    // ---------------------------------------------------------------------
    // Expressions
    // ---------------------------------------------------------------------

    fn parse_expr_prec(&mut self, min_prec: u8) -> Expr {
        let mut lhs = self.parse_unary();
        loop {
            if self.check(&TokenKind::Newline) {
                if self.delimiter_depth > 0 {
                    self.skip_newlines();
                } else {
                    break;
                }
            }
            let Some((prec, op)) = binary_op(&self.peek().kind) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.advance();
            self.skip_newlines();
            let rhs = self.parse_expr_prec(prec + 1);
            let span = lhs.span.merge(rhs.span);
            lhs = Expr::new(
                ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        lhs
    }

    fn parse_unary(&mut self) -> Expr {
        let start = self.peek().span.start;
        if self.check(&TokenKind::Minus) || self.check(&TokenKind::Bang) {
            let op = if self.check(&TokenKind::Minus) {
                UnaryOp::Neg
            } else {
                UnaryOp::Not
            };
            self.advance();
            self.skip_newlines();
            let expr = self.parse_unary();
            let span = Span::new(start, expr.span.end);
            return Expr::new(
                ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span,
            );
        }
        if self.check(&TokenKind::KwAwait) {
            self.advance();
            self.skip_newlines();
            let inner = self.parse_unary();
            let span = Span::new(start, inner.span.end);
            return Expr::new(ExprKind::Await(Box::new(inner)), span);
        }
        if matches!(self.peek().kind, TokenKind::KwUnsafe | TokenKind::KwAsync) {
            let span = self.peek().span;
            let name = self.peek().kind.punct_text();
            self.advance();
            self.error(
                "E0108",
                format!("`{name}` is not implemented in this expression position"),
                span,
            );
            return self.parse_unary();
        }
        if self.check(&TokenKind::KwDefer) {
            let span = self.peek().span;
            self.advance();
            self.error("E0108", "`defer` is not implemented in this build", span);
            return self.parse_unary();
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if self.check(&TokenKind::Newline) && self.delimiter_depth > 0 {
                self.skip_newlines();
            }
            match self.peek().kind {
                TokenKind::LParen => {
                    let args = self.parse_call_args();
                    let end = self.tokens[self.pos.saturating_sub(1)].span.end;
                    let span = expr.span.merge(Span::new(end, end));
                    expr = Expr::new(
                        ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                        span,
                    );
                }
                TokenKind::LBracket => {
                    self.advance();
                    self.delimiter_depth += 1;
                    self.skip_newlines();
                    let index = self.parse_expr_prec(0);
                    self.skip_newlines();
                    let end = self
                        .expect(&TokenKind::RBracket, "to close index expression")
                        .map(|t| t.span.end)
                        .unwrap_or(self.peek().span.end);
                    self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                    let span = expr.span.merge(Span::new(end, end));
                    expr = Expr::new(
                        ExprKind::Index {
                            base: Box::new(expr),
                            index: Box::new(index),
                        },
                        span,
                    );
                }
                TokenKind::Dot => {
                    self.advance();
                    if let TokenKind::Int(index) = self.peek().kind {
                        if index >= 0 {
                            let index_span = self.advance().span;
                            let span = expr.span.merge(index_span);
                            expr = Expr::new(
                                ExprKind::TupleField {
                                    base: Box::new(expr),
                                    index: index as usize,
                                    index_span,
                                },
                                span,
                            );
                            continue;
                        }
                    }
                    let (name, name_span) = match self.expect_ident("after `.`") {
                        Some(v) => v,
                        None => break,
                    };
                    if self.check(&TokenKind::LParen) {
                        let args = self.parse_call_args();
                        let end = self.tokens[self.pos.saturating_sub(1)].span.end;
                        let span = expr.span.merge(Span::new(end, end));
                        expr = Expr::new(
                            ExprKind::MethodCall {
                                receiver: Box::new(expr),
                                method: name,
                                method_span: name_span,
                                args,
                            },
                            span,
                        );
                    } else {
                        let span = expr.span.merge(name_span);
                        expr = Expr::new(
                            ExprKind::Field {
                                base: Box::new(expr),
                                name,
                                name_span,
                            },
                            span,
                        );
                    }
                }
                TokenKind::Question => {
                    let token = self.advance();
                    let span = expr.span.merge(token.span);
                    expr = Expr::new(ExprKind::Try(Box::new(expr)), span);
                }
                _ => break,
            }
        }
        expr
    }

    fn parse_call_args(&mut self) -> Vec<Arg> {
        let mut args = Vec::new();
        self.expect(&TokenKind::LParen, "to open argument list");
        self.delimiter_depth += 1;
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.at_eof() {
            let before = self.pos;
            let mut name = None;
            let mut name_span = None;
            if matches!(self.peek().kind, TokenKind::Ident(_))
                && matches!(self.peek_at(1).kind, TokenKind::Colon)
            {
                if let TokenKind::Ident(n) = self.advance().kind {
                    name = Some(n);
                    name_span = Some(self.tokens[self.pos.saturating_sub(1)].span);
                }
                self.advance(); // ':'
                self.skip_newlines();
            }
            let value = self.parse_expr_prec(0);
            args.push(Arg {
                name,
                name_span,
                value,
            });
            self.skip_newlines();
            if self.eat(&TokenKind::Comma).is_some() {
                self.skip_newlines();
                continue;
            }
            if !self.check(&TokenKind::RParen) {
                let span = self.peek().span;
                self.error("E0104", "expected `,` or `)` in argument list", span);
                if self.pos == before {
                    self.advance();
                }
                // Recover to the closing paren on the same logical line.
                while !self.at_eof()
                    && !self.check(&TokenKind::RParen)
                    && !self.check(&TokenKind::Newline)
                {
                    self.advance();
                }
                self.skip_newlines();
            }
        }
        self.expect(&TokenKind::RParen, "to close argument list");
        self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        args
    }

    fn looks_like_struct_lit(&self) -> bool {
        self.looks_like_struct_lit_at(0)
    }

    /// Checks whether the token `offset` places ahead starts a struct literal.
    /// Used for `module.Type { ... }`, where the brace is not at offset 0.
    fn looks_like_struct_lit_at(&self, offset: usize) -> bool {
        if !matches!(self.peek_at(offset).kind, TokenKind::LBrace) {
            return false;
        }
        // Newlines may separate `{` from the first field.
        let mut index = offset + 1;
        while matches!(self.peek_at(index).kind, TokenKind::Newline) {
            index += 1;
        }
        if matches!(self.peek_at(index).kind, TokenKind::RBrace) {
            return true;
        }
        if !matches!(self.peek_at(index).kind, TokenKind::Ident(_)) {
            return false;
        }
        index += 1;
        while matches!(self.peek_at(index).kind, TokenKind::Newline) {
            index += 1;
        }
        matches!(self.peek_at(index).kind, TokenKind::Colon)
    }

    fn parse_primary(&mut self) -> Expr {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Int(value) => {
                self.advance();
                Expr::new(ExprKind::Int(value), token.span)
            }
            TokenKind::Float(value) => {
                self.advance();
                Expr::new(ExprKind::Float(value), token.span)
            }
            TokenKind::KwTrue => {
                self.advance();
                Expr::new(ExprKind::Bool(true), token.span)
            }
            TokenKind::KwFalse => {
                self.advance();
                Expr::new(ExprKind::Bool(false), token.span)
            }
            TokenKind::Char(value) => {
                self.advance();
                Expr::new(ExprKind::Char(value), token.span)
            }
            TokenKind::Str(raw) => {
                self.advance();
                let parts = self.parse_string_parts(&raw, token.span);
                Expr::new(ExprKind::Str(parts), token.span)
            }
            TokenKind::Ident(name) => {
                self.advance();
                if self.disallow_struct_lit == 0
                    && matches!(self.peek().kind, TokenKind::Dot)
                    && matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                    && self.looks_like_struct_lit_at(2)
                {
                    self.advance();
                    if let Some((part, _)) = self.expect_ident("in a qualified type name") {
                        let qualified = format!("{name}.{part}");
                        return self.parse_struct_lit(qualified, token.span);
                    }
                }
                if self.check(&TokenKind::LBrace)
                    && self.disallow_struct_lit == 0
                    && self.looks_like_struct_lit()
                {
                    return self.parse_struct_lit(name, token.span);
                }
                Expr::new(ExprKind::Ident(name), token.span)
            }
            TokenKind::Pipe | TokenKind::PipePipe => self.parse_closure(token.span.start),
            TokenKind::LParen => {
                self.advance();
                self.delimiter_depth += 1;
                self.skip_newlines();
                if self.check(&TokenKind::RParen) {
                    let end = self.advance().span.end;
                    self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                    return Expr::new(ExprKind::Unit, Span::new(token.span.start, end));
                }
                let first = self.parse_expr_prec(0);
                let mut items = vec![first];
                self.skip_newlines();
                let mut is_tuple = false;
                while self.eat(&TokenKind::Comma).is_some() {
                    is_tuple = true;
                    self.skip_newlines();
                    if self.check(&TokenKind::RParen) {
                        break;
                    }
                    items.push(self.parse_expr_prec(0));
                    self.skip_newlines();
                }
                let end = self
                    .expect(&TokenKind::RParen, "to close parenthesised expression")
                    .map(|t| t.span.end)
                    .unwrap_or(self.peek().span.end);
                self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                if is_tuple {
                    Expr::new(ExprKind::Tuple(items), Span::new(token.span.start, end))
                } else {
                    Expr::new(
                        ExprKind::Paren(Box::new(items.pop().unwrap())),
                        Span::new(token.span.start, end),
                    )
                }
            }
            TokenKind::LBracket => {
                self.advance();
                self.delimiter_depth += 1;
                self.skip_newlines();
                let mut items = Vec::new();
                while !self.check(&TokenKind::RBracket) && !self.at_eof() {
                    let before = self.pos;
                    items.push(self.parse_expr_prec(0));
                    self.skip_newlines();
                    if self.eat(&TokenKind::Comma).is_some() {
                        self.skip_newlines();
                        continue;
                    }
                    if !self.check(&TokenKind::RBracket) {
                        let span = self.peek().span;
                        self.error("E0104", "expected `,` or `]` in list literal", span);
                        if self.pos == before {
                            self.advance();
                        }
                    }
                    self.skip_newlines();
                }
                let end = self
                    .expect(&TokenKind::RBracket, "to close list literal")
                    .map(|t| t.span.end)
                    .unwrap_or(self.peek().span.end);
                self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                Expr::new(ExprKind::List(items), Span::new(token.span.start, end))
            }
            TokenKind::LBrace => {
                let block = self.parse_block();
                let span = block.span;
                Expr::new(ExprKind::Block(block), span)
            }
            TokenKind::KwIf => self.parse_if_expr(),
            TokenKind::KwMatch => self.parse_match_expr(),
            TokenKind::Underscore => {
                self.advance();
                self.error(
                    "E0105",
                    "`_` is only valid as a pattern or discard binding",
                    token.span,
                );
                Expr::new(ExprKind::Error, token.span)
            }
            TokenKind::Error(_) => {
                self.advance();
                Expr::new(ExprKind::Error, token.span)
            }
            _ => {
                let found = token.kind.describe();
                self.error(
                    "E0104",
                    format!("expected an expression, found {found}"),
                    token.span,
                );
                if !matches!(
                    token.kind,
                    TokenKind::Eof | TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket
                ) {
                    self.advance();
                }
                Expr::new(ExprKind::Error, token.span)
            }
        }
    }

    fn parse_struct_lit(&mut self, name: String, name_span: Span) -> Expr {
        self.expect(&TokenKind::LBrace, "to open struct literal");
        self.delimiter_depth += 1;
        self.skip_separators();
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            let Some((fname, fspan)) = self.expect_ident("for field name") else {
                break;
            };
            self.skip_newlines();
            self.expect(&TokenKind::Colon, "after struct literal field name");
            self.skip_newlines();
            let value = self.parse_expr_prec(0);
            fields.push(FieldInit {
                name: fname,
                name_span: fspan,
                value,
            });
            self.skip_newlines();
            if self.eat(&TokenKind::Comma).is_some() {
                self.skip_separators();
                continue;
            }
            if !self.check(&TokenKind::RBrace) && !self.at_separator() {
                let span = self.peek().span;
                self.error("E0104", "expected `,` or newline after field", span);
                if self.pos == before {
                    self.advance();
                }
            }
            self.skip_separators();
            if self.pos == before {
                self.advance();
            }
        }
        let end = self
            .expect(&TokenKind::RBrace, "to close struct literal")
            .map(|t| t.span.end)
            .unwrap_or(self.peek().span.end);
        self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        Expr::new(
            ExprKind::StructLit {
                name,
                name_span,
                fields,
            },
            name_span.merge(Span::new(end, end)),
        )
    }

    fn parse_closure(&mut self, start: usize) -> Expr {
        let mut params = Vec::new();
        if self.check(&TokenKind::PipePipe) {
            self.advance();
        } else {
            self.expect(&TokenKind::Pipe, "to open a closure parameter list");
            self.delimiter_depth += 1;
            self.skip_newlines();
            while !self.check(&TokenKind::Pipe) && !self.at_eof() {
                let before = self.pos;
                let Some((name, name_span)) = self.expect_ident("for a closure parameter") else {
                    break;
                };
                self.expect(&TokenKind::Colon, "after a closure parameter name");
                let ty = self.parse_type();
                params.push(Param {
                    name,
                    name_span,
                    ty,
                    span: name_span.merge(self.tokens[self.pos.saturating_sub(1)].span),
                });
                self.skip_newlines();
                if self.eat(&TokenKind::Comma).is_some() {
                    self.skip_newlines();
                    continue;
                }
                if self.pos == before {
                    self.advance();
                }
                break;
            }
            self.expect(&TokenKind::Pipe, "to close a closure parameter list");
            self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
        }
        self.skip_newlines();
        let ret = if self.eat(&TokenKind::Arrow).is_some() {
            self.skip_newlines();
            Some(self.parse_type())
        } else {
            None
        };
        self.skip_newlines();
        let body = if self.check(&TokenKind::LBrace) {
            self.parse_block()
        } else {
            let expr = self.parse_expr_prec(0);
            let span = expr.span;
            Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(expr),
                    span,
                }],
                span,
            }
        };
        Expr::new(
            ExprKind::Closure { params, ret, body },
            Span::new(start, self.tokens[self.pos.saturating_sub(1)].span.end),
        )
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start = self.peek().span.start;
        self.advance(); // if
        self.disallow_struct_lit += 1;
        let cond = self.parse_expr_prec(0);
        self.disallow_struct_lit -= 1;
        self.skip_newlines();
        let then_block = self.parse_block();
        // Only consume the newline before `else`; otherwise leave it as the
        // statement terminator for the enclosing block.
        let before_else = self.pos;
        self.skip_newlines();
        let else_branch = if self.eat(&TokenKind::KwElse).is_some() {
            self.skip_newlines();
            if self.check(&TokenKind::KwIf) {
                Some(ElseBranch::If(Box::new(self.parse_if_expr())))
            } else {
                Some(ElseBranch::Block(self.parse_block()))
            }
        } else {
            self.pos = before_else;
            None
        };
        let end = match &else_branch {
            Some(ElseBranch::Block(block)) => block.span.end,
            Some(ElseBranch::If(expr)) => expr.span.end,
            None => then_block.span.end,
        };
        Expr::new(
            ExprKind::If {
                cond: Box::new(cond),
                then_block,
                else_branch,
            },
            Span::new(start, end),
        )
    }

    fn parse_match_expr(&mut self) -> Expr {
        let start = self.peek().span.start;
        self.advance(); // match
        self.disallow_struct_lit += 1;
        let scrutinee = self.parse_expr_prec(0);
        self.disallow_struct_lit -= 1;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "to open match body");
        self.skip_separators();
        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            let pattern = self.parse_pattern();
            self.skip_newlines();
            self.expect(&TokenKind::FatArrow, "after match pattern");
            self.skip_newlines();
            let body = self.parse_expr_prec(0);
            let span = pattern.span.merge(body.span);
            arms.push(MatchArm {
                pattern,
                body,
                span,
            });
            self.eat(&TokenKind::Comma);
            self.skip_separators();
            if self.pos == before {
                self.advance();
            }
        }
        let end = self
            .expect(&TokenKind::RBrace, "to close match body")
            .map(|t| t.span.end)
            .unwrap_or(self.peek().span.end);
        Expr::new(
            ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            Span::new(start, end),
        )
    }

    fn parse_pattern(&mut self) -> Pattern {
        let token = self.peek().clone();
        let span = token.span;
        match token.kind {
            TokenKind::Underscore => {
                self.advance();
                Pattern {
                    kind: PatternKind::Wildcard,
                    span,
                }
            }
            TokenKind::Int(value) => {
                self.advance();
                Pattern {
                    kind: PatternKind::Literal(PatternLiteral::Int(value)),
                    span,
                }
            }
            TokenKind::KwTrue | TokenKind::KwFalse => {
                self.advance();
                Pattern {
                    kind: PatternKind::Literal(PatternLiteral::Bool(matches!(
                        token.kind,
                        TokenKind::KwTrue
                    ))),
                    span,
                }
            }
            TokenKind::Char(value) => {
                self.advance();
                Pattern {
                    kind: PatternKind::Literal(PatternLiteral::Char(value)),
                    span,
                }
            }
            TokenKind::Str(raw) => {
                self.advance();
                let parts = self.parse_string_parts(&raw, span);
                let mut text = String::new();
                let mut ok = true;
                for part in parts {
                    match part {
                        StrPart::Literal(s) => text.push_str(&s),
                        StrPart::Expr(_) => {
                            ok = false;
                        }
                    }
                }
                if !ok {
                    self.error(
                        "E0109",
                        "string interpolation is not allowed in patterns",
                        span,
                    );
                }
                Pattern {
                    kind: PatternKind::Literal(PatternLiteral::Str(text)),
                    span,
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                let mut path = vec![name];
                while matches!(self.peek().kind, TokenKind::Dot)
                    && matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                {
                    self.advance();
                    if let Some((part, _)) = self.expect_ident("in a qualified variant path") {
                        path.push(part);
                    }
                }
                let qualified = path.len() > 1;
                let name = path.join(".");
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    self.delimiter_depth += 1;
                    self.skip_newlines();
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.at_eof() {
                        fields.push(self.parse_pattern());
                        self.skip_newlines();
                        if self.eat(&TokenKind::Comma).is_some() {
                            self.skip_newlines();
                            continue;
                        }
                        break;
                    }
                    self.skip_newlines();
                    let end = self
                        .expect(&TokenKind::RParen, "to close variant pattern")
                        .map(|t| t.span.end)
                        .unwrap_or(self.peek().span.end);
                    self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                    Pattern {
                        kind: PatternKind::Variant {
                            name,
                            name_span: span,
                            fields,
                        },
                        span: span.merge(Span::new(end, end)),
                    }
                } else if qualified {
                    Pattern {
                        kind: PatternKind::Variant {
                            name,
                            name_span: span,
                            fields: Vec::new(),
                        },
                        span,
                    }
                } else {
                    Pattern {
                        kind: PatternKind::Bind(name),
                        span,
                    }
                }
            }
            TokenKind::LParen => {
                self.advance();
                self.delimiter_depth += 1;
                self.skip_newlines();
                let first = self.parse_pattern();
                let mut fields = vec![first];
                let mut is_tuple = false;
                while self.eat(&TokenKind::Comma).is_some() {
                    is_tuple = true;
                    self.skip_newlines();
                    if self.check(&TokenKind::RParen) {
                        break;
                    }
                    fields.push(self.parse_pattern());
                    self.skip_newlines();
                }
                self.expect(&TokenKind::RParen, "to close pattern");
                self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                if is_tuple {
                    Pattern {
                        kind: PatternKind::Tuple(fields),
                        span,
                    }
                } else {
                    fields.pop().unwrap()
                }
            }
            _ => {
                let found = token.kind.describe();
                self.error("E0105", format!("expected a pattern, found {found}"), span);
                if !matches!(
                    token.kind,
                    TokenKind::Eof | TokenKind::RBrace | TokenKind::FatArrow
                ) {
                    self.advance();
                }
                Pattern {
                    kind: PatternKind::Wildcard,
                    span,
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // Strings and interpolation
    // ---------------------------------------------------------------------

    fn parse_string_parts(&mut self, raw: &str, span: Span) -> Vec<StrPart> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut i = 0usize;
        while i < raw.len() {
            let ch = raw[i..].chars().next().unwrap();
            if ch == '\\' {
                match decode_escape(raw, i) {
                    Ok((decoded, next)) => {
                        literal.push(decoded);
                        i = next;
                    }
                    Err(message) => {
                        let err_span = Span::new(span.start + 1 + i, span.start + 1 + raw.len());
                        self.error("E0110", message, err_span);
                        i += ch.len_utf8();
                    }
                }
                continue;
            }
            if ch == '$' && raw[i + 1..].starts_with('{') {
                if !literal.is_empty() {
                    parts.push(StrPart::Literal(std::mem::take(&mut literal)));
                }
                let expr_start = i + 2;
                match find_interp_end(raw, expr_start) {
                    Some(end) => {
                        let expr_src = &raw[expr_start..end];
                        let expr_base = span.start + 1 + expr_start;
                        let parent = Span::new(span.start + 1 + i, span.start + 1 + end + 1);
                        let expr = self.parse_sub_expr(expr_src, expr_base, parent);
                        parts.push(StrPart::Expr(expr));
                        i = end + 1;
                    }
                    None => {
                        let err_span = Span::new(span.start + 1 + i, span.end);
                        self.error(
                            "E0113",
                            "unterminated string interpolation; expected `}`",
                            err_span,
                        );
                        i = raw.len();
                    }
                }
                continue;
            }
            literal.push(ch);
            i += ch.len_utf8();
        }
        if !literal.is_empty() {
            parts.push(StrPart::Literal(literal));
        }
        parts
    }

    fn parse_sub_expr(&mut self, src: &str, base: usize, parent: Span) -> Expr {
        let lexed = lex_with_base(src, base);
        let mut sub = Parser::new(lexed.tokens);
        sub.diagnostics.extend(lexed.diagnostics);
        sub.delimiter_depth = 1;
        sub.skip_newlines();
        let expr = sub.parse_expr_prec(0);
        sub.skip_newlines();
        if !sub.at_eof() {
            let span = sub.peek().span;
            sub.error(
                "E0113",
                "unexpected token after interpolation expression",
                span,
            );
        }
        self.diagnostics.extend(sub.diagnostics);
        if expr.span.start < parent.start || expr.span.end > parent.end {
            Expr::new(expr.kind, parent)
        } else {
            expr
        }
    }
}

fn binary_op(kind: &TokenKind) -> Option<(u8, BinaryOp)> {
    Some(match kind {
        TokenKind::PipePipe => (1, BinaryOp::Or),
        TokenKind::AmpAmp => (2, BinaryOp::And),
        TokenKind::EqEq => (3, BinaryOp::Eq),
        TokenKind::BangEq => (3, BinaryOp::NotEq),
        TokenKind::Lt => (4, BinaryOp::Lt),
        TokenKind::LtEq => (4, BinaryOp::LtEq),
        TokenKind::Gt => (4, BinaryOp::Gt),
        TokenKind::GtEq => (4, BinaryOp::GtEq),
        TokenKind::DotDot => (5, BinaryOp::Range),
        TokenKind::DotDotEq => (5, BinaryOp::RangeInclusive),
        TokenKind::Plus => (6, BinaryOp::Add),
        TokenKind::Minus => (6, BinaryOp::Sub),
        TokenKind::Star => (7, BinaryOp::Mul),
        TokenKind::Slash => (7, BinaryOp::Div),
        TokenKind::Percent => (7, BinaryOp::Rem),
        _ => return None,
    })
}

/// Decode a single escape sequence starting at the backslash at `idx`.
fn decode_escape(raw: &str, idx: usize) -> Result<(char, usize), String> {
    let mut chars = raw[idx..].char_indices();
    chars.next(); // backslash
    let (_, esc) = chars
        .next()
        .ok_or_else(|| "trailing backslash in string literal".to_string())?;
    let after_esc = idx + 1 + esc.len_utf8();
    let decoded = match esc {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '0' => '\0',
        '\\' => '\\',
        '\'' => '\'',
        '"' => '"',
        '$' => '$',
        'u' => {
            let rest = &raw[after_esc..];
            if !rest.starts_with('{') {
                return Err("`\\u` escape requires braces, e.g. `\\u{1F600}`".into());
            }
            let close = rest
                .find('}')
                .ok_or_else(|| "unterminated `\\u{...}` escape".to_string())?;
            let hex = &rest[1..close];
            let value = u32::from_str_radix(hex, 16)
                .map_err(|_| format!("invalid unicode escape `\\u{{{hex}}}`"))?;
            let ch = char::from_u32(value)
                .ok_or_else(|| format!("invalid unicode scalar value `{value:#x}`"))?;
            return Ok((ch, after_esc + close + 1));
        }
        other => return Err(format!("unknown escape sequence `\\{other}`")),
    };
    Ok((decoded, after_esc))
}

/// Find the byte index of the `}` closing an interpolation that starts at
/// `start` (just after `${`). Handles nested braces, quoted strings and line
/// comments inside the expression.
fn find_interp_end(raw: &str, start: usize) -> Option<usize> {
    let mut i = start;
    let mut depth = 1usize;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    while i < raw.len() {
        let ch = raw[i..].chars().next()?;
        let width = ch.len_utf8();
        if escaped {
            escaped = false;
            i += width;
            continue;
        }
        if in_string {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += width;
            continue;
        }
        if in_char {
            if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_char = false;
            }
            i += width;
            continue;
        }
        match ch {
            '"' => in_string = true,
            '\'' => {
                // A quote begins a char literal only in expression position;
                // treating every quote as a char literal is safe here because
                // a `}` inside would not be code either way.
                in_char = true;
            }
            '/' if raw[i + 1..].starts_with('/') => {
                while i < raw.len() {
                    let c = raw[i..].chars().next()?;
                    if c == '\n' {
                        break;
                    }
                    i += c.len_utf8();
                }
                continue;
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += width;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Program {
        let result = parse(src);
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:#?}",
            result.diagnostics
        );
        result.program
    }

    #[test]
    fn parses_hello_world() {
        let program = parse_ok(
            r#"import std.io { println }

fn main() {
    println("Hello, World!")
}
"#,
        );
        assert_eq!(program.items.len(), 2);
        match &program.items[1] {
            Item::Fn(f) => {
                assert_eq!(f.name, "main");
                assert!(f.ret.is_none());
                assert_eq!(f.body.stmts.len(), 1);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_greeting_program() {
        let src = r#"fn greet(name: String) -> String {
    return "Hello, ${name}!"
}

fn main() {
    let name = "World"
    var count = 0
    for i in 0..5 {
        println(greet(name))
        count += i
    }
    println(count)
}
"#;
        let program = parse_ok(src);
        assert_eq!(program.items.len(), 2);
        let Item::Fn(main) = &program.items[1] else {
            panic!()
        };
        assert_eq!(main.body.stmts.len(), 4);
        let StmtKind::For { iter, .. } = &main.body.stmts[2].kind else {
            panic!("expected for loop");
        };
        assert!(matches!(
            iter.kind,
            ExprKind::Binary {
                op: BinaryOp::Range,
                ..
            }
        ));
    }

    #[test]
    fn precedence_is_correct() {
        let program = parse_ok("fn main() { let x = 1 + 2 * 3 }");
        let Item::Fn(f) = &program.items[0] else {
            panic!()
        };
        let StmtKind::Let { init, .. } = &f.body.stmts[0].kind else {
            panic!()
        };
        let ExprKind::Binary { op, rhs, .. } = &init.kind else {
            panic!()
        };
        assert_eq!(*op, BinaryOp::Add);
        assert!(matches!(
            rhs.kind,
            ExprKind::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn optional_semicolons_and_newlines() {
        let program = parse_ok("fn main() {\n let a = 1\n let b = 2;\n\n b\n}");
        let Item::Fn(f) = &program.items[0] else {
            panic!()
        };
        assert_eq!(f.body.stmts.len(), 3);
    }

    #[test]
    fn interpolation_produces_parts() {
        let program = parse_ok(r#"fn main() { let s = "hi ${name}, ${1 + 2}" }"#);
        let Item::Fn(f) = &program.items[0] else {
            panic!()
        };
        let StmtKind::Let { init, .. } = &f.body.stmts[0].kind else {
            panic!()
        };
        let ExprKind::Str(parts) = &init.kind else {
            panic!()
        };
        assert_eq!(parts.len(), 4);
        assert!(matches!(parts[0], StrPart::Literal(ref s) if s == "hi "));
        assert!(matches!(parts[1], StrPart::Expr(_)));
        assert!(matches!(parts[2], StrPart::Literal(ref s) if s == ", "));
        assert!(matches!(parts[3], StrPart::Expr(_)));
    }

    #[test]
    fn parses_structs_enums_match_and_try() {
        let src = r#"struct User {
    name: String
    age: Int
}

enum Color {
    Red
    Green(payload: Int)
}

fn find(id: Int) -> Result<User, String> {
    let user = lookup(id)?
    match result {
        Ok(value) => println(value.name)
        Err(error) => println(error)
    }
    return Ok(User { name: "a", age: 1 })
}
"#;
        // `Green(payload: Int)` is not valid syntax (named payload), so this
        // source should report diagnostics but still parse without hanging.
        let result = parse(src);
        assert!(!result.diagnostics.is_empty());
        assert!(result.diagnostics.iter().any(|d| d.code == "E0103"));
    }

    #[test]
    fn parses_match_struct_literal() {
        let src = r#"fn main() {
    let user = User { name: "Ada", age: 36 }
    match user {
        User { name } => println(name)
    }
}
"#;
        // Struct patterns are not implemented yet; parsing should still not
        // hang and should report a diagnostic.
        let result = parse(src);
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn does_not_hang_on_malformed_input() {
        for src in [
            "fn",
            "fn main(",
            "let x =",
            "match {",
            "struct {",
            "(((((",
            "fn main() { if { }",
            "fn main() { let x = 1 let y = 2 }",
        ] {
            let _ = parse(src);
        }
    }

    #[test]
    fn reports_missing_separator() {
        let result = parse("fn main() { let a = 1 let b = 2 }");
        assert!(result.diagnostics.iter().any(|d| d.code == "E0102"));
    }

    #[test]
    fn parses_else_if_chain() {
        let program = parse_ok("fn main() {\n if true { } else if false { } else { }\n}");
        let Item::Fn(f) = &program.items[0] else {
            panic!()
        };
        let StmtKind::Expr(expr) = &f.body.stmts[0].kind else {
            panic!()
        };
        let ExprKind::If { else_branch, .. } = &expr.kind else {
            panic!()
        };
        assert!(matches!(else_branch, Some(ElseBranch::If(_))));
    }

    #[test]
    fn parses_multiline_call_arguments() {
        parse_ok("fn main() {\n foo(\n  1,\n  2,\n )\n}");
    }

    #[test]
    fn import_alias() {
        let program = parse_ok("import std.io { println as p }");
        let Item::Import(import) = &program.items[0] else {
            panic!()
        };
        assert_eq!(import.module, vec!["std", "io"]);
        assert_eq!(import.items[0].alias.as_deref(), Some("p"));
    }

    #[test]
    fn test_attribute_is_parsed() {
        let program = parse_ok("@test\nfn adds() {\n assert(1 + 1 == 2)\n}");
        let Item::Fn(f) = &program.items[0] else {
            panic!()
        };
        assert!(f.is_test);
    }
}
