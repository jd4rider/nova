//! Name resolution and type checking for Nova.
//!
//! The implemented subset is deliberately small but sound: explicit types at
//! function boundaries, local inference, immutable bindings by default, no
//! implicit numeric conversion, exhaustive matching for `Bool`, `Option` and
//! `Result`, checked `?` propagation and value structs.
//!
//! Unsupported language features are reported with a roadmap diagnostic rather
//! than silently accepted. No code is generated when any error diagnostic is
//! produced.

mod typed;
mod types;

pub use typed::*;
pub use types::Type;

use std::collections::{HashMap, HashSet};

use nova_diag::{Diagnostic, Span};
use nova_parser::ast::{
    self, BinaryOp, ElseBranch, ExprKind, PatternKind, PatternLiteral, StmtKind, UnaryOp,
};

pub struct CheckResult {
    pub program: TypedProgram,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }
}

pub fn check(program: &ast::Program) -> CheckResult {
    Checker::default().run(program)
}

#[derive(Clone)]
struct FnSig {
    params: Vec<Type>,
    ret: Type,
    is_test: bool,
    span: Span,
}

#[derive(Clone)]
struct LocalBinding {
    id: LocalId,
    ty: Type,
    mutable: bool,
}

struct FnState {
    ret: Type,
    scopes: Vec<HashMap<String, LocalBinding>>,
    locals: Vec<LocalInfo>,
    loop_depth: usize,
}

impl FnState {
    fn new(ret: Type) -> Self {
        Self {
            ret,
            scopes: Vec::new(),
            locals: Vec::new(),
            loop_depth: 0,
        }
    }
}

#[derive(Default)]
struct Coverage {
    wildcard: bool,
    bool_true: bool,
    bool_false: bool,
    some: bool,
    none: bool,
    ok: bool,
    err: bool,
    literal: bool,
}

#[derive(Default)]
struct Checker {
    funcs: HashMap<String, FnSig>,
    func_order: Vec<String>,
    structs: HashMap<String, StructDef>,
    struct_order: Vec<String>,
    struct_spans: HashMap<String, Span>,
    aliases: HashMap<String, BuiltinFn>,
    diagnostics: Vec<Diagnostic>,
    current: Option<FnState>,
    next_local: u32,
}

impl Checker {
    fn run(mut self, program: &ast::Program) -> CheckResult {
        self.collect_imports(program);
        self.collect_struct_names(program);
        self.collect_struct_fields(program);
        self.detect_struct_cycles();
        self.collect_functions(program);

        let mut functions = Vec::new();
        for item in &program.items {
            if let ast::Item::Fn(decl) = item {
                if !self.funcs.contains_key(&decl.name) {
                    continue; // generic or duplicate; already diagnosed
                }
                if let Some(func) = self.check_fn(decl) {
                    functions.push(func);
                }
            }
        }

        let tests: Vec<String> = functions
            .iter()
            .filter(|f| f.is_test)
            .map(|f| f.name.clone())
            .collect();
        let main = if functions.iter().any(|f| f.name == "main" && !f.is_test) {
            Some("main".to_string())
        } else {
            None
        };

        // Preserve source order for deterministic output and object layout.
        functions.sort_by_key(|f| {
            self.func_order
                .iter()
                .position(|n| n == &f.name)
                .unwrap_or(usize::MAX)
        });

        CheckResult {
            program: TypedProgram {
                functions,
                structs: self
                    .struct_order
                    .iter()
                    .filter_map(|n| self.structs.get(n).cloned())
                    .collect(),
                tests,
                main,
            },
            diagnostics: self.diagnostics,
        }
    }

    // ---------------------------------------------------------------------
    // Collection and signatures
    // ---------------------------------------------------------------------

