//! Typed high-level IR consumed by `nova-codegen-llvm`.
//!
//! All names are resolved: locals have unique [`LocalId`]s, struct fields are
//! indices and every expression carries its final [`Type`]. Built-in
//! operations are explicit variants instead of string method names wherever
//! codegen needs special treatment.

use nova_diag::Span;
use nova_parser::ast::{AssignOp, BinaryOp, UnaryOp};

use crate::types::Type;

#[derive(Clone, Debug)]
pub struct TypedProgram {
    pub functions: Vec<TypedFn>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub externs: Vec<ExternDef>,
    pub trait_impls: Vec<TraitImplDef>,
    pub tests: Vec<String>,
    pub main: Option<String>,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
}

#[derive(Clone, Debug)]
pub struct TraitImplDef {
    pub trait_name: String,
    pub ty: Type,
    /// Method symbols in trait declaration order, for vtable layout.
    pub methods: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ExternDef {
    pub name: String,
    pub symbol: String,
    pub params: Vec<Type>,
    pub ret: Type,
}

#[derive(Clone, Debug)]
pub struct EnumVariantDef {
    pub name: String,
    /// Positional payload types. Empty for a unit variant.
    pub fields: Vec<Type>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

#[derive(Clone, Debug)]
pub struct LocalInfo {
    pub id: LocalId,
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub is_param: bool,
}

#[derive(Clone, Debug)]
pub struct TypedFn {
    pub name: String,
    pub params: Vec<LocalId>,
    /// Outer locals loaded from the hidden environment pointer at entry.
    pub captures: Vec<LocalId>,
    pub ret: Type,
    pub body: TBlock,
    pub locals: Vec<LocalInfo>,
    pub span: Span,
    pub is_test: bool,
}

#[derive(Clone, Debug)]
pub struct TBlock {
    pub stmts: Vec<TStmt>,
    pub tail: Option<Box<TExpr>>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TStmt {
    pub kind: TStmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TStmtKind {
    Let {
        local: LocalId,
        init: TExpr,
    },
    Assign {
        target: LValue,
        op: AssignOp,
        value: TExpr,
    },
    Expr(TExpr),
    While {
        cond: TExpr,
        body: TBlock,
    },
    ForRange {
        var: LocalId,
        start: TExpr,
        end: TExpr,
        inclusive: bool,
        body: TBlock,
    },
    ForList {
        var: LocalId,
        list: TExpr,
        body: TBlock,
    },
    Return(Option<TExpr>),
    Break,
    Continue,
}

#[derive(Clone, Debug)]
pub enum LValue {
    Local(LocalId),
    Field(Box<LValue>, usize),
    TupleField(Box<LValue>, usize),
    Index(Box<LValue>, Box<TExpr>),
}

#[derive(Clone, Debug)]
pub struct TExpr {
    pub kind: TExprKind,
    pub ty: Type,
    pub span: Span,
}

impl TExpr {
    pub fn new(kind: TExprKind, ty: Type, span: Span) -> Self {
        Self { kind, ty, span }
    }
}

#[derive(Clone, Debug)]
pub enum TExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(Vec<TStrPart>),
    Local(LocalId),
    Unary {
        op: UnaryOp,
        expr: Box<TExpr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<TExpr>,
        rhs: Box<TExpr>,
    },
    /// Call to a user-defined function, by source name.
    Call {
        name: String,
        args: Vec<TExpr>,
    },
    /// Reference to a named function or a generated closure function.
    FunctionRef {
        name: String,
    },
    Closure {
        name: String,
        captures: Vec<TExpr>,
    },
    FnCall {
        callee: Box<TExpr>,
        args: Vec<TExpr>,
    },
    /// Box a concrete value and pair it with a vtable.
    TraitObject {
        value: Box<TExpr>,
        trait_name: String,
        value_ty: Type,
    },
    /// Dynamic dispatch through a trait object's vtable.
    DynCall {
        receiver: Box<TExpr>,
        trait_name: String,
        index: usize,
        args: Vec<TExpr>,
    },
    Builtin {
        kind: BuiltinFn,
        args: Vec<TExpr>,
    },
    Method {
        receiver: Box<TExpr>,
        method: BuiltinMethod,
        args: Vec<TExpr>,
    },
    /// Call to a method declared in an `impl` block. `name` is the resolved
    /// `Type.method` symbol; the receiver is passed as the first argument.
    UserMethod {
        receiver: Box<TExpr>,
        name: String,
        args: Vec<TExpr>,
    },
    ListPush {
        list: LValue,
        value: Box<TExpr>,
    },
    Field {
        base: Box<TExpr>,
        index: usize,
    },
    Index {
        base: Box<TExpr>,
        index: Box<TExpr>,
    },
    StructLit {
        name: String,
        fields: Vec<(usize, TExpr)>,
    },
    /// Construct a user enum variant. `tag` is the declaration index.
    EnumVariant {
        enum_name: String,
        tag: u32,
        fields: Vec<TExpr>,
    },
    If {
        cond: Box<TExpr>,
        then_block: TBlock,
        else_branch: Option<Box<TExpr>>,
    },
    Match {
        scrutinee: Box<TExpr>,
        arms: Vec<TArm>,
    },
    Block(TBlock),
    List(Vec<TExpr>),
    MapEmpty,
    SetEmpty,
    MapInsert {
        map: LValue,
        key: Box<TExpr>,
        value: Box<TExpr>,
    },
    SetInsert {
        set: LValue,
        value: Box<TExpr>,
    },
    Array(Vec<TExpr>),
    Tuple(Vec<TExpr>),
    TupleField {
        base: Box<TExpr>,
        index: usize,
    },
    OptionSome(Box<TExpr>),
    OptionNone,
    ResultOk(Box<TExpr>),
    ResultErr(Box<TExpr>),
    Try(Box<TExpr>),
    Unit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinFn {
    Println,
    Print,
    Eprintln,
    Assert,
    AssertEq,
    Panic,
    TermPaste,
    HttpGet,
    JsonString,
    CharString,
    HttpListen,
    HttpAccept,
    HttpMethod,
    HttpPath,
    HttpQuery,
    HttpBody,
    HttpHeader,
    HttpForm,
    HttpRespond,
    HttpClose,
}

impl BuiltinFn {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "println" => BuiltinFn::Println,
            "print" => BuiltinFn::Print,
            "eprintln" => BuiltinFn::Eprintln,
            "assert" => BuiltinFn::Assert,
            "assert_eq" => BuiltinFn::AssertEq,
            "panic" => BuiltinFn::Panic,
            "term_paste" => BuiltinFn::TermPaste,
            "http_get" => BuiltinFn::HttpGet,
            "json_string" => BuiltinFn::JsonString,
            "char_string" => BuiltinFn::CharString,
            "http_listen" => BuiltinFn::HttpListen,
            "http_accept" => BuiltinFn::HttpAccept,
            "http_method" => BuiltinFn::HttpMethod,
            "http_path" => BuiltinFn::HttpPath,
            "http_query" => BuiltinFn::HttpQuery,
            "http_body" => BuiltinFn::HttpBody,
            "http_header" => BuiltinFn::HttpHeader,
            "http_form" => BuiltinFn::HttpForm,
            "http_respond" => BuiltinFn::HttpRespond,
            "http_close" => BuiltinFn::HttpClose,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            BuiltinFn::Println => "println",
            BuiltinFn::Print => "print",
            BuiltinFn::Eprintln => "eprintln",
            BuiltinFn::Assert => "assert",
            BuiltinFn::AssertEq => "assert_eq",
            BuiltinFn::Panic => "panic",
            BuiltinFn::TermPaste => "term_paste",
            BuiltinFn::HttpGet => "http_get",
            BuiltinFn::JsonString => "json_string",
            BuiltinFn::CharString => "char_string",
            BuiltinFn::HttpListen => "http_listen",
            BuiltinFn::HttpAccept => "http_accept",
            BuiltinFn::HttpMethod => "http_method",
            BuiltinFn::HttpPath => "http_path",
            BuiltinFn::HttpQuery => "http_query",
            BuiltinFn::HttpBody => "http_body",
            BuiltinFn::HttpHeader => "http_header",
            BuiltinFn::HttpForm => "http_form",
            BuiltinFn::HttpRespond => "http_respond",
            BuiltinFn::HttpClose => "http_close",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinMethod {
    ListLen,
    ListIsEmpty,
    ListGet,
    ListPush,
    ListToSlice,
    MapLen,
    MapGet,
    MapContainsKey,
    SetLen,
    SetContains,
    ArrayLen,
    ArrayToSlice,
    SliceLen,
    StringLen,
    StringIsEmpty,
    StringContains,
    StringStartsWith,
    StringEndsWith,
    StringTrim,
    StringTrimStart,
    StringTrimEnd,
    StringSplit,
    StringJoin,
    StringDisplayWidth,
    StringClip,
    StringWrap,
    StringGraphemes,
    StringChars,
    StringReplace,
    StringLower,
    OptionIsSome,
    OptionIsNone,
    OptionUnwrapOr,
    ResultIsOk,
    ResultIsErr,
    ResultUnwrapOr,
    ToString,
}

#[derive(Clone, Debug)]
pub struct TStrPart {
    pub kind: TStrPartKind,
}

#[derive(Clone, Debug)]
pub enum TStrPartKind {
    Literal(String),
    Expr(TExpr),
}

#[derive(Clone, Debug)]
pub struct TArm {
    pub pattern: TPattern,
    pub body: TExpr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TPattern {
    Wildcard,
    Bind(LocalId),
    Bool(bool),
    Int(i64),
    Char(char),
    String(String),
    Some(Option<LocalId>),
    None,
    Ok(Option<LocalId>),
    Err(Option<LocalId>),
    /// User enum variant, identified by declaration tag. Each entry binds one
    /// positional payload field; `None` means the field is ignored.
    Enum {
        tag: u32,
        bindings: Vec<Option<LocalId>>,
    },
    /// Tuple pattern. Each entry binds one positional element.
    Tuple(Vec<Option<LocalId>>),
}
