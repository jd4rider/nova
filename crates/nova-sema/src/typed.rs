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
pub struct EnumVariantDef {
    pub name: String,
    /// `None` for a unit variant; at most one payload value per variant.
    pub payload: Option<Type>,
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
    Builtin {
        kind: BuiltinFn,
        args: Vec<TExpr>,
    },
    Method {
        receiver: Box<TExpr>,
        method: BuiltinMethod,
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
        payload: Option<Box<TExpr>>,
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
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinMethod {
    ListLen,
    ListIsEmpty,
    ListGet,
    ListPush,
    StringLen,
    StringIsEmpty,
    StringContains,
    StringStartsWith,
    StringEndsWith,
    StringTrim,
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
    /// User enum variant, identified by declaration tag.
    Enum {
        tag: u32,
        payload: Option<LocalId>,
    },
}