    fn collect_imports(&mut self, program: &ast::Program) {
        for item in &program.items {
            let ast::Item::Import(import) = item else {
                continue;
            };
            let module = import.module.join(".");
            let allowed: Option<Vec<BuiltinFn>> = match module.as_str() {
                "std.io" => Some(vec![
                    BuiltinFn::Println,
                    BuiltinFn::Print,
                    BuiltinFn::Eprintln,
                ]),
                "std.core" => Some(vec![
                    BuiltinFn::Assert,
                    BuiltinFn::AssertEq,
                    BuiltinFn::Panic,
                ]),
                _ => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0204",
                            format!("unknown standard library module `{module}`"),
                            import.span,
                        )
                        .with_note("this build provides `std.io` and `std.core`")
                        .with_note(
                            "user-defined modules and package dependencies are not implemented yet",
                        ),
                    );
                    None
                }
            };
            let Some(allowed) = allowed else {
                continue;
            };
            for item in &import.items {
                let builtin = allowed
                    .iter()
                    .find(|b| b.name() == item.name)
                    .copied()
                    .or_else(|| BuiltinFn::from_name(&item.name));
                match builtin {
                    Some(builtin) => {
                        let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                        if self.aliases.insert(local.clone(), builtin).is_some() {
                            self.diagnostics.push(Diagnostic::error(
                                "E0200",
                                format!("duplicate import name `{local}`"),
                                item.name_span,
                            ));
                        }
                    }
                    None => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0204",
                            format!(
                                "`{}` is not provided by `{module}` in this build",
                                item.name
                            ),
                            item.name_span,
                        ));
                    }
                }
            }
        }
    }

    fn collect_struct_names(&mut self, program: &ast::Program) {
        for item in &program.items {
            match item {
                ast::Item::Struct(decl) => {
                    if !decl.generics.is_empty() {
                        self.diagnostics.push(Diagnostic::error(
                            "E0230",
                            "generic structs are not implemented in this build",
                            decl.name_span,
                        ));
                        continue;
                    }
                    if self.structs.contains_key(&decl.name) {
                        self.diagnostics.push(Diagnostic::error(
                            "E0200",
                            format!("duplicate struct `{}`", decl.name),
                            decl.name_span,
                        ));
                        continue;
                    }
                    self.structs.insert(
                        decl.name.clone(),
                        StructDef {
                            name: decl.name.clone(),
                            fields: Vec::new(),
                        },
                    );
                    self.struct_spans.insert(decl.name.clone(), decl.name_span);
                    self.struct_order.push(decl.name.clone());
                }
                ast::Item::Enum(decl) => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0201",
                            "user-defined enums are not implemented in this build",
                            decl.name_span,
                        )
                        .with_note(
                            "`Option<T>` and `Result<T, E>` are built in; user enums are on the roadmap",
                        ),
                    );
                }
                _ => {}
            }
        }
    }

    fn collect_struct_fields(&mut self, program: &ast::Program) {
        for item in &program.items {
            let ast::Item::Struct(decl) = item else {
                continue;
            };
            if !decl.generics.is_empty() {
                continue;
            }
            let mut fields = Vec::new();
            let mut seen = HashSet::new();
            for field in &decl.fields {
                if !seen.insert(field.name.clone()) {
                    self.diagnostics.push(Diagnostic::error(
                        "E0201",
                        format!("duplicate field `{}`", field.name),
                        field.name_span,
                    ));
                    continue;
                }
                let ty = self.resolve_type(&field.ty);
                fields.push(FieldDef {
                    name: field.name.clone(),
                    ty,
                });
            }
            self.structs.insert(
                decl.name.clone(),
                StructDef {
                    name: decl.name.clone(),
                    fields,
                },
            );
        }
    }

    fn detect_struct_cycles(&mut self) {
        let structs = self.structs.clone();
        let spans = self.struct_spans.clone();
        let names: Vec<String> = self.struct_order.clone();
        let mut state: HashMap<String, u8> = HashMap::new();
        let mut stack: Vec<String> = Vec::new();
        let mut reported: HashSet<String> = HashSet::new();
        for name in names {
            self.dfs_struct(
                &name,
                &structs,
                &spans,
                &mut state,
                &mut stack,
                &mut reported,
            );
        }
    }

    fn dfs_struct(
        &mut self,
        name: &str,
        structs: &HashMap<String, StructDef>,
        spans: &HashMap<String, Span>,
        state: &mut HashMap<String, u8>,
        stack: &mut Vec<String>,
        reported: &mut HashSet<String>,
    ) {
        match state.get(name).copied().unwrap_or(0) {
            2 => return,
            1 => {
                if reported.insert(name.to_string()) {
                    let mut cycle = stack.clone();
                    if let Some(idx) = cycle.iter().position(|n| n == name) {
                        cycle = cycle[idx..].to_vec();
                    }
                    cycle.push(name.to_string());
                    let span = spans.get(name).copied().unwrap_or_default();
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0201",
                            format!("recursive value type `{name}` has infinite size"),
                            span,
                        )
                        .with_note(format!("cycle: {}", cycle.join(" -> ")))
                        .with_note("store the cycle behind `List<T>` or break it in a later milestone with an indirection"),
                    );
                }
                return;
            }
            _ => {}
        }
        state.insert(name.to_string(), 1);
        stack.push(name.to_string());
        if let Some(def) = structs.get(name) {
            for field in &def.fields {
                let mut refs = Vec::new();
                collect_value_structs(&field.ty, &mut refs);
                for reference in refs {
                    self.dfs_struct(&reference, structs, spans, state, stack, reported);
                }
            }
        }
        stack.pop();
        state.insert(name.to_string(), 2);
    }

    fn collect_functions(&mut self, program: &ast::Program) {
        for item in &program.items {
            let ast::Item::Fn(decl) = item else {
                continue;
            };
            if !decl.generics.is_empty() {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0230",
                        "generic functions are not implemented in this build",
                        decl.name_span,
                    )
                    .with_note("generic monomorphisation is planned for milestone 6"),
                );
                continue;
            }
            if self.funcs.contains_key(&decl.name) {
                self.diagnostics.push(Diagnostic::error(
                    "E0200",
                    format!("duplicate function `{}`", decl.name),
                    decl.name_span,
                ));
                continue;
            }
            let mut params = Vec::new();
            let mut seen = HashSet::new();
            for param in &decl.params {
                if !seen.insert(param.name.clone()) {
                    self.diagnostics.push(Diagnostic::error(
                        "E0200",
                        format!("duplicate parameter `{}`", param.name),
                        param.name_span,
                    ));
                }
                params.push(self.resolve_type(&param.ty));
            }
            let ret = match &decl.ret {
                Some(ty) => self.resolve_type(ty),
                None => Type::Unit,
            };
            if decl.is_pub && decl.ret.is_none() {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0312",
                        format!("public function `{}` must declare a return type", decl.name),
                        decl.name_span,
                    )
                    .with_note("use `-> Unit` when no value is returned"),
                );
            }
            if decl.is_test {
                if !params.is_empty() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0315",
                        "@test functions must not take parameters",
                        decl.name_span,
                    ));
                }
                if ret != Type::Unit {
                    self.diagnostics.push(Diagnostic::error(
                        "E0315",
                        "@test functions must return Unit",
                        decl.name_span,
                    ));
                }
            }
            if decl.name == "main" {
                if !params.is_empty() {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0313",
                            "`main` must not take parameters in this build",
                            decl.name_span,
                        )
                        .with_note("command-line arguments are planned but not implemented yet"),
                    );
                }
                if ret != Type::Unit {
                    self.diagnostics.push(Diagnostic::error(
                        "E0313",
                        "`main` must return Unit in this build",
                        decl.name_span,
                    ));
                }
            }
            self.funcs.insert(
                decl.name.clone(),
                FnSig {
                    params,
                    ret,
                    is_test: decl.is_test,
                    span: decl.name_span,
                },
            );
            self.func_order.push(decl.name.clone());
        }
    }

    fn resolve_type(&mut self, ty: &ast::TypeExpr) -> Type {
        match &ty.kind {
            ast::TypeExprKind::Unit => Type::Unit,
            ast::TypeExprKind::Tuple(_) => {
                self.diagnostics.push(Diagnostic::error(
                    "E0201",
                    "tuple types are not implemented in this build",
                    ty.span,
                ));
                Type::Error
            }
            ast::TypeExprKind::Named { path, args } => {
                if path.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        "E0202",
                        format!(
                            "qualified type names are not supported: `{}`",
                            path.join(".")
                        ),
                        ty.span,
                    ));
                    return Type::Error;
                }
                let name = &path[0];
                match name.as_str() {
                    "Unit" => Type::Unit,
                    "Bool" => Type::Bool,
                    "Int" => Type::Int,
                    "Float" => Type::Float,
                    "Char" => Type::Char,
                    "String" => Type::String,
                    "List" => {
                        if args.len() != 1 {
                            self.diagnostics.push(Diagnostic::error(
                                "E0202",
                                format!("`List` expects 1 type argument, found {}", args.len()),
                                ty.span,
                            ));
                            return Type::Error;
                        }
                        Type::list(self.resolve_type(&args[0]))
                    }
                    "Option" => {
                        if args.len() != 1 {
                            self.diagnostics.push(Diagnostic::error(
                                "E0202",
                                format!("`Option` expects 1 type argument, found {}", args.len()),
                                ty.span,
                            ));
                            return Type::Error;
                        }
                        Type::option(self.resolve_type(&args[0]))
                    }
                    "Result" => {
                        if args.len() != 2 {
                            self.diagnostics.push(Diagnostic::error(
                                "E0202",
                                format!("`Result` expects 2 type arguments, found {}", args.len()),
                                ty.span,
                            ));
                            return Type::Error;
                        }
                        Type::result(self.resolve_type(&args[0]), self.resolve_type(&args[1]))
                    }
                    other => {
                        if self.structs.contains_key(other) {
                            if !args.is_empty() {
                                self.diagnostics.push(Diagnostic::error(
                                    "E0202",
                                    format!("struct `{other}` does not take type arguments"),
                                    ty.span,
                                ));
                            }
                            Type::Struct(other.to_string())
                        } else {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    "E0202",
                                    format!("unknown type `{other}`"),
                                    ty.span,
                                )
                                .with_note(
                                    "built-in types: Int, Float, Bool, Char, String, Unit, List<T>, Option<T>, Result<T, E>",
                                ),
                            );
                            Type::Error
                        }
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // Function bodies
    // ---------------------------------------------------------------------

    fn check_fn(&mut self, decl: &ast::FnDecl) -> Option<TypedFn> {
        let sig = self.funcs.get(&decl.name)?.clone();
        self.current = Some(FnState::new(sig.ret.clone()));
        self.next_local = 0;
        self.push_scope();

        let mut param_ids = Vec::new();
        for (param, ty) in decl.params.iter().zip(sig.params.iter()) {
            let id = self.add_local(&param.name, ty.clone(), false, true, param.name_span);
            param_ids.push(id);
        }

        let expected = if sig.ret == Type::Unit {
            None
        } else {
            Some(sig.ret.clone())
        };
        let body = self.check_block(&decl.body, expected.as_ref(), sig.ret != Type::Unit);

        if sig.ret != Type::Unit {
            let ends =
                body.tail.is_some() || body.stmts.last().map(stmt_terminates).unwrap_or(false);
            if !ends {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0314",
                        format!(
                            "function `{}` may return without a value (expected `{}`)",
                            decl.name, sig.ret
                        ),
                        decl.name_span,
                    )
                    .with_note("add a `return` on every path, or finish with an expression"),
                );
            }
        }

        self.pop_scope();
        let state = self.current.take()?;
        Some(TypedFn {
            name: decl.name.clone(),
            params: param_ids,
            ret: sig.ret,
            body,
            locals: state.locals,
            span: decl.span,
            is_test: decl.is_test,
        })
    }

    fn check_block(
        &mut self,
        block: &ast::Block,
        expected: Option<&Type>,
        use_tail: bool,
    ) -> TBlock {
        self.push_scope();
        let mut stmts = Vec::new();
        let mut tail = None;
        let count = block.stmts.len();
        for (index, stmt) in block.stmts.iter().enumerate() {
            if use_tail && index + 1 == count {
                if let StmtKind::Expr(expr) = &stmt.kind {
                    let checked = self.check_expr(expr, expected);
                    let produces_value = checked.ty != Type::Unit
                        || expected.map(|t| *t == Type::Unit).unwrap_or(true);
                    if produces_value {
                        tail = Some(Box::new(checked));
                    } else {
                        // A trailing `if` without `else` is a statement, not a
                        // value; keep it so missing-return analysis sees it.
                        stmts.push(TStmt {
                            kind: TStmtKind::Expr(checked),
                            span: stmt.span,
                        });
                    }
                    break;
                }
            }
            if let Some(checked) = self.check_stmt(stmt) {
                stmts.push(checked);
            }
        }
        if use_tail
            && tail.is_none()
            && expected.map(|t| *t != Type::Unit).unwrap_or(false)
            && !stmts.last().map(stmt_terminates).unwrap_or(false)
        {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0305",
                    format!(
                        "block does not produce a value of type `{}`",
                        expected.unwrap()
                    ),
                    block.span,
                )
                .with_note("add a final expression or return a value"),
            );
        }
        self.pop_scope();
        TBlock {
            stmts,
            tail,
            span: block.span,
        }
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt) -> Option<TStmt> {
        match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                name_span,
                ty,
                init,
            } => {
                let declared = ty.as_ref().map(|t| self.resolve_type(t));
                let init_ty = self.check_expr(init, declared.as_ref());
                let bind_ty = declared.unwrap_or_else(|| init_ty.ty.clone());
                let id = self.add_local(name, bind_ty, *mutable, false, *name_span);
                Some(TStmt {
                    kind: TStmtKind::Let {
                        local: id,
                        init: init_ty,
                    },
                    span: stmt.span,
                })
            }
            StmtKind::Assign { target, op, value } => {
                let lvalue = self.convert_lvalue(target);
                if !self.root_is_mutable(target) {
                    let span = lvalue_target_span(target);
                    let name = lvalue_target_name(target).unwrap_or_else(|| "value".to_string());
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0301",
                            format!("cannot assign to immutable binding `{name}`"),
                            span,
                        )
                        .with_note("declare the binding with `var` to make it mutable"),
                    );
                }
                let target_ty = lvalue
                    .as_ref()
                    .map(|(_, ty)| ty.clone())
                    .unwrap_or(Type::Error);
                let checked_value = self.check_expr(value, Some(&target_ty));
                if let Some((target, ty)) = lvalue {
                    if *op != ast::AssignOp::Assign {
                        let ok = match op {
                            ast::AssignOp::Add => {
                                ty == Type::String || ty == Type::Int || ty == Type::Float
                            }
                            ast::AssignOp::Sub
                            | ast::AssignOp::Mul
                            | ast::AssignOp::Div
                            | ast::AssignOp::Rem => ty.is_numeric(),
                            ast::AssignOp::Assign => true,
                        };
                        if !ok && !ty.is_error() {
                            self.diagnostics.push(Diagnostic::error(
                                "E0306",
                                format!("assignment operator is not defined for `{ty}`"),
                                target_span(&target, stmt.span),
                            ));
                        }
                    }
                    Some(TStmt {
                        kind: TStmtKind::Assign {
                            target,
                            op: *op,
                            value: checked_value,
                        },
                        span: stmt.span,
                    })
                } else {
                    Some(TStmt {
                        kind: TStmtKind::Expr(checked_value),
                        span: stmt.span,
                    })
                }
            }
            StmtKind::Expr(expr) => {
                let checked = self.check_expr(expr, None);
                Some(TStmt {
                    kind: TStmtKind::Expr(checked),
                    span: stmt.span,
                })
            }
            StmtKind::While { cond, body } => {
                let cond = self.check_expr(cond, Some(&Type::Bool));
                self.state_mut().loop_depth += 1;
                let body = self.check_block(body, None, false);
                self.state_mut().loop_depth = self.state_mut().loop_depth.saturating_sub(1);
                Some(TStmt {
                    kind: TStmtKind::While { cond, body },
                    span: stmt.span,
                })
            }
            StmtKind::For {
                name,
                name_span,
                iter,
                body,
            } => {
                if let ExprKind::Binary { op, lhs, rhs } = &iter.kind {
                    if matches!(op, BinaryOp::Range | BinaryOp::RangeInclusive) {
                        let start = self.check_expr(lhs, Some(&Type::Int));
                        let end = self.check_expr(rhs, Some(&Type::Int));
                        self.push_scope();
                        let var = self.add_local(name, Type::Int, false, false, *name_span);
                        self.state_mut().loop_depth += 1;
                        let body = self.check_block(body, None, false);
                        self.state_mut().loop_depth = self.state_mut().loop_depth.saturating_sub(1);
                        self.pop_scope();
                        return Some(TStmt {
                            kind: TStmtKind::ForRange {
                                var,
                                start,
                                end,
                                inclusive: *op == BinaryOp::RangeInclusive,
                                body,
                            },
                            span: stmt.span,
                        });
                    }
                }
                let list = self.check_expr(iter, None);
                let elem_ty = match &list.ty {
                    Type::List(elem) => (**elem).clone(),
                    Type::Error => Type::Error,
                    other => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0307",
                                format!("cannot iterate over `{other}`"),
                                iter.span,
                            )
                            .with_note("`for` accepts an Int range (`0..10`) or a `List<T>`"),
                        );
                        Type::Error
                    }
                };
                self.push_scope();
                let var = self.add_local(name, elem_ty, false, false, *name_span);
                self.state_mut().loop_depth += 1;
                let body = self.check_block(body, None, false);
                self.state_mut().loop_depth = self.state_mut().loop_depth.saturating_sub(1);
                self.pop_scope();
                Some(TStmt {
                    kind: TStmtKind::ForList { var, list, body },
                    span: stmt.span,
                })
            }
            StmtKind::Return(value) => {
                let ret = self.state().ret.clone();
                let checked = match value {
                    Some(expr) => {
                        if ret == Type::Unit {
                            self.diagnostics.push(Diagnostic::error(
                                "E0304",
                                "cannot return a value from a function returning Unit",
                                expr.span,
                            ));
                        }
                        Some(self.check_expr(expr, Some(&ret)))
                    }
                    None => {
                        if ret != Type::Unit && !ret.is_error() {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    "E0304",
                                    format!("return value expected: function returns `{ret}`"),
                                    stmt.span,
                                )
                                .with_note("write `return <expr>`"),
                            );
                        }
                        None
                    }
                };
                Some(TStmt {
                    kind: TStmtKind::Return(checked),
                    span: stmt.span,
                })
            }
            StmtKind::Break => {
                if self.state().loop_depth == 0 {
                    self.diagnostics.push(Diagnostic::error(
                        "E0302",
                        "`break` outside of a loop",
                        stmt.span,
                    ));
                }
                Some(TStmt {
                    kind: TStmtKind::Break,
                    span: stmt.span,
                })
            }
            StmtKind::Continue => {
                if self.state().loop_depth == 0 {
                    self.diagnostics.push(Diagnostic::error(
                        "E0302",
                        "`continue` outside of a loop",
                        stmt.span,
                    ));
                }
                Some(TStmt {
                    kind: TStmtKind::Continue,
                    span: stmt.span,
                })
            }
        }
    }

    // ---------------------------------------------------------------------
    // Expressions
    // ---------------------------------------------------------------------

    fn check_expr(&mut self, expr: &ast::Expr, expected: Option<&Type>) -> TExpr {
        let span = expr.span;
        match &expr.kind {
            ExprKind::Int(value) => self.finish(
                TExpr::new(TExprKind::Int(*value), Type::Int, span),
                expected,
            ),
            ExprKind::Float(value) => self.finish(
                TExpr::new(TExprKind::Float(*value), Type::Float, span),
                expected,
            ),
            ExprKind::Bool(value) => self.finish(
                TExpr::new(TExprKind::Bool(*value), Type::Bool, span),
                expected,
            ),
            ExprKind::Char(value) => self.finish(
                TExpr::new(TExprKind::Char(*value), Type::Char, span),
                expected,
            ),
            ExprKind::Str(parts) => {
                let mut checked = Vec::new();
                for part in parts {
                    match part {
                        ast::StrPart::Literal(text) => checked.push(TStrPart {
                            kind: TStrPartKind::Literal(text.clone()),
                        }),
                        ast::StrPart::Expr(inner) => {
                            let te = self.check_expr(inner, None);
                            if !te.ty.is_printable() && !te.ty.is_error() {
                                self.diagnostics.push(
                                    Diagnostic::error(
                                        "E0320",
                                        format!("`{}` cannot be interpolated into a string", te.ty),
                                        inner.span,
                                    )
                                    .with_note(
                                        "interpolation supports Bool, Int, Float, Char and String",
                                    ),
                                );
                            }
                            checked.push(TStrPart {
                                kind: TStrPartKind::Expr(te),
                            });
                        }
                    }
                }
                self.finish(
                    TExpr::new(TExprKind::String(checked), Type::String, span),
                    expected,
                )
            }
            ExprKind::Unit => self.finish(TExpr::new(TExprKind::Unit, Type::Unit, span), expected),
            ExprKind::Paren(inner) => self.check_expr(inner, expected),
            ExprKind::Ident(name) => self.check_ident(name, span, expected),
            ExprKind::Unary { op, expr: inner } => {
                let operand = self.check_expr(inner, None);
                let ty = match op {
                    UnaryOp::Neg => {
                        if operand.ty.is_numeric() || operand.ty.is_error() {
                            operand.ty.clone()
                        } else {
                            self.diagnostics.push(Diagnostic::error(
                                "E0306",
                                format!("unary `-` is not defined for `{}`", operand.ty),
                                span,
                            ));
                            Type::Error
                        }
                    }
                    UnaryOp::Not => {
                        self.expect_type(&operand.ty, &Type::Bool, inner.span);
                        Type::Bool
                    }
                };
                self.finish(
                    TExpr::new(
                        TExprKind::Unary {
                            op: *op,
                            expr: Box::new(operand),
                        },
                        ty,
                        span,
                    ),
                    expected,
                )
            }
            ExprKind::Binary { op, lhs, rhs } => self.check_binary(*op, lhs, rhs, span, expected),
            ExprKind::Call { callee, args } => self.check_call(callee, args, span, expected),
            ExprKind::MethodCall {
                receiver,
                method,
                method_span,
                args,
            } => self.check_method(receiver, method, *method_span, args, span, expected),
            ExprKind::Field {
                base,
                name,
                name_span,
            } => {
                let checked_base = self.check_expr(base, None);
                let ty = if let Type::Struct(struct_name) = &checked_base.ty {
                    if let Some(def) = self.structs.get(struct_name) {
                        if let Some((index, field)) =
                            def.fields.iter().enumerate().find(|(_, f)| &f.name == name)
                        {
                            let field_ty = field.ty.clone();
                            return self.finish(
                                TExpr::new(
                                    TExprKind::Field {
                                        base: Box::new(checked_base),
                                        index,
                                    },
                                    field_ty,
                                    span,
                                ),
                                expected,
                            );
                        }
                        self.diagnostics.push(Diagnostic::error(
                            "E0205",
                            format!("struct `{struct_name}` has no field `{name}`"),
                            *name_span,
                        ));
                    }
                    Type::Error
                } else if checked_base.ty.is_error() {
                    Type::Error
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        "E0205",
                        format!(
                            "field access `.{}` requires a struct, found `{}`",
                            name, checked_base.ty
                        ),
                        *name_span,
                    ));
                    Type::Error
                };
                TExpr::new(
                    TExprKind::Field {
                        base: Box::new(checked_base),
                        index: 0,
                    },
                    ty,
                    span,
                )
            }
            ExprKind::Index { base, index } => {
                let checked_base = self.check_expr(base, None);
                let index = self.check_expr(index, Some(&Type::Int));
                let ty = match &checked_base.ty {
                    Type::List(elem) => (**elem).clone(),
                    Type::Error => Type::Error,
                    other => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0205",
                                format!("indexing is not defined for `{other}`"),
                                span,
                            )
                            .with_note("only `List<T>` supports indexing; use `.get(i)` for a checked Option"),
                        );
                        Type::Error
                    }
                };
                self.finish(
                    TExpr::new(
                        TExprKind::Index {
                            base: Box::new(checked_base),
                            index: Box::new(index),
                        },
                        ty,
                        span,
                    ),
                    expected,
                )
            }
            ExprKind::StructLit {
                name,
                name_span,
                fields,
            } => self.check_struct_lit(name, *name_span, fields, span, expected),
            ExprKind::If {
                cond,
                then_block,
                else_branch,
            } => self.check_if(cond, then_block, else_branch, span, expected),
            ExprKind::Match { scrutinee, arms } => {
                self.check_match(scrutinee, arms, span, expected)
            }
            ExprKind::Block(block) => {
                let checked = self.check_block(block, expected, true);
                let ty = checked
                    .tail
                    .as_ref()
                    .map(|e| e.ty.clone())
                    .unwrap_or(Type::Unit);
                TExpr::new(TExprKind::Block(checked), ty, span)
            }
            ExprKind::List(items) => {
                let mut elem_ty = match expected {
                    Some(Type::List(inner)) => Some((**inner).clone()),
                    Some(other) if !other.is_error() => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0308",
                            format!("expected `{other}`, found `List<_>`"),
                            span,
                        ));
                        None
                    }
                    _ => None,
                };
                let mut checked = Vec::new();
                for item in items {
                    let te = self.check_expr(item, elem_ty.as_ref());
                    if elem_ty.is_none() {
                        elem_ty = Some(te.ty.clone());
                    }
                    checked.push(te);
                }
                let elem_ty = elem_ty.unwrap_or_else(|| {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0210",
                            "cannot infer the element type of an empty list",
                            span,
                        )
                        .with_note("add a type annotation, e.g. `let xs: List<Int> = []`"),
                    );
                    Type::Error
                });
                TExpr::new(TExprKind::List(checked), Type::list(elem_ty), span)
            }
            ExprKind::Range { .. } => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0209",
                        "ranges are only valid as the iterable of a `for` loop",
                        span,
                    )
                    .with_note("write `for i in 0..10 { ... }`"),
                );
                TExpr::new(TExprKind::Unit, Type::Error, span)
            }
            ExprKind::Try(inner) => {
                let operand = self.check_expr(inner, None);
                let ret = self.state().ret.clone();
                let ty = match (&operand.ty, &ret) {
                    (Type::Option(inner_ty), Type::Option(_)) => (**inner_ty).clone(),
                    (Type::Result(ok, err), Type::Result(_, ret_err)) => {
                        if err != ret_err && !err.is_error() && !ret_err.is_error() {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    "E0306",
                                    format!(
                                        "`?` cannot propagate error type `{err}` from a function returning `Result<_, {ret_err}>`"
                                    ),
                                    span,
                                )
                                .with_note("the error types must match exactly; there is no implicit conversion"),
                            );
                        }
                        (**ok).clone()
                    }
                    (Type::Error, _) => Type::Error,
                    (Type::Option(_), other) => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0306",
                                format!("`?` on Option requires the function to return Option, found `{other}`"),
                                span,
                            ),
                        );
                        Type::Error
                    }
                    (Type::Result(_, _), other) => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0306",
                                format!("`?` on Result requires the function to return Result, found `{other}`"),
                                span,
                            ),
                        );
                        Type::Error
                    }
                    (other, _) => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0306",
                            format!("`?` is not defined for `{other}`"),
                            span,
                        ));
                        Type::Error
                    }
                };
                self.finish(
                    TExpr::new(TExprKind::Try(Box::new(operand)), ty, span),
                    expected,
                )
            }
            ExprKind::Error => TExpr::new(TExprKind::Unit, Type::Error, span),
        }
    }

    fn check_ident(&mut self, name: &str, span: Span, expected: Option<&Type>) -> TExpr {
        if let Some(local) = self.lookup(name) {
            let ty = local.ty.clone();
            return self.finish(TExpr::new(TExprKind::Local(local.id), ty, span), expected);
        }
        if name == "None" {
            let inner = match expected {
                Some(Type::Option(inner)) => (**inner).clone(),
                Some(other) if !other.is_error() => {
                    self.diagnostics.push(Diagnostic::error(
                        "E0308",
                        format!("expected `{other}`, found `Option<_>`"),
                        span,
                    ));
                    Type::Error
                }
                Some(_) => Type::Error,
                _ => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0210",
                            "cannot infer the type parameter of `None`",
                            span,
                        )
                        .with_note("annotate the binding or use it where `Option<T>` is expected"),
                    );
                    Type::Error
                }
            };
            return TExpr::new(TExprKind::OptionNone, Type::option(inner), span);
        }
        if matches!(name, "Some" | "Ok" | "Err") {
            self.diagnostics.push(Diagnostic::error(
                "E0205",
                format!("`{name}` is a constructor and requires an argument"),
                span,
            ));
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        if self.funcs.contains_key(name) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0205",
                    format!("function `{name}` cannot be used as a value"),
                    span,
                )
                .with_note("first-class functions and closures are not implemented yet"),
            );
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        if self.structs.contains_key(name) {
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        self.diagnostics.push(
            Diagnostic::error("E0203", format!("unknown name `{name}`"), span)
                .with_note("check the spelling, or declare the binding before using it"),
        );
        TExpr::new(TExprKind::Unit, Type::Error, span)
    }

    fn check_binary(
        &mut self,
        op: BinaryOp,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let left = self.check_expr(lhs, None);
        let right = match op {
            BinaryOp::And | BinaryOp::Or => {
                self.expect_type(&left.ty, &Type::Bool, lhs.span);
                self.check_expr(rhs, Some(&Type::Bool))
            }
            BinaryOp::Range | BinaryOp::RangeInclusive => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0209",
                        "ranges are only valid as the iterable of a `for` loop",
                        span,
                    )
                    .with_note("write `for i in 0..10 { ... }`"),
                );
                self.check_expr(rhs, None)
            }
            _ => {
                if !left.ty.is_error() {
                    self.check_expr(rhs, Some(&left.ty))
                } else {
                    self.check_expr(rhs, None)
                }
            }
        };
        let ty = match op {
            BinaryOp::And | BinaryOp::Or | BinaryOp::Eq | BinaryOp::NotEq => Type::Bool,
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                if !left.ty.is_ordered_scalar() && !left.ty.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0306",
                        format!("comparison is not defined for `{}`", left.ty),
                        span,
                    ));
                }
                Type::Bool
            }
            BinaryOp::Add => {
                if left.ty == Type::String {
                    Type::String
                } else if left.ty.is_numeric() {
                    left.ty.clone()
                } else if !left.ty.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0306",
                        format!("operator `+` is not defined for `{}`", left.ty),
                        span,
                    ));
                    Type::Error
                } else {
                    Type::Error
                }
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                if left.ty.is_numeric() {
                    left.ty.clone()
                } else if !left.ty.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0306",
                        format!("this arithmetic operator is not defined for `{}`", left.ty),
                        span,
                    ));
                    Type::Error
                } else {
                    Type::Error
                }
            }
            BinaryOp::Range | BinaryOp::RangeInclusive => Type::Error,
        };
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
            && !left.ty.is_equatable()
            && !left.ty.is_error()
        {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0306",
                    format!("equality is not defined for `{}`", left.ty),
                    span,
                )
                .with_note("equality supports Bool, Int, Float, Char, String and Unit"),
            );
        }
        self.finish(
            TExpr::new(
                TExprKind::Binary {
                    op,
                    lhs: Box::new(left),
                    rhs: Box::new(right),
                },
                ty,
                span,
            ),
            expected,
        )
    }

    fn check_call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Arg],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let ExprKind::Ident(name) = &callee.kind else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0205",
                    "only named functions can be called in this build",
                    callee.span,
                )
                .with_note("closures and function values are not implemented yet"),
            );
            for arg in args {
                self.check_expr(&arg.value, None);
            }
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        };

        if name == "Some" || name == "Ok" || name == "Err" || name == "None" {
            return self.check_constructor(name, args, span, expected);
        }

        for arg in args {
            if let Some(span) = arg.name_span {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0208",
                        "named arguments are not implemented in this build",
                        span,
                    )
                    .with_note("pass arguments positionally"),
                );
            }
        }

        let builtin = BuiltinFn::from_name(name).or_else(|| self.aliases.get(name).copied());
        if let Some(builtin) = builtin {
            return self.check_builtin(builtin, args, span, expected);
        }

        let Some(sig) = self.funcs.get(name).cloned() else {
            if self.structs.contains_key(name) {
                self.diagnostics.push(Diagnostic::error(
                    "E0205",
                    format!("`{name}` is a struct; construct it with `{name} {{ ... }}`"),
                    callee.span,
                ));
            } else {
                self.diagnostics.push(Diagnostic::error(
                    "E0203",
                    format!("unknown function `{name}`"),
                    callee.span,
                ));
            }
            for arg in args {
                self.check_expr(&arg.value, None);
            }
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        };

        if sig.is_test {
            self.diagnostics.push(Diagnostic::error(
                "E0205",
                format!("`{name}` is a test function and cannot be called directly"),
                callee.span,
            ));
        }
        if args.len() != sig.params.len() {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0208",
                    format!(
                        "`{name}` expects {} argument{}, found {}",
                        sig.params.len(),
                        if sig.params.len() == 1 { "" } else { "s" },
                        args.len()
                    ),
                    span,
                )
                .with_label(callee.span, format!("`{name}` declared here")),
            );
        }
        let mut checked = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            let expected_arg = sig.params.get(index);
            let te = self.check_expr(&arg.value, expected_arg);
            checked.push(te);
        }
        for (index, te) in checked.iter().enumerate() {
            if let Some(expected_arg) = sig.params.get(index) {
                if te.ty != *expected_arg && !te.ty.is_error() && !expected_arg.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0308",
                        format!(
                            "argument {} to `{name}` has type `{}`, expected `{expected_arg}`",
                            index + 1,
                            te.ty
                        ),
                        te.span,
                    ));
                }
            }
        }
        self.finish(
            TExpr::new(
                TExprKind::Call {
                    name: name.clone(),
                    args: checked,
                },
                sig.ret.clone(),
                span,
            ),
            expected,
        )
    }

    fn check_constructor(
        &mut self,
        name: &str,
        args: &[ast::Arg],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        match name {
            "None" => {
                self.diagnostics.push(Diagnostic::error(
                    "E0205",
                    "`None` does not take arguments",
                    span,
                ));
                TExpr::new(TExprKind::OptionNone, Type::option(Type::Error), span)
            }
            "Some" => {
                if args.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        "E0208",
                        format!("`Some` expects 1 argument, found {}", args.len()),
                        span,
                    ));
                    for arg in args {
                        self.check_expr(&arg.value, None);
                    }
                    return TExpr::new(TExprKind::OptionNone, Type::option(Type::Error), span);
                }
                let inner_expected = match expected {
                    Some(Type::Option(inner)) => Some((**inner).clone()),
                    _ => None,
                };
                let inner = self.check_expr(&args[0].value, inner_expected.as_ref());
                let ty = match expected {
                    Some(Type::Option(inner)) => Type::option((**inner).clone()),
                    Some(other) if !other.is_error() => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0308",
                            format!("expected `{other}`, found `Option<{}>`", inner.ty),
                            span,
                        ));
                        Type::option(inner.ty.clone())
                    }
                    _ => Type::option(inner.ty.clone()),
                };
                TExpr::new(TExprKind::OptionSome(Box::new(inner)), ty, span)
            }
            "Ok" | "Err" => {
                if args.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        "E0208",
                        format!("`{name}` expects 1 argument, found {}", args.len()),
                        span,
                    ));
                    for arg in args {
                        self.check_expr(&arg.value, None);
                    }
                    let ty = expected
                        .cloned()
                        .unwrap_or(Type::result(Type::Error, Type::Error));
                    return TExpr::new(
                        TExprKind::ResultOk(Box::new(TExpr::new(
                            TExprKind::Unit,
                            Type::Error,
                            span,
                        ))),
                        ty,
                        span,
                    );
                }
                let expected_ty = match expected {
                    Some(Type::Result(ok, err)) => {
                        if name == "Ok" {
                            Some((**ok).clone())
                        } else {
                            Some((**err).clone())
                        }
                    }
                    Some(other) if !other.is_error() => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0308",
                            format!("expected `{other}`, found `{name}(_)`"),
                            span,
                        ));
                        None
                    }
                    _ => None,
                };
                let value = self.check_expr(&args[0].value, expected_ty.as_ref());
                let ty = match expected {
                    Some(Type::Result(ok, err)) => Type::result((**ok).clone(), (**err).clone()),
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0210",
                                format!(
                                    "cannot infer the full `Result` type from `{name}` alone"
                                ),
                                span,
                            )
                            .with_note("add a type annotation or return it from a function with a `Result` return type"),
                        );
                        if name == "Ok" {
                            Type::result(value.ty.clone(), Type::Error)
                        } else {
                            Type::result(Type::Error, value.ty.clone())
                        }
                    }
                };
                let kind = if name == "Ok" {
                    TExprKind::ResultOk(Box::new(value))
                } else {
                    TExprKind::ResultErr(Box::new(value))
                };
                TExpr::new(kind, ty, span)
            }
            _ => unreachable!(),
        }
    }

    fn check_builtin(
        &mut self,
        builtin: BuiltinFn,
        args: &[ast::Arg],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let expected_count = match builtin {
            BuiltinFn::AssertEq => 2,
            _ => 1,
        };
        if args.len() != expected_count {
            self.diagnostics.push(Diagnostic::error(
                "E0208",
                format!(
                    "`{}` expects {} argument{}, found {}",
                    builtin.name(),
                    expected_count,
                    if expected_count == 1 { "" } else { "s" },
                    args.len()
                ),
                span,
            ));
        }
        let mut checked = Vec::new();
        match builtin {
            BuiltinFn::Println | BuiltinFn::Print | BuiltinFn::Eprintln => {
                if let Some(arg) = args.first() {
                    let te = self.check_expr(&arg.value, None);
                    if !te.ty.is_printable() && !te.ty.is_error() {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0320",
                                format!("`{}` cannot print `{}`", builtin.name(), te.ty),
                                te.span,
                            )
                            .with_note("printable types: Bool, Int, Float, Char, String"),
                        );
                    }
                    checked.push(te);
                }
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: checked,
                        },
                        Type::Unit,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::Assert => {
                if let Some(arg) = args.first() {
                    checked.push(self.check_expr(&arg.value, Some(&Type::Bool)));
                }
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: checked,
                        },
                        Type::Unit,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::Panic => {
                if let Some(arg) = args.first() {
                    checked.push(self.check_expr(&arg.value, Some(&Type::String)));
                }
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: checked,
                        },
                        Type::Never,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::AssertEq => {
                let left = args
                    .first()
                    .map(|a| self.check_expr(&a.value, None))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                let right = args
                    .get(1)
                    .map(|a| self.check_expr(&a.value, Some(&left.ty)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                if !left.ty.is_equatable() && !left.ty.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0306",
                        format!("`assert_eq` does not support `{}`", left.ty),
                        span,
                    ));
                }
                if left.ty != right.ty && !left.ty.is_error() && !right.ty.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0308",
                        format!(
                            "`assert_eq` arguments have different types (`{}` and `{}`)",
                            left.ty, right.ty
                        ),
                        span,
                    ));
                }
                checked.push(left);
                checked.push(right);
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: checked,
                        },
                        Type::Unit,
                        span,
                    ),
                    expected,
                )
            }
        }
    }

    fn check_method(
        &mut self,
        receiver: &ast::Expr,
        method: &str,
        method_span: Span,
        args: &[ast::Arg],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        for arg in args {
            if let Some(span) = arg.name_span {
                self.diagnostics.push(Diagnostic::error(
                    "E0208",
                    "named arguments are not implemented in this build",
                    span,
                ));
            }
        }
        let recv = self.check_expr(receiver, None);
        let recv_ty = recv.ty.clone();
        match &recv_ty {
            Type::List(elem) => match method {
                "len" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ListLen,
                            args: Vec::new(),
                        },
                        Type::Int,
                        span,
                    );
                    self.finish(te, expected)
                }
                "is_empty" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ListIsEmpty,
                            args: Vec::new(),
                        },
                        Type::Bool,
                        span,
                    );
                    self.finish(te, expected)
                }
                "get" => {
                    self.require_args(method, args, 1, span);
                    let index = args
                        .first()
                        .map(|a| self.check_expr(&a.value, Some(&Type::Int)))
                        .map(|e| vec![e])
                        .unwrap_or_default();
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ListGet,
                            args: index,
                        },
                        Type::option((**elem).clone()),
                        span,
                    );
                    self.finish(te, expected)
                }
                "push" => {
                    self.require_args(method, args, 1, span);
                    let value = args
                        .first()
                        .map(|a| self.check_expr(&a.value, Some(elem.as_ref())));
                    if !self.root_is_mutable(receiver) {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0301",
                                "`push` requires a mutable list binding",
                                receiver.span,
                            )
                            .with_note("declare the list with `var`"),
                        );
                    }
                    let te = match (self.convert_lvalue(receiver), value) {
                        (Some((list, _)), Some(value)) => TExpr::new(
                            TExprKind::ListPush {
                                list,
                                value: Box::new(value),
                            },
                            Type::Unit,
                            span,
                        ),
                        _ => TExpr::new(TExprKind::Unit, Type::Error, span),
                    };
                    self.finish(te, expected)
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::String => match method {
                "len" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::StringLen,
                            args: Vec::new(),
                        },
                        Type::Int,
                        span,
                    );
                    self.finish(te, expected)
                }
                "is_empty" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::StringIsEmpty,
                            args: Vec::new(),
                        },
                        Type::Bool,
                        span,
                    );
                    self.finish(te, expected)
                }
                "to_string" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv.clone()),
                            method: BuiltinMethod::ToString,
                            args: Vec::new(),
                        },
                        Type::String,
                        span,
                    );
                    self.finish(te, expected)
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::Option(inner) => match method {
                "is_some" | "is_none" => {
                    self.require_args(method, args, 0, span);
                    let m = if method == "is_some" {
                        BuiltinMethod::OptionIsSome
                    } else {
                        BuiltinMethod::OptionIsNone
                    };
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: m,
                            args: Vec::new(),
                        },
                        Type::Bool,
                        span,
                    );
                    self.finish(te, expected)
                }
                "unwrap_or" => {
                    self.require_args(method, args, 1, span);
                    let arg = args
                        .first()
                        .map(|a| self.check_expr(&a.value, Some(inner.as_ref())))
                        .map(|e| vec![e])
                        .unwrap_or_default();
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::OptionUnwrapOr,
                            args: arg,
                        },
                        (**inner).clone(),
                        span,
                    );
                    self.finish(te, expected)
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::Result(ok, _) => match method {
                "is_ok" | "is_err" => {
                    self.require_args(method, args, 0, span);
                    let m = if method == "is_ok" {
                        BuiltinMethod::ResultIsOk
                    } else {
                        BuiltinMethod::ResultIsErr
                    };
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: m,
                            args: Vec::new(),
                        },
                        Type::Bool,
                        span,
                    );
                    self.finish(te, expected)
                }
                "unwrap_or" => {
                    self.require_args(method, args, 1, span);
                    let arg = args
                        .first()
                        .map(|a| self.check_expr(&a.value, Some(ok.as_ref())))
                        .map(|e| vec![e])
                        .unwrap_or_default();
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ResultUnwrapOr,
                            args: arg,
                        },
                        (**ok).clone(),
                        span,
                    );
                    self.finish(te, expected)
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::Int | Type::Float | Type::Bool | Type::Char => match method {
                "to_string" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ToString,
                            args: Vec::new(),
                        },
                        Type::String,
                        span,
                    );
                    self.finish(te, expected)
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::Error => TExpr::new(TExprKind::Unit, Type::Error, span),
            other => {
                self.diagnostics.push(Diagnostic::error(
                    "E0206",
                    format!("type `{other}` has no method `{method}`"),
                    method_span,
                ));
                for arg in args {
                    self.check_expr(&arg.value, None);
                }
                TExpr::new(TExprKind::Unit, Type::Error, span)
            }
        }
    }

    fn unknown_method(&mut self, method: &str, ty: &Type, span: Span, expr_span: Span) -> TExpr {
        self.diagnostics.push(
            Diagnostic::error(
                "E0206",
                format!("type `{ty}` has no method `{method}`"),
                span,
            )
            .with_note("methods are limited to the built-in standard library in this build"),
        );
        TExpr::new(TExprKind::Unit, Type::Error, expr_span)
    }

    fn require_args(&mut self, method: &str, args: &[ast::Arg], expected: usize, span: Span) {
        if args.len() != expected {
            self.diagnostics.push(Diagnostic::error(
                "E0208",
                format!(
                    "`{method}` expects {expected} argument{}, found {}",
                    if expected == 1 { "" } else { "s" },
                    args.len()
                ),
                span,
            ));
        }
    }

    fn check_struct_lit(
        &mut self,
        name: &str,
        name_span: Span,
        fields: &[ast::FieldInit],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let Some(def) = self.structs.get(name).cloned() else {
            self.diagnostics.push(Diagnostic::error(
                "E0202",
                format!("unknown struct `{name}`"),
                name_span,
            ));
            for field in fields {
                self.check_expr(&field.value, None);
            }
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        };
        if let Some(expected) = expected {
            if !expected.is_error() && *expected != Type::Struct(name.to_string()) {
                self.diagnostics.push(Diagnostic::error(
                    "E0308",
                    format!("expected `{expected}`, found `{name}`"),
                    name_span,
                ));
            }
        }
        let mut provided: HashMap<&str, &ast::FieldInit> = HashMap::new();
        for field in fields {
            if provided.insert(field.name.as_str(), field).is_some() {
                self.diagnostics.push(Diagnostic::error(
                    "E0207",
                    format!("field `{}` is specified more than once", field.name),
                    field.name_span,
                ));
            }
            if !def.fields.iter().any(|f| f.name == field.name) {
                self.diagnostics.push(Diagnostic::error(
                    "E0207",
                    format!("struct `{name}` has no field `{}`", field.name),
                    field.name_span,
                ));
            }
        }
        let mut checked_fields = Vec::new();
        for (index, field_def) in def.fields.iter().enumerate() {
            match provided.get(field_def.name.as_str()) {
                Some(init) => {
                    let checked = self.check_expr(&init.value, Some(&field_def.ty));
                    checked_fields.push((index, checked));
                }
                None => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0207",
                            format!("missing field `{}` in `{name}` literal", field_def.name),
                            span,
                        )
                        .with_note("all struct fields must be initialised"),
                    );
                }
            }
        }
        TExpr::new(
            TExprKind::StructLit {
                name: name.to_string(),
                fields: checked_fields,
            },
            Type::Struct(name.to_string()),
            span,
        )
    }

    fn check_if(
        &mut self,
        cond: &ast::Expr,
        then_block: &ast::Block,
        else_branch: &Option<ElseBranch>,
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let cond = self.check_expr(cond, Some(&Type::Bool));
        match else_branch {
            Some(else_branch) => {
                let then_block = self.check_block(then_block, expected, true);
                let then_ty = then_block
                    .tail
                    .as_ref()
                    .map(|e| e.ty.clone())
                    .unwrap_or(Type::Unit);
                let else_expr = match else_branch {
                    ElseBranch::Block(block) => {
                        let checked = self.check_block(block, expected, true);
                        let ty = checked
                            .tail
                            .as_ref()
                            .map(|e| e.ty.clone())
                            .unwrap_or(Type::Unit);
                        TExpr::new(TExprKind::Block(checked), ty, block.span)
                    }
                    ElseBranch::If(expr) => self.check_expr(expr, expected),
                };
                let else_ty = else_expr.ty.clone();
                if then_ty != else_ty && !then_ty.is_error() && !else_ty.is_error() {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0308",
                            format!(
                                "`if` branches have incompatible types (`{then_ty}` and `{else_ty}`)"
                            ),
                            span,
                        )
                        .with_note("both branches of an `if` expression must produce the same type"),
                    );
                }
                let ty = if !then_ty.is_error() {
                    then_ty
                } else {
                    else_ty
                };
                TExpr::new(
                    TExprKind::If {
                        cond: Box::new(cond),
                        then_block,
                        else_branch: Some(Box::new(else_expr)),
                    },
                    ty,
                    span,
                )
            }
            None => {
                let then_block = self.check_block(then_block, None, false);
                if let Some(expected) = expected {
                    if *expected != Type::Unit && !expected.is_error() {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0305",
                                format!(
                                    "`if` without `else` produces Unit, but `{expected}` was expected"
                                ),
                                span,
                            )
                            .with_note("add an `else` branch that produces a value"),
                        );
                    }
                }
                TExpr::new(
                    TExprKind::If {
                        cond: Box::new(cond),
                        then_block,
                        else_branch: None,
                    },
                    Type::Unit,
                    span,
                )
            }
        }
    }

    fn check_match(
        &mut self,
        scrutinee: &ast::Expr,
        arms: &[ast::MatchArm],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let scrutinee = self.check_expr(scrutinee, None);
        let mut coverage = Coverage::default();
        let mut checked_arms = Vec::new();
        let mut arm_ty: Option<Type> = expected.cloned();
        for arm in arms {
            self.push_scope();
            let pattern = self.check_pattern(&arm.pattern, &scrutinee.ty, &mut coverage);
            let body = self.check_expr(&arm.body, arm_ty.as_ref());
            if arm_ty.is_none() {
                arm_ty = Some(body.ty.clone());
            } else if let Some(existing) = &arm_ty {
                if *existing != body.ty && !existing.is_error() && !body.ty.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0308",
                        format!(
                            "match arms have incompatible types (`{existing}` and `{}`)",
                            body.ty
                        ),
                        arm.body.span,
                    ));
                }
            }
            checked_arms.push(TArm {
                pattern,
                body,
                span: arm.span,
            });
            self.pop_scope();
        }
        self.check_exhaustive(&scrutinee.ty, &coverage, span);
        let ty = arm_ty.unwrap_or(Type::Error);
        TExpr::new(
            TExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: checked_arms,
            },
            ty,
            span,
        )
    }

    fn check_pattern(
        &mut self,
        pattern: &ast::Pattern,
        scrutinee: &Type,
        coverage: &mut Coverage,
    ) -> TPattern {
        match &pattern.kind {
            PatternKind::Wildcard => {
                coverage.wildcard = true;
                TPattern::Wildcard
            }
            PatternKind::Bind(name) => {
                coverage.wildcard = true;
                let id = self.add_local(name, scrutinee.clone(), false, false, pattern.span);
                TPattern::Bind(id)
            }
            PatternKind::Literal(lit) => {
                coverage.literal = true;
                match (lit, scrutinee) {
                    (PatternLiteral::Int(value), Type::Int) => TPattern::Int(*value),
                    (PatternLiteral::Bool(value), Type::Bool) => {
                        if *value {
                            coverage.bool_true = true;
                        } else {
                            coverage.bool_false = true;
                        }
                        TPattern::Bool(*value)
                    }
                    (PatternLiteral::Char(value), Type::Char) => TPattern::Char(*value),
                    (PatternLiteral::Str(value), Type::String) => TPattern::String(value.clone()),
                    (_, Type::Error) => TPattern::Wildcard,
                    (lit, ty) => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0311",
                            format!("pattern `{lit:?}` cannot match a value of type `{ty}`"),
                            pattern.span,
                        ));
                        TPattern::Wildcard
                    }
                }
            }
            PatternKind::Variant {
                name,
                name_span,
                inner,
            } => match scrutinee {
                Type::Option(payload) => match name.as_str() {
                    "Some" => {
                        coverage.some = true;
                        let id =
                            self.bind_payload(inner, payload, *name_span, pattern.span, "Some");
                        TPattern::Some(id)
                    }
                    "None" => {
                        coverage.none = true;
                        if inner.is_some() {
                            self.diagnostics.push(Diagnostic::error(
                                "E0311",
                                "`None` does not take a payload pattern",
                                pattern.span,
                            ));
                        }
                        TPattern::None
                    }
                    other => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0311",
                            format!("`{other}` is not a variant of `Option`"),
                            *name_span,
                        ));
                        TPattern::Wildcard
                    }
                },
                Type::Result(ok, err) => match name.as_str() {
                    "Ok" => {
                        coverage.ok = true;
                        let id = self.bind_payload(inner, ok, *name_span, pattern.span, "Ok");
                        TPattern::Ok(id)
                    }
                    "Err" => {
                        coverage.err = true;
                        let id = self.bind_payload(inner, err, *name_span, pattern.span, "Err");
                        TPattern::Err(id)
                    }
                    other => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0311",
                            format!("`{other}` is not a variant of `Result`"),
                            *name_span,
                        ));
                        TPattern::Wildcard
                    }
                },
                Type::Error => TPattern::Wildcard,
                other => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0311",
                            format!("variant pattern `{name}` cannot match `{other}`"),
                            *name_span,
                        )
                        .with_note("only Option and Result can be pattern-matched in this build"),
                    );
                    TPattern::Wildcard
                }
            },
        }
    }

    fn bind_payload(
        &mut self,
        inner: &Option<Box<ast::Pattern>>,
        payload_ty: &Type,
        variant_span: Span,
        pattern_span: Span,
        variant: &str,
    ) -> Option<LocalId> {
        match inner.as_deref() {
            None => {
                self.diagnostics.push(Diagnostic::error(
                    "E0311",
                    format!(
                        "`{variant}` pattern requires a payload pattern, e.g. `{variant}(value)`"
                    ),
                    pattern_span,
                ));
                None
            }
            Some(ast::Pattern {
                kind: PatternKind::Wildcard,
                ..
            }) => None,
            Some(ast::Pattern {
                kind: PatternKind::Bind(name),
                span,
                ..
            }) => Some(self.add_local(name, payload_ty.clone(), false, false, *span)),
            Some(other) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0311",
                        "nested patterns are not implemented in this build",
                        other.span,
                    )
                    .with_note("bind the payload with a name and match it in an inner `match`"),
                );
                let _ = variant_span;
                None
            }
        }
    }

    fn check_exhaustive(&mut self, scrutinee: &Type, coverage: &Coverage, span: Span) {
        if scrutinee.is_error() || coverage.wildcard {
            return;
        }
        let missing: Vec<&str> = match scrutinee {
            Type::Bool => {
                let mut missing = Vec::new();
                if !coverage.bool_true {
                    missing.push("true");
                }
                if !coverage.bool_false {
                    missing.push("false");
                }
                missing
            }
            Type::Option(_) => {
                let mut missing = Vec::new();
                if !coverage.some {
                    missing.push("Some(_)");
                }
                if !coverage.none {
                    missing.push("None");
                }
                missing
            }
            Type::Result(_, _) => {
                let mut missing = Vec::new();
                if !coverage.ok {
                    missing.push("Ok(_)");
                }
                if !coverage.err {
                    missing.push("Err(_)");
                }
                missing
            }
            Type::Int | Type::Char | Type::String => {
                vec!["_ (a wildcard or binding)"]
            }
            Type::Struct(_) | Type::List(_) | Type::Unit | Type::Float | Type::Never => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0311",
                        format!("`{scrutinee}` cannot be matched exhaustively in this build"),
                        span,
                    )
                    .with_note("match supports Bool, Int, Char, String, Option and Result"),
                );
                return;
            }
            Type::Error => return,
        };
        if !missing.is_empty() {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0310",
                    format!("non-exhaustive `match` on `{scrutinee}`"),
                    span,
                )
                .with_note(format!("missing patterns: {}", missing.join(", ")))
                .with_note("add the missing pattern or a wildcard `_`"),
            );
        }
    }

    // ---------------------------------------------------------------------
    // LValues and mutability
    // ---------------------------------------------------------------------

    fn convert_lvalue(&mut self, expr: &ast::Expr) -> Option<(LValue, Type)> {
        match &expr.kind {
            ExprKind::Paren(inner) => self.convert_lvalue(inner),
            ExprKind::Ident(name) => match self.lookup(name) {
                Some(local) => Some((LValue::Local(local.id), local.ty)),
                None => {
                    self.diagnostics.push(Diagnostic::error(
                        "E0203",
                        format!("unknown binding `{name}`"),
                        expr.span,
                    ));
                    None
                }
            },
            ExprKind::Field {
                base,
                name,
                name_span,
            } => {
                let (base_lv, base_ty) = self.convert_lvalue(base)?;
                if let Type::Struct(struct_name) = base_ty {
                    if let Some(def) = self.structs.get(&struct_name) {
                        if let Some((index, field)) =
                            def.fields.iter().enumerate().find(|(_, f)| &f.name == name)
                        {
                            let ty = field.ty.clone();
                            return Some((LValue::Field(Box::new(base_lv), index), ty));
                        }
                    }
                    self.diagnostics.push(Diagnostic::error(
                        "E0205",
                        format!("struct `{struct_name}` has no field `{name}`"),
                        *name_span,
                    ));
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        "E0205",
                        format!("field assignment requires a struct, found `{base_ty}`"),
                        *name_span,
                    ));
                }
                None
            }
            ExprKind::Index { base, index } => {
                let (base_lv, base_ty) = self.convert_lvalue(base)?;
                let index = self.check_expr(index, Some(&Type::Int));
                if let Type::List(elem) = base_ty {
                    Some((
                        LValue::Index(Box::new(base_lv), Box::new(index)),
                        (*elem).clone(),
                    ))
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        "E0303",
                        format!("index assignment requires a List, found `{base_ty}`"),
                        expr.span,
                    ));
                    None
                }
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "E0303",
                    "invalid assignment target",
                    expr.span,
                ));
                None
            }
        }
    }

    fn root_is_mutable(&self, expr: &ast::Expr) -> bool {
        match &expr.kind {
            ExprKind::Paren(inner) => self.root_is_mutable(inner),
            ExprKind::Ident(name) => self
                .lookup(name)
                .map(|binding| binding.mutable)
                .unwrap_or(true),
            ExprKind::Field { base, .. } => self.root_is_mutable(base),
            ExprKind::Index { base, .. } => self.root_is_mutable(base),
            _ => true,
        }
    }

    // ---------------------------------------------------------------------
    // Scope and type helpers
    // ---------------------------------------------------------------------

    fn finish(&mut self, expr: TExpr, expected: Option<&Type>) -> TExpr {
        if let Some(expected) = expected {
            self.expect_type(&expr.ty, expected, expr.span);
        }
        expr
    }

    fn expect_type(&mut self, actual: &Type, expected: &Type, span: Span) {
        if actual.is_error() || expected.is_error() || *actual == Type::Never {
            return;
        }
        if actual != expected {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0308",
                    format!("type mismatch: expected `{expected}`, found `{actual}`"),
                    span,
                )
                .with_note("Nova has no implicit conversions; convert explicitly"),
            );
        }
    }

    fn push_scope(&mut self) {
        if let Some(state) = self.current.as_mut() {
            state.scopes.push(HashMap::new());
        }
    }

    fn pop_scope(&mut self) {
        if let Some(state) = self.current.as_mut() {
            state.scopes.pop();
        }
    }

    fn add_local(
        &mut self,
        name: &str,
        ty: Type,
        mutable: bool,
        is_param: bool,
        span: Span,
    ) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        let state = self.current.as_mut().expect("local outside function");
        let duplicate = state
            .scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false);
        if let Some(scope) = state.scopes.last_mut() {
            scope.insert(
                name.to_string(),
                LocalBinding {
                    id,
                    ty: ty.clone(),
                    mutable,
                },
            );
        }
        state.locals.push(LocalInfo {
            id,
            name: name.to_string(),
            ty,
            mutable,
            is_param,
        });
        if duplicate {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0200",
                    format!("duplicate binding `{name}` in the same scope"),
                    span,
                )
                .with_note("use a new name or an inner block to shadow"),
            );
        }
        id
    }

    fn lookup(&self, name: &str) -> Option<LocalBinding> {
        let state = self.current.as_ref()?;
        for scope in state.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(binding.clone());
            }
        }
        None
    }

    fn state(&self) -> &FnState {
        self.current.as_ref().expect("expression outside function")
    }

    fn state_mut(&mut self) -> &mut FnState {
        self.current.as_mut().expect("expression outside function")
    }
}

