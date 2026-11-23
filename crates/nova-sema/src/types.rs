//! Nova's static type model.
//!
//! There are no implicit conversions, no implicit null and no subtyping in the
//! currently implemented subset. `Error` is an internal poison type that stops
//! cascading diagnostics after a failure.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Unit,
    Bool,
    Int,
    Float,
    Char,
    String,
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Struct(String),
    /// Reserved for non-returning expressions; not yet produced by the checker.
    Never,
    Error,
}

impl Type {
    pub fn option(inner: Type) -> Type {
        Type::Option(Box::new(inner))
    }

    pub fn result(ok: Type, err: Type) -> Type {
        Type::Result(Box::new(ok), Box::new(err))
    }

    pub fn list(inner: Type) -> Type {
        Type::List(Box::new(inner))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    pub fn is_printable(&self) -> bool {
        matches!(
            self,
            Type::Bool | Type::Int | Type::Float | Type::Char | Type::String
        )
    }

    /// Types that can be compared with `==` in the current subset.
    pub fn is_equatable(&self) -> bool {
        matches!(
            self,
            Type::Unit | Type::Bool | Type::Int | Type::Float | Type::Char | Type::String
        )
    }

    pub fn is_ordered_scalar(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Char)
    }

    /// True for types stored inline without indirection (used for layout
    /// decisions and self-referential struct detection).
    pub fn contains_value_struct(&self, name: &str) -> bool {
        match self {
            Type::Struct(n) => n == name,
            Type::Option(inner) => inner.contains_value_struct(name),
            Type::Result(ok, err) => {
                ok.contains_value_struct(name) || err.contains_value_struct(name)
            }
            Type::List(_) => false,
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Unit => write!(f, "Unit"),
            Type::Bool => write!(f, "Bool"),
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Char => write!(f, "Char"),
            Type::String => write!(f, "String"),
            Type::List(inner) => write!(f, "List<{inner}>"),
            Type::Option(inner) => write!(f, "Option<{inner}>"),
            Type::Result(ok, err) => write!(f, "Result<{ok}, {err}>"),
            Type::Struct(name) => write!(f, "{name}"),
            Type::Never => write!(f, "Never"),
            Type::Error => write!(f, "<error>"),
        }
    }
}
