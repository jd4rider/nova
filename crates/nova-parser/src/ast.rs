//! Untyped syntax tree for Nova.
//!
//! Spans are UTF-8 byte ranges. The AST intentionally contains no resolved
//! names or types; `nova-sema` produces the typed tree used by codegen.

use nova_diag::Span;

#[derive(Clone, Debug)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Impl(ImplDecl),
    Trait(TraitDecl),
    Extern(ExternFnDecl),
    Macro(MacroDecl),
    Import(ImportDecl),
}

#[derive(Clone, Debug)]
pub struct FnDecl {
    pub name: String,
    pub name_span: Span,
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Block,
    pub is_pub: bool,
    pub is_test: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub name_span: Span,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StructDecl {
    pub name: String,
    pub name_span: Span,
    pub generics: Vec<String>,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FieldDecl {
    pub name: String,
    pub name_span: Span,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub name: String,
    pub name_span: Span,
    pub generics: Vec<String>,
    pub variants: Vec<VariantDecl>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct VariantDecl {
    pub name: String,
    pub name_span: Span,
    /// Positional payload types. Empty for a unit variant.
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ImplDecl {
    pub type_name: String,
    pub type_span: Span,
    pub generics: Vec<String>,
    pub type_args: Vec<TypeExpr>,
    pub trait_name: Option<String>,
    pub methods: Vec<FnDecl>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ExternFnDecl {
    pub abi: String,
    pub name: String,
    pub name_span: Span,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MacroDecl {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<(String, Span)>,
    /// Macro body block. In expression position only the trailing expression
    /// contributes a value; in statement position the statements are spliced
    /// into the caller's block.
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TraitDecl {
    pub name: String,
    pub name_span: Span,
    pub methods: Vec<FnSignature>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FnSignature {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ImportDecl {
    pub module: Vec<String>,
    /// Relative file path for a user module import.
    pub path: Option<String>,
    /// Module alias from `import "file.nova" as name`.
    pub alias: Option<String>,
    pub items: Vec<ImportItem>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ImportItem {
    pub name: String,
    pub name_span: Span,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeExprKind {
    Named {
        path: Vec<String>,
        args: Vec<TypeExpr>,
    },
    /// `dyn Trait` trait object type.
    Dyn(String),
    Unit,
    Tuple(Vec<TypeExpr>),
    Array {
        elem: Box<TypeExpr>,
        len: usize,
    },
    Fn {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: String,
        name_span: Span,
        ty: Option<TypeExpr>,
        init: Expr,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    Expr(Expr),
    While {
        cond: Expr,
        body: Block,
    },
    For {
        name: String,
        name_span: Span,
        iter: Expr,
        body: Block,
    },
    Return(Option<Expr>),
    Break,
    Continue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(Vec<StrPart>),
    Ident(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        method_span: Span,
        args: Vec<Arg>,
    },
    Field {
        base: Box<Expr>,
        name: String,
        name_span: Span,
    },
    TupleField {
        base: Box<Expr>,
        index: usize,
        index_span: Span,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    StructLit {
        name: String,
        name_span: Span,
        fields: Vec<FieldInit>,
    },
    If {
        cond: Box<Expr>,
        then_block: Block,
        else_branch: Option<ElseBranch>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Block(Block),
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Closure {
        params: Vec<Param>,
        ret: Option<TypeExpr>,
        body: Block,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },
    Paren(Box<Expr>),
    Unit,
    Try(Box<Expr>),
    Await(Box<Expr>),
    /// Placeholder produced during error recovery. Never reaches type checking
    /// without a diagnostic already being emitted.
    Error,
}

#[derive(Clone, Debug)]
pub enum StrPart {
    Literal(String),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub name: Option<String>,
    pub name_span: Option<Span>,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct FieldInit {
    pub name: String,
    pub name_span: Span,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub enum ElseBranch {
    Block(Block),
    If(Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatternKind {
    Wildcard,
    Bind(String),
    Literal(PatternLiteral),
    Tuple(Vec<Pattern>),
    Variant {
        name: String,
        name_span: Span,
        fields: Vec<Pattern>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternLiteral {
    Int(i64),
    Bool(bool),
    Char(char),
    Str(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Range,
    RangeInclusive,
}