fn collect_value_structs(ty: &Type, out: &mut Vec<String>) {
    match ty {
        Type::Struct(name) => out.push(name.clone()),
        Type::Option(inner) => collect_value_structs(inner, out),
        Type::Result(ok, err) => {
            collect_value_structs(ok, out);
            collect_value_structs(err, out);
        }
        // Lists are heap indirections, so they break value cycles.
        _ => {}
    }
}

fn lvalue_target_span(expr: &ast::Expr) -> Span {
    match &expr.kind {
        ExprKind::Paren(inner) => lvalue_target_span(inner),
        ExprKind::Field { base, .. } => lvalue_target_span(base),
        ExprKind::Index { base, .. } => lvalue_target_span(base),
        _ => expr.span,
    }
}

fn lvalue_target_name(expr: &ast::Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Paren(inner) => lvalue_target_name(inner),
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::Field { base, .. } => lvalue_target_name(base),
        ExprKind::Index { base, .. } => lvalue_target_name(base),
        _ => None,
    }
}

fn target_span(target: &LValue, fallback: Span) -> Span {
    match target {
        LValue::Local(_) => fallback,
        LValue::Field(base, _) | LValue::Index(base, _) => target_span(base, fallback),
    }
}

fn block_terminates(block: &TBlock) -> bool {
    block.tail.is_some() || block.stmts.last().map(stmt_terminates).unwrap_or(false)
}

fn stmt_terminates(stmt: &TStmt) -> bool {
    match &stmt.kind {
        TStmtKind::Return(_) => true,
        TStmtKind::Expr(expr) => expr_terminates(expr),
        _ => false,
    }
}

fn expr_terminates(expr: &TExpr) -> bool {
    match &expr.kind {
        TExprKind::If {
            then_block,
            else_branch: Some(else_branch),
            ..
        } => block_terminates(then_block) && expr_terminates(else_branch),
        TExprKind::Match { arms, .. } => arms.iter().all(|arm| expr_terminates(&arm.body)),
        TExprKind::Block(block) => block_terminates(block),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_parser::parse;

    fn check_src(src: &str) -> CheckResult {
        let parsed = parse(src);
        assert!(
            parsed.diagnostics.is_empty(),
            "parse diagnostics: {:#?}",
            parsed.diagnostics
        );
        check(&parsed.program)
    }

    fn errors(result: &CheckResult) -> Vec<&Diagnostic> {
        result.diagnostics.iter().filter(|d| d.is_error()).collect()
    }

    #[test]
    fn accepts_hello_world() {
        let result = check_src(
            r#"import std.io { println }
fn main() {
    println("Hello, World!")
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
        assert_eq!(result.program.main.as_deref(), Some("main"));
        assert_eq!(result.program.functions.len(), 1);
    }

    #[test]
    fn infers_locals_and_checks_types() {
        let result = check_src(
            r#"fn add(a: Int, b: Int) -> Int {
    return a + b
}
fn main() {
    let x = add(1, 2)
    let y = 1.5
    println(x)
    println(y)
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
        let main = result
            .program
            .functions
            .iter()
            .find(|f| f.name == "main")
            .unwrap();
        let local_types: Vec<&Type> = main.locals.iter().map(|l| &l.ty).collect();
        assert!(local_types.contains(&&Type::Int));
        assert!(local_types.contains(&&Type::Float));
    }

    #[test]
    fn rejects_implicit_int_to_float() {
        let result = check_src(
            r#"fn main() {
    let x: Float = 1
}
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0308"));
    }

    #[test]
    fn rejects_immutable_assignment() {
        let result = check_src(
            r#"fn main() {
    let x = 1
    x = 2
}
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0301"));
    }

    #[test]
    fn allows_var_assignment_and_compound() {
        let result = check_src(
            r#"fn main() {
    var x = 1
    x += 2
    x = 4
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn checks_result_propagation() {
        let result = check_src(
            r#"fn lookup(id: Int) -> Result<String, String> {
    if id == 0 {
        return Err("missing")
    }
    return Ok("found")
}

fn find(id: Int) -> Result<String, String> {
    let value = lookup(id)?
    return Ok(value)
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn rejects_try_outside_result_function() {
        let result = check_src(
            r#"fn find() -> Int {
    let value = lookup()?
    return 1
}
"#,
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "E0203" || d.code == "E0306"));
    }

    #[test]
    fn checks_option_and_match_exhaustiveness() {
        let result = check_src(
            r#"fn first(items: List<Int>) -> Option<Int> {
    return items.get(0)
}

fn main() {
    match first([1, 2, 3]) {
        Some(value) => println(value)
        None => println("empty")
    }
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn rejects_non_exhaustive_match() {
        let result = check_src(
            r#"fn main() {
    match true {
        true => println("yes")
    }
}
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0310"));
    }

    #[test]
    fn checks_struct_literal_and_fields() {
        let result = check_src(
            r#"struct User {
    name: String
    age: Int
}

fn main() {
    let user = User { name: "Ada", age: 36 }
    println(user.name)
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn rejects_missing_struct_field() {
        let result = check_src(
            r#"struct User {
    name: String
    age: Int
}

fn main() {
    let user = User { name: "Ada" }
    println(user.name)
}
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0207"));
    }

    #[test]
    fn rejects_unprintable_println() {
        let result = check_src(
            r#"fn main() {
    println([1, 2])
}
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0320"));
    }

    #[test]
    fn infers_string_interpolation_parts() {
        let result = check_src(
            r#"fn greet(name: String) -> String {
    return "Hello, ${name}!"
}
fn main() {
    println(greet("World"))
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn rejects_mutable_list_push_on_let() {
        let result = check_src(
            r#"fn main() {
    let xs = [1, 2]
    xs.push(3)
}
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0301"));
    }

    #[test]
    fn allows_mutable_list_push() {
        let result = check_src(
            r#"fn main() {
    var xs = [1, 2]
    xs.push(3)
    println(xs.len())
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn rejects_recursive_struct() {
        let result = check_src(
            r#"struct Node {
    next: Node
}
fn main() { }
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0201"));
    }

    #[test]
    fn rejects_generic_function_explicitly() {
        let result = check_src(
            r#"fn first<T>(items: List<T>) -> Option<T> {
    return items.get(0)
}
fn main() { }
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0230"));
    }

    #[test]
    fn requires_return_on_all_paths() {
        let result = check_src(
            r#"fn value() -> Int {
    if true {
        return 1
    }
}
fn main() { }
"#,
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "E0314"));
    }

    #[test]
    fn allows_tail_expression_return() {
        let result = check_src(
            r#"fn value() -> Int {
    let x = 41
    x + 1
}
fn main() {
    println(value())
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn checks_test_functions() {
        let result = check_src(
            r#"@test
fn adds() {
    assert_eq(1 + 1, 2)
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
        assert_eq!(result.program.tests, vec!["adds"]);
    }

    #[test]
    fn rejects_unknown_names() {
        let result = check_src("fn main() { nope() }");
        assert!(result.diagnostics.iter().any(|d| d.code == "E0203"));
    }

    #[test]
    fn rejects_break_outside_loop() {
        let result = check_src("fn main() { break }");
        assert!(result.diagnostics.iter().any(|d| d.code == "E0302"));
    }
}
