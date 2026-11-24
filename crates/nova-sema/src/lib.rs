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
    receiver: Option<Type>,
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
    /// Covered variants of a user enum, by variant name.
    variants: HashSet<String>,
}

#[derive(Default)]
struct Checker {
    funcs: HashMap<String, FnSig>,
    func_order: Vec<String>,
    /// Receiver type name -> method name -> resolved `Type.method` symbol.
    methods: HashMap<String, HashMap<String, String>>,
    /// Trait name -> required method names.
    traits: HashMap<String, HashSet<String>>,
    /// Trait name -> method signatures in declaration order (self excluded).
    trait_sigs: HashMap<String, Vec<(String, Vec<Type>, Type)>>,
    /// (trait, concrete type display) -> (concrete type, method symbols).
    trait_impls: HashMap<(String, String), (Type, Vec<String>)>,
    generic_fns: HashMap<String, ast::FnDecl>,
    generic_specs: HashMap<String, String>,
    generic_queue: Vec<ast::FnDecl>,
    externs: Vec<ExternDef>,
    /// Aliases introduced by `import "file.nova" as name`.
    modules: HashSet<String>,
    macros: HashMap<String, ast::MacroDecl>,
    macro_depth: usize,
    struct_templates: HashMap<String, ast::StructDecl>,
    enum_templates: HashMap<String, ast::EnumDecl>,
    impl_templates: Vec<ast::ImplDecl>,
    /// Variant name -> (generic enum base, tag, declared payload types).
    generic_variants: HashMap<String, (String, u32, Vec<ast::TypeExpr>)>,
    /// Concrete type name -> (generic base, type arguments).
    specializations: HashMap<String, (String, Vec<Type>)>,
    specializing: HashSet<String>,
    structs: HashMap<String, StructDef>,
    struct_order: Vec<String>,
    struct_spans: HashMap<String, Span>,
    enums: HashMap<String, crate::typed::EnumDef>,
    enum_order: Vec<String>,
    enum_spans: HashMap<String, Span>,
    aliases: HashMap<String, BuiltinFn>,
    diagnostics: Vec<Diagnostic>,
    current: Option<FnState>,
    next_local: u32,
    closure_fns: Vec<TypedFn>,
    next_closure: u32,
}

/// Collect every name referenced from `block` that is not bound inside it.
/// Nested blocks and closures contribute references, but names they bind are
/// excluded so a closure parameter never shadows its way into a capture.
fn collect_block_names(block: &ast::Block, out: &mut HashSet<String>, bound: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match &stmt.kind {
            StmtKind::Let { name, init, .. } => {
                collect_expr_names(init, out, bound);
                bound.insert(name.clone());
            }
            StmtKind::Assign { target, value, .. } => {
                collect_expr_names(target, out, bound);
                collect_expr_names(value, out, bound);
            }
            StmtKind::Expr(expr) => collect_expr_names(expr, out, bound),
            StmtKind::While { cond, body } => {
                collect_expr_names(cond, out, bound);
                let mut inner = bound.clone();
                collect_block_names(body, out, &mut inner);
            }
            StmtKind::For {
                name, iter, body, ..
            } => {
                collect_expr_names(iter, out, bound);
                let mut inner = bound.clone();
                inner.insert(name.clone());
                collect_block_names(body, out, &mut inner);
            }
            StmtKind::Return(Some(expr)) => collect_expr_names(expr, out, bound),
            StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
        }
    }
}

/// Last segment of a dotted name: `Key` for `bubble.Event.Key`.
fn last_segment(path: &str) -> &str {
    match path.rsplit_once('.') {
        Some((_, tail)) => tail,
        None => path,
    }
}

fn collect_pattern_names(pattern: &ast::Pattern, out: &mut HashSet<String>) {
    match &pattern.kind {
        PatternKind::Bind(name) => {
            out.insert(name.clone());
        }
        PatternKind::Tuple(items) => {
            for item in items {
                collect_pattern_names(item, out);
            }
        }
        PatternKind::Variant { fields, .. } => {
            for field in fields {
                collect_pattern_names(field, out);
            }
        }
        PatternKind::Wildcard | PatternKind::Literal(_) => {}
    }
}

fn collect_expr_names(expr: &ast::Expr, out: &mut HashSet<String>, bound: &HashSet<String>) {
    match &expr.kind {
        ExprKind::Ident(name) => {
            if !bound.contains(name) {
                out.insert(name.clone());
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Paren(expr)
        | ExprKind::Try(expr)
        | ExprKind::Await(expr) => collect_expr_names(expr, out, bound),
        ExprKind::Binary { lhs, rhs, .. }
        | ExprKind::Range {
            start: lhs,
            end: rhs,
            ..
        } => {
            collect_expr_names(lhs, out, bound);
            collect_expr_names(rhs, out, bound);
        }
        ExprKind::Call { callee, args } => {
            collect_expr_names(callee, out, bound);
            for arg in args {
                collect_expr_names(&arg.value, out, bound);
            }
        }
        ExprKind::MethodCall { receiver, args, .. } => {
            collect_expr_names(receiver, out, bound);
            for arg in args {
                collect_expr_names(&arg.value, out, bound);
            }
        }
        ExprKind::Field { base, .. } | ExprKind::TupleField { base, .. } => {
            collect_expr_names(base, out, bound)
        }
        ExprKind::Index { base, index } => {
            collect_expr_names(base, out, bound);
            collect_expr_names(index, out, bound);
        }
        ExprKind::StructLit { fields, .. } => {
            for field in fields {
                collect_expr_names(&field.value, out, bound);
            }
        }
        ExprKind::If {
            cond,
            then_block,
            else_branch,
        } => {
            collect_expr_names(cond, out, bound);
            let mut inner = bound.clone();
            collect_block_names(then_block, out, &mut inner);
            match else_branch {
                Some(ElseBranch::Block(block)) => {
                    let mut inner = bound.clone();
                    collect_block_names(block, out, &mut inner);
                }
                Some(ElseBranch::If(expr)) => collect_expr_names(expr, out, bound),
                None => {}
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_names(scrutinee, out, bound);
            for arm in arms {
                let mut inner = bound.clone();
                collect_pattern_names(&arm.pattern, &mut inner);
                collect_expr_names(&arm.body, out, &inner);
            }
        }
        ExprKind::Block(block) => {
            let mut inner = bound.clone();
            collect_block_names(block, out, &mut inner);
        }
        ExprKind::List(items) | ExprKind::Tuple(items) => {
            for item in items {
                collect_expr_names(item, out, bound);
            }
        }
        ExprKind::Closure { params, body, .. } => {
            let mut inner = bound.clone();
            for param in params {
                inner.insert(param.name.clone());
            }
            collect_block_names(body, out, &mut inner);
        }
        ExprKind::Str(parts) => {
            for part in parts {
                if let ast::StrPart::Expr(expr) = part {
                    collect_expr_names(expr, out, bound);
                }
            }
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Unit
        | ExprKind::Error => {}
    }
}

/// Types that have a stable, scalar C ABI in this build. Heap values such as
/// `String`, `List` and `Map` are deliberately excluded because the runtime
/// layout is not a public C contract.
fn is_c_abi_scalar(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Unit | Type::Bool | Type::Int | Type::Float | Type::Char
    )
}

impl Checker {
    fn run(mut self, program: &ast::Program) -> CheckResult {
        self.collect_imports(program);
        self.collect_struct_names(program);
        self.collect_struct_fields(program);
        self.collect_enum_variants(program);
        self.detect_struct_cycles();
        self.collect_externs(program);
        self.collect_macros(program);
        self.collect_traits(program);
        self.collect_functions(program);
        self.collect_methods(program);

        let mut functions = Vec::new();
        for item in &program.items {
            match item {
                ast::Item::Fn(decl) => {
                    if !self.funcs.contains_key(&decl.name) {
                        continue; // generic or duplicate; already diagnosed
                    }
                    if let Some(func) = self.check_fn(decl, &decl.name) {
                        functions.push(func);
                    }
                }
                ast::Item::Impl(decl) => {
                    for method in &decl.methods {
                        let full = format!("{}.{}", decl.type_name, method.name);
                        if !self.funcs.contains_key(&full) {
                            continue;
                        }
                        if let Some(func) = self.check_fn(method, &full) {
                            functions.push(func);
                        }
                    }
                }
                _ => {}
            }
        }

        while let Some(decl) = self.generic_queue.pop() {
            if let Some(func) = self.check_fn(&decl, &decl.name) {
                functions.push(func);
            }
        }
        let closures = std::mem::take(&mut self.closure_fns);
        functions.extend(closures);

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
                enums: self
                    .enum_order
                    .iter()
                    .filter_map(|n| self.enums.get(n).cloned())
                    .collect(),
                externs: std::mem::take(&mut self.externs),
                trait_impls: self
                    .trait_impls
                    .iter()
                    .map(|((trait_name, _), (ty, methods))| TraitImplDef {
                        trait_name: trait_name.clone(),
                        ty: ty.clone(),
                        methods: methods.clone(),
                    })
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
            if import.path.is_some() {
                if let Some(alias) = &import.alias {
                    self.modules.insert(alias.clone());
                }
                continue;
            }
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
                    if self.structs.contains_key(&decl.name)
                        || self.struct_templates.contains_key(&decl.name)
                        || self.enums.contains_key(&decl.name)
                        || self.enum_templates.contains_key(&decl.name)
                    {
                        self.diagnostics.push(Diagnostic::error(
                            "E0200",
                            format!("duplicate struct `{}`", decl.name),
                            decl.name_span,
                        ));
                        continue;
                    }
                    self.struct_spans.insert(decl.name.clone(), decl.name_span);
                    if !decl.generics.is_empty() {
                        self.struct_templates
                            .insert(decl.name.clone(), decl.clone());
                        continue;
                    }
                    self.structs.insert(
                        decl.name.clone(),
                        StructDef {
                            name: decl.name.clone(),
                            fields: Vec::new(),
                        },
                    );
                    self.struct_order.push(decl.name.clone());
                }
                ast::Item::Enum(decl) => {
                    if self.structs.contains_key(&decl.name)
                        || self.struct_templates.contains_key(&decl.name)
                        || self.enums.contains_key(&decl.name)
                        || self.enum_templates.contains_key(&decl.name)
                    {
                        self.diagnostics.push(Diagnostic::error(
                            "E0201",
                            format!("duplicate type `{}`", decl.name),
                            decl.name_span,
                        ));
                        continue;
                    }
                    self.enum_spans.insert(decl.name.clone(), decl.name_span);
                    if !decl.generics.is_empty() {
                        self.enum_templates.insert(decl.name.clone(), decl.clone());
                        continue;
                    }
                    self.enums.insert(
                        decl.name.clone(),
                        crate::typed::EnumDef {
                            name: decl.name.clone(),
                            variants: Vec::new(),
                        },
                    );
                    self.enum_order.push(decl.name.clone());
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

    fn collect_enum_variants(&mut self, program: &ast::Program) {
        let mut owners: HashMap<String, String> = HashMap::new();
        for item in &program.items {
            let ast::Item::Enum(decl) = item else {
                continue;
            };
            let generic = !decl.generics.is_empty();
            let mut variants = Vec::new();
            let mut seen = HashSet::new();
            for (index, variant) in decl.variants.iter().enumerate() {
                if !seen.insert(variant.name.clone()) {
                    self.diagnostics.push(Diagnostic::error(
                        "E0201",
                        format!("duplicate variant `{}`", variant.name),
                        variant.name_span,
                    ));
                    continue;
                }
                if matches!(variant.name.as_str(), "Some" | "None" | "Ok" | "Err") {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0201",
                            format!("`{}` is a reserved built-in variant name", variant.name),
                            variant.name_span,
                        )
                        .with_note(
                            "rename the variant so it cannot be confused with Option/Result",
                        ),
                    );
                    continue;
                }
                if let Some(previous) = owners.insert(variant.name.clone(), decl.name.clone()) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0201",
                            format!(
                                "variant `{}` is already declared by enum `{previous}`",
                                variant.name
                            ),
                            variant.name_span,
                        )
                        .with_note("variant constructors are global in this build; give each one a unique name"),
                    );
                    continue;
                }
                if generic {
                    self.generic_variants.insert(
                        variant.name.clone(),
                        (decl.name.clone(), index as u32, variant.fields.clone()),
                    );
                    continue;
                }
                let fields = variant
                    .fields
                    .iter()
                    .map(|ty| self.resolve_type(ty))
                    .collect();
                variants.push(crate::typed::EnumVariantDef {
                    name: variant.name.clone(),
                    fields,
                });
            }
            if !generic {
                self.enums.insert(
                    decl.name.clone(),
                    crate::typed::EnumDef {
                        name: decl.name.clone(),
                        variants,
                    },
                );
            }
        }
    }

    /// Drops a leading `module.` from a path when `module` is an imported
    /// alias that is not shadowed by a value.
    fn strip_module_path<'a>(&self, path: &'a str) -> &'a str {
        if let Some((head, rest)) = path.split_once('.') {
            if self.modules.contains(head) && self.lookup(head).is_none() {
                return rest;
            }
        }
        path
    }

    /// Recognises `module.Type.Variant` in expression position, where the
    /// parser produced a chain of field accesses rooted at a module alias.
    fn qualified_variant(expr: &ast::Expr) -> Option<(String, String)> {
        let ExprKind::Field { base, name, .. } = &expr.kind else {
            return None;
        };
        let ExprKind::Field {
            base: root,
            name: _,
            ..
        } = &base.kind
        else {
            return None;
        };
        let ExprKind::Ident(module) = &root.kind else {
            return None;
        };
        Some((module.clone(), name.clone()))
    }

    fn lookup_variant(&self, name: &str) -> Option<(String, u32, Vec<Type>)> {
        for enum_name in &self.enum_order {
            let Some(def) = self.enums.get(enum_name) else {
                continue;
            };
            if let Some((index, variant)) = def
                .variants
                .iter()
                .enumerate()
                .find(|(_, variant)| variant.name == name)
            {
                return Some((def.name.clone(), index as u32, variant.fields.clone()));
            }
        }
        None
    }

    fn enum_variant(&self, enum_name: &str, variant_name: &str) -> Option<(u32, Vec<Type>)> {
        let def = self.enums.get(enum_name)?;
        let (index, variant) = def
            .variants
            .iter()
            .enumerate()
            .find(|(_, variant)| variant.name == variant_name)?;
        Some((index as u32, variant.fields.clone()))
    }

    fn specialize_struct(&mut self, base: &str, args: &[Type], span: Span) -> Type {
        let key = format!(
            "{}<{}>",
            base,
            args.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
        if self.structs.contains_key(&key) {
            return Type::Struct(key);
        }
        let Some(template) = self.struct_templates.get(base).cloned() else {
            return Type::Error;
        };
        if !self.specializing.insert(key.clone()) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0201",
                    format!("recursive generic type `{key}` has infinite size"),
                    span,
                )
                .with_note("store the cycle behind `List<T>`"),
            );
            return Type::Error;
        }
        let subst: HashMap<String, Type> = template
            .generics
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        let mut fields = Vec::new();
        for field in &template.fields {
            let substituted = substitute_type_expr(&field.ty, &subst);
            fields.push(FieldDef {
                name: field.name.clone(),
                ty: self.resolve_type(&substituted),
            });
        }
        self.specializing.remove(&key);
        self.structs.insert(
            key.clone(),
            StructDef {
                name: key.clone(),
                fields,
            },
        );
        self.struct_order.push(key.clone());
        self.struct_spans.insert(key.clone(), template.name_span);
        self.specializations
            .insert(key.clone(), (base.to_string(), args.to_vec()));
        Type::Struct(key)
    }

    fn specialize_enum(&mut self, base: &str, args: &[Type], span: Span) -> String {
        let key = format!(
            "{}<{}>",
            base,
            args.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
        if self.enums.contains_key(&key) {
            return key;
        }
        let Some(template) = self.enum_templates.get(base).cloned() else {
            return key;
        };
        if !self.specializing.insert(key.clone()) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0201",
                    format!("recursive generic type `{key}` has infinite size"),
                    span,
                )
                .with_note("store the cycle behind `List<T>`"),
            );
            return key;
        }
        let subst: HashMap<String, Type> = template
            .generics
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        let mut variants = Vec::new();
        for variant in &template.variants {
            let fields = variant
                .fields
                .iter()
                .map(|ty| {
                    let substituted = substitute_type_expr(ty, &subst);
                    self.resolve_type(&substituted)
                })
                .collect();
            variants.push(crate::typed::EnumVariantDef {
                name: variant.name.clone(),
                fields,
            });
        }
        self.specializing.remove(&key);
        self.enums.insert(
            key.clone(),
            crate::typed::EnumDef {
                name: key.clone(),
                variants,
            },
        );
        self.enum_order.push(key.clone());
        self.enum_spans.insert(key.clone(), template.name_span);
        self.specializations
            .insert(key.clone(), (base.to_string(), args.to_vec()));
        key
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
                if self.funcs.contains_key(&decl.name) || self.generic_fns.contains_key(&decl.name)
                {
                    self.diagnostics.push(Diagnostic::error(
                        "E0200",
                        format!("duplicate function `{}`", decl.name),
                        decl.name_span,
                    ));
                    continue;
                }
                self.generic_fns.insert(decl.name.clone(), decl.clone());
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
                    receiver: None,
                    is_test: decl.is_test,
                    span: decl.name_span,
                },
            );
            self.func_order.push(decl.name.clone());
        }
    }

    fn collect_externs(&mut self, program: &ast::Program) {
        for item in &program.items {
            let ast::Item::Extern(decl) = item else {
                continue;
            };
            if self.funcs.contains_key(&decl.name) {
                self.diagnostics.push(Diagnostic::error(
                    "E0200",
                    format!("duplicate function/extern `{}`", decl.name),
                    decl.name_span,
                ));
                continue;
            }
            let params: Vec<Type> = decl
                .params
                .iter()
                .map(|param| self.resolve_type(&param.ty))
                .collect();
            let ret: Type = decl
                .ret
                .as_ref()
                .map(|ty| self.resolve_type(ty))
                .unwrap_or(Type::Unit);
            let mut valid = decl.abi == "C";
            if decl.abi != "C" {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0241",
                        format!("unsupported extern ABI `\"{}\"`", decl.abi),
                        decl.name_span,
                    )
                    .with_note("only `extern \"C\"` is implemented in this build"),
                );
            }
            for (param, ty) in decl.params.iter().zip(params.iter()) {
                if !is_c_abi_scalar(ty) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0240",
                            format!(
                                "extern parameter `{}` has type `{ty}`, which has no C ABI representation",
                                param.name
                            ),
                            param.name_span,
                        )
                        .with_note(
                            "extern functions accept Bool, Int, Float, Char and Unit; values are passed by copy",
                        ),
                    );
                    valid = false;
                }
            }
            if !is_c_abi_scalar(&ret) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0240",
                        format!("extern return type `{ret}` has no C ABI representation"),
                        decl.ret
                            .as_ref()
                            .map(|ty| ty.span)
                            .unwrap_or(decl.name_span),
                    )
                    .with_note(
                        "extern functions may return Bool, Int, Float, Char or Unit; heap values cannot cross the boundary",
                    ),
                );
                valid = false;
            }
            self.funcs.insert(
                decl.name.clone(),
                FnSig {
                    params: params.clone(),
                    ret: ret.clone(),
                    receiver: None,
                    is_test: false,
                    span: decl.name_span,
                },
            );
            self.func_order.push(decl.name.clone());
            if valid {
                self.externs.push(ExternDef {
                    name: decl.name.clone(),
                    symbol: decl.name.clone(),
                    params,
                    ret,
                });
            }
        }
    }

    fn collect_macros(&mut self, program: &ast::Program) {
        for item in &program.items {
            let ast::Item::Macro(decl) = item else {
                continue;
            };
            if self.funcs.contains_key(&decl.name) || self.macros.contains_key(&decl.name) {
                self.diagnostics.push(Diagnostic::error(
                    "E0200",
                    format!("duplicate macro/function `{}`", decl.name),
                    decl.name_span,
                ));
                continue;
            }
            self.macros.insert(decl.name.clone(), decl.clone());
        }
    }

    fn collect_traits(&mut self, program: &ast::Program) {
        for item in &program.items {
            let ast::Item::Trait(decl) = item else {
                continue;
            };
            if self.traits.contains_key(&decl.name) {
                self.diagnostics.push(Diagnostic::error(
                    "E0200",
                    format!("duplicate trait `{}`", decl.name),
                    decl.name_span,
                ));
                continue;
            }
            let mut methods = HashSet::new();
            let mut sigs = Vec::new();
            for method in &decl.methods {
                if !methods.insert(method.name.clone()) {
                    self.diagnostics.push(Diagnostic::error(
                        "E0200",
                        format!("duplicate trait method `{}`", method.name),
                        method.name_span,
                    ));
                }
                if method.params.first().map(|p| p.name.as_str()) != Some("self") {
                    self.diagnostics.push(Diagnostic::error(
                        "E0201",
                        format!(
                            "trait method `{}` must take `self` as its first parameter",
                            method.name
                        ),
                        method.name_span,
                    ));
                }
                let params = method
                    .params
                    .iter()
                    .filter(|param| param.name != "self")
                    .map(|param| self.resolve_type(&param.ty))
                    .collect();
                let ret = method
                    .ret
                    .as_ref()
                    .map(|ty| self.resolve_type(ty))
                    .unwrap_or(Type::Unit);
                sigs.push((method.name.clone(), params, ret));
            }
            self.traits.insert(decl.name.clone(), methods);
            self.trait_sigs.insert(decl.name.clone(), sigs);
        }
    }

    fn collect_methods(&mut self, program: &ast::Program) {
        for item in &program.items {
            let ast::Item::Impl(decl) = item else {
                continue;
            };
            if let Some(trait_name) = &decl.trait_name {
                let Some(required) = self.traits.get(trait_name).cloned() else {
                    self.diagnostics.push(Diagnostic::error(
                        "E0201",
                        format!("unknown trait `{trait_name}`"),
                        decl.type_span,
                    ));
                    continue;
                };
                let provided: HashSet<String> = decl
                    .methods
                    .iter()
                    .map(|method| method.name.clone())
                    .collect();
                let mut missing: Vec<&String> = required
                    .iter()
                    .filter(|name| !provided.contains(*name))
                    .collect();
                missing.sort();
                for name in missing {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0201",
                            format!(
                                "`impl {trait_name} for {}` is missing required method `{name}`",
                                decl.type_name
                            ),
                            decl.type_span,
                        )
                        .with_note("all trait methods must be implemented"),
                    );
                }
            }
            if !decl.generics.is_empty() {
                if decl.trait_name.is_some() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0230",
                        "trait impls for generic types are not implemented in this build",
                        decl.type_span,
                    ));
                    continue;
                }
                if !self.struct_templates.contains_key(&decl.type_name)
                    && !self.enum_templates.contains_key(&decl.type_name)
                {
                    self.diagnostics.push(Diagnostic::error(
                        "E0201",
                        format!(
                            "cannot implement methods for unknown type `{}`",
                            decl.type_name
                        ),
                        decl.type_span,
                    ));
                    continue;
                }
                let valid_target = decl.type_args.len() == decl.generics.len()
                    && decl
                        .type_args
                        .iter()
                        .zip(decl.generics.iter())
                        .all(|(arg, generic)| {
                            matches!(
                                &arg.kind,
                                ast::TypeExprKind::Named { path, args }
                                    if args.is_empty() && path.len() == 1 && &path[0] == generic
                            )
                        });
                if !valid_target {
                    self.diagnostics.push(Diagnostic::error(
                        "E0230",
                        "generic impl targets must be written as `impl<T> Type<T>` with the parameters in order",
                        decl.type_span,
                    ));
                    continue;
                }
                for method in &decl.methods {
                    if method.params.first().map(|p| p.name.as_str()) != Some("self") {
                        self.diagnostics.push(Diagnostic::error(
                            "E0201",
                            format!(
                                "method `{}` must take `self` as its first parameter",
                                method.name
                            ),
                            method.name_span,
                        ));
                    }
                }
                self.impl_templates.push(decl.clone());
                continue;
            }
            let receiver = if self.structs.contains_key(&decl.type_name) {
                Type::Struct(decl.type_name.clone())
            } else if self.enums.contains_key(&decl.type_name) {
                Type::Enum(decl.type_name.clone())
            } else {
                self.diagnostics.push(Diagnostic::error(
                    "E0201",
                    format!(
                        "cannot implement methods for unknown type `{}`",
                        decl.type_name
                    ),
                    decl.type_span,
                ));
                continue;
            };
            for method in &decl.methods {
                if !method.generics.is_empty() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0230",
                        "generic methods are not implemented in this build",
                        method.name_span,
                    ));
                    continue;
                }
                if method.params.first().map(|p| p.name.as_str()) != Some("self") {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0201",
                            format!(
                                "method `{}` must take `self` as its first parameter",
                                method.name
                            ),
                            method.name_span,
                        )
                        .with_note("static methods in `impl` are not implemented yet"),
                    );
                    continue;
                }
                let full = format!("{}.{}", decl.type_name, method.name);
                if self.funcs.contains_key(&full) {
                    self.diagnostics.push(Diagnostic::error(
                        "E0200",
                        format!(
                            "duplicate method `{}` for `{}`",
                            method.name, decl.type_name
                        ),
                        method.name_span,
                    ));
                    continue;
                }
                let mut params = Vec::new();
                let mut seen = HashSet::from(["self".to_string()]);
                for param in method.params.iter().skip(1) {
                    if !seen.insert(param.name.clone()) {
                        self.diagnostics.push(Diagnostic::error(
                            "E0200",
                            format!("duplicate parameter `{}`", param.name),
                            param.name_span,
                        ));
                    }
                    params.push(self.resolve_type(&param.ty));
                }
                let ret = method
                    .ret
                    .as_ref()
                    .map(|ty| self.resolve_type(ty))
                    .unwrap_or(Type::Unit);
                self.funcs.insert(
                    full.clone(),
                    FnSig {
                        params,
                        ret,
                        receiver: Some(receiver.clone()),
                        is_test: false,
                        span: method.name_span,
                    },
                );
                self.func_order.push(full.clone());
                self.methods
                    .entry(decl.type_name.clone())
                    .or_default()
                    .insert(method.name.clone(), full);
            }
            if let Some(trait_name) = &decl.trait_name {
                if let Some(sigs) = self.trait_sigs.get(trait_name).cloned() {
                    let mut symbols = Vec::new();
                    let mut compatible = true;
                    for (name, params, ret) in &sigs {
                        let Some(full) = self
                            .methods
                            .get(&decl.type_name)
                            .and_then(|map| map.get(name))
                            .cloned()
                        else {
                            compatible = false;
                            break;
                        };
                        if let Some(sig) = self.funcs.get(&full) {
                            if sig.params != *params || sig.ret != *ret {
                                compatible = false;
                                self.diagnostics.push(Diagnostic::error(
                                    "E0201",
                                    format!(
                                        "method `{name}` in `impl {trait_name} for {}` does not match the trait signature",
                                        decl.type_name
                                    ),
                                    sig.span,
                                ));
                                break;
                            }
                        }
                        symbols.push(full);
                    }
                    if compatible {
                        self.trait_impls.insert(
                            (trait_name.clone(), receiver.to_string()),
                            (receiver.clone(), symbols),
                        );
                    }
                }
            }
        }
    }

    fn lookup_method(&mut self, ty: &Type, method: &str) -> Option<String> {
        let name = match ty {
            Type::Struct(name) | Type::Enum(name) => name.clone(),
            _ => return None,
        };
        if let Some(full) = self
            .methods
            .get(&name)
            .and_then(|map| map.get(method))
            .cloned()
        {
            return Some(full);
        }
        let (base, args) = self.specializations.get(&name)?.clone();
        let template = self
            .impl_templates
            .iter()
            .find(|decl| decl.type_name == base && decl.methods.iter().any(|m| m.name == method))?
            .clone();
        let method_decl = template.methods.iter().find(|m| m.name == method)?.clone();
        if method_decl.params.first().map(|p| p.name.as_str()) != Some("self") {
            return None;
        }
        let subst: HashMap<String, Type> = template
            .generics
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        let full = format!("{name}.{method}");
        if !self.funcs.contains_key(&full) {
            let mut concrete = method_decl.clone();
            concrete.name = full.clone();
            concrete.generics.clear();
            for param in concrete.params.iter_mut().skip(1) {
                param.ty = substitute_type_expr(&param.ty, &subst);
            }
            concrete.ret = concrete
                .ret
                .as_ref()
                .map(|ret| substitute_type_expr(ret, &subst));
            let params: Vec<Type> = concrete
                .params
                .iter()
                .skip(1)
                .map(|param| self.resolve_type(&param.ty))
                .collect();
            let ret = concrete
                .ret
                .as_ref()
                .map(|ty| self.resolve_type(ty))
                .unwrap_or(Type::Unit);
            self.funcs.insert(
                full.clone(),
                FnSig {
                    params,
                    ret,
                    receiver: Some(ty.clone()),
                    is_test: false,
                    span: method_decl.name_span,
                },
            );
            self.func_order.push(full.clone());
            self.generic_queue.push(concrete);
            self.methods
                .entry(name.clone())
                .or_default()
                .insert(method.to_string(), full.clone());
        }
        Some(full)
    }

    fn resolve_type(&mut self, ty: &ast::TypeExpr) -> Type {
        match &ty.kind {
            ast::TypeExprKind::Dyn(name) => {
                if self.traits.contains_key(name) {
                    Type::TraitObject(name.clone())
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        "E0202",
                        format!("unknown trait `{name}`"),
                        ty.span,
                    ));
                    Type::Error
                }
            }
            ast::TypeExprKind::Unit => Type::Unit,
            ast::TypeExprKind::Tuple(items) => {
                Type::Tuple(items.iter().map(|item| self.resolve_type(item)).collect())
            }
            ast::TypeExprKind::Array { elem, len } => {
                Type::Array(Box::new(self.resolve_type(elem)), *len)
            }
            ast::TypeExprKind::Fn { params, ret } => Type::Fn(
                params
                    .iter()
                    .map(|param| self.resolve_type(param))
                    .collect(),
                Box::new(self.resolve_type(ret)),
            ),
            ast::TypeExprKind::Named { path, args } => {
                let name = if path.len() == 1 {
                    &path[0]
                } else if path.len() == 2
                    && self.modules.contains(&path[0])
                    && self.lookup(&path[0]).is_none()
                {
                    &path[1]
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        "E0202",
                        format!(
                            "qualified type names are not supported: `{}`",
                            path.join(".")
                        ),
                        ty.span,
                    ));
                    return Type::Error;
                };
                if let Some(template) = self.struct_templates.get(name).cloned() {
                    if args.len() != template.generics.len() {
                        self.diagnostics.push(Diagnostic::error(
                            "E0202",
                            format!(
                                "`{name}` expects {} type argument(s), found {}",
                                template.generics.len(),
                                args.len()
                            ),
                            ty.span,
                        ));
                        return Type::Error;
                    }
                    let resolved: Vec<Type> =
                        args.iter().map(|arg| self.resolve_type(arg)).collect();
                    return self.specialize_struct(name, &resolved, ty.span);
                }
                if let Some(template) = self.enum_templates.get(name).cloned() {
                    if args.len() != template.generics.len() {
                        self.diagnostics.push(Diagnostic::error(
                            "E0202",
                            format!(
                                "`{name}` expects {} type argument(s), found {}",
                                template.generics.len(),
                                args.len()
                            ),
                            ty.span,
                        ));
                        return Type::Error;
                    }
                    let resolved: Vec<Type> =
                        args.iter().map(|arg| self.resolve_type(arg)).collect();
                    return Type::Enum(self.specialize_enum(name, &resolved, ty.span));
                }
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
                    "Slice" => {
                        if args.len() != 1 {
                            self.diagnostics.push(Diagnostic::error(
                                "E0202",
                                format!("`Slice` expects 1 type argument, found {}", args.len()),
                                ty.span,
                            ));
                            return Type::Error;
                        }
                        Type::Slice(Box::new(self.resolve_type(&args[0])))
                    }
                    "Map" => {
                        if args.len() != 2 {
                            self.diagnostics.push(Diagnostic::error(
                                "E0202",
                                format!("`Map` expects 2 type arguments, found {}", args.len()),
                                ty.span,
                            ));
                            return Type::Error;
                        }
                        Type::Map(
                            Box::new(self.resolve_type(&args[0])),
                            Box::new(self.resolve_type(&args[1])),
                        )
                    }
                    "Set" => {
                        if args.len() != 1 {
                            self.diagnostics.push(Diagnostic::error(
                                "E0202",
                                format!("`Set` expects 1 type argument, found {}", args.len()),
                                ty.span,
                            ));
                            return Type::Error;
                        }
                        Type::Set(Box::new(self.resolve_type(&args[0])))
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
                        if self.structs.contains_key(other) || self.enums.contains_key(other) {
                            if !args.is_empty() {
                                self.diagnostics.push(Diagnostic::error(
                                    "E0202",
                                    format!("type `{other}` does not take type arguments"),
                                    ty.span,
                                ));
                            }
                            if self.enums.contains_key(other) {
                                Type::Enum(other.to_string())
                            } else {
                                Type::Struct(other.to_string())
                            }
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

    fn check_closure(
        &mut self,
        params: &[ast::Param],
        ret: Option<&ast::TypeExpr>,
        body: &ast::Block,
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let mut referenced: HashSet<String> = HashSet::new();
        let mut bound: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
        collect_block_names(body, &mut referenced, &mut bound);
        let mut captures: Vec<(LocalId, String, Type)> = referenced
            .into_iter()
            .filter_map(|name| {
                self.lookup(&name)
                    .map(|binding| (binding.id, name, binding.ty))
            })
            .collect();
        captures.sort_by_key(|(id, _, _)| *id);
        captures.dedup_by_key(|(id, _, _)| *id);
        let outer = self.current.take();
        let outer_next = self.next_local;
        let declared_ret = ret.map(|ty| self.resolve_type(ty));
        let expected_ret = match expected {
            Some(Type::Fn(_, ret)) => Some((**ret).clone()),
            _ => None,
        };
        let target_ret = declared_ret.clone().or(expected_ret);
        self.current = Some(FnState::new(target_ret.clone().unwrap_or(Type::Error)));
        self.next_local = outer_next;
        self.push_scope();
        for (id, name, ty) in &captures {
            if let Some(state) = self.current.as_mut() {
                state.locals.push(LocalInfo {
                    id: *id,
                    name: name.clone(),
                    ty: ty.clone(),
                    mutable: false,
                    is_param: true,
                });
                if let Some(scope) = state.scopes.last_mut() {
                    scope.insert(
                        name.clone(),
                        LocalBinding {
                            id: *id,
                            ty: ty.clone(),
                            mutable: false,
                        },
                    );
                }
            }
        }
        let mut param_tys = Vec::new();
        let mut param_ids = Vec::new();
        for param in params {
            let ty = self.resolve_type(&param.ty);
            let id = self.add_local(&param.name, ty.clone(), false, true, param.name_span);
            param_tys.push(ty);
            param_ids.push(id);
        }
        let checked_body = self.check_block(body, target_ret.as_ref(), true);
        let inferred_ret = target_ret
            .or_else(|| checked_body.tail.as_ref().map(|expr| expr.ty.clone()))
            .or_else(|| {
                checked_body.stmts.iter().find_map(|stmt| match &stmt.kind {
                    TStmtKind::Return(Some(expr)) => Some(expr.ty.clone()),
                    _ => None,
                })
            })
            .unwrap_or(Type::Unit);
        self.pop_scope();
        let state = self.current.take().expect("closure state");
        let name = format!("__closure_{}", self.next_closure);
        self.next_closure += 1;
        self.closure_fns.push(TypedFn {
            name: name.clone(),
            params: param_ids,
            captures: captures.iter().map(|(id, _, _)| *id).collect(),
            ret: inferred_ret.clone(),
            body: checked_body,
            locals: state.locals,
            span,
            is_test: false,
        });
        self.current = outer;
        self.next_local = outer_next;
        let capture_exprs = captures
            .iter()
            .map(|(id, _, ty)| TExpr::new(TExprKind::Local(*id), ty.clone(), span))
            .collect();
        let ty = Type::Fn(param_tys, Box::new(inferred_ret));
        self.finish(
            TExpr::new(
                TExprKind::Closure {
                    name,
                    captures: capture_exprs,
                },
                ty,
                span,
            ),
            expected,
        )
    }

    fn check_fn(&mut self, decl: &ast::FnDecl, resolved_name: &str) -> Option<TypedFn> {
        let sig = self.funcs.get(resolved_name)?.clone();
        self.current = Some(FnState::new(sig.ret.clone()));
        self.next_local = 0;
        self.push_scope();

        let mut param_ids = Vec::new();
        let skip = if sig.receiver.is_some() {
            if let Some(recv) = sig.receiver.clone() {
                let id = self.add_local("self", recv, false, true, decl.name_span);
                param_ids.push(id);
            }
            1
        } else {
            0
        };
        for (param, ty) in decl.params.iter().skip(skip).zip(sig.params.iter()) {
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
                            resolved_name, sig.ret
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
            name: resolved_name.to_string(),
            params: param_ids,
            captures: Vec::new(),
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
        let expanded = self.expand_stmt_macros(&block.stmts, 0);
        let count = expanded.len();
        for (index, stmt) in expanded.iter().enumerate() {
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

    /// Replace statement-position macro calls with the statements of the
    /// expanded macro body. Expansion is AST substitution and inherits the
    /// caller's scope, just like expression macros.
    fn expand_stmt_macros(&mut self, stmts: &[ast::Stmt], depth: usize) -> Vec<ast::Stmt> {
        let mut expanded = Vec::new();
        for stmt in stmts {
            if let StmtKind::Expr(ast::Expr {
                kind: ExprKind::Call { callee, args },
                span,
            }) = &stmt.kind
            {
                if let ExprKind::Ident(name) = &callee.kind {
                    if let Some(mac) = self.macros.get(name).cloned() {
                        if args.len() != mac.params.len() {
                            self.diagnostics.push(Diagnostic::error(
                                "E0208",
                                format!(
                                    "macro `{name}` expects {} argument{}, found {}",
                                    mac.params.len(),
                                    if mac.params.len() == 1 { "" } else { "s" },
                                    args.len()
                                ),
                                *span,
                            ));
                            continue;
                        }
                        if depth >= 32 {
                            self.diagnostics.push(Diagnostic::error(
                                "E0208",
                                "macro expansion exceeded the depth limit",
                                *span,
                            ));
                            continue;
                        }
                        let substitutions: HashMap<String, ast::Expr> = mac
                            .params
                            .iter()
                            .zip(args.iter())
                            .map(|((param, _), arg)| (param.clone(), arg.value.clone()))
                            .collect();
                        let body = substitute_macro_block(&mac.body, &substitutions);
                        expanded.extend(self.expand_stmt_macros(&body.stmts, depth + 1));
                        continue;
                    }
                }
            }
            expanded.push(stmt.clone());
        }
        expanded
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
            ExprKind::Await(inner) => self.check_expr(inner, expected),
            ExprKind::Closure { params, ret, body } => {
                self.check_closure(params, ret.as_ref(), body, span, expected)
            }
            ExprKind::Tuple(items) => {
                let expected_items = match expected {
                    Some(Type::Tuple(expected_items)) => {
                        if expected_items.len() != items.len() {
                            self.diagnostics.push(Diagnostic::error(
                                "E0308",
                                format!(
                                    "tuple arity mismatch: expected {} element(s), found {}",
                                    expected_items.len(),
                                    items.len()
                                ),
                                span,
                            ));
                        }
                        Some(expected_items)
                    }
                    Some(other) if !other.is_error() => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0308",
                            format!("expected `{other}`, found a tuple"),
                            span,
                        ));
                        None
                    }
                    _ => None,
                };
                let mut checked = Vec::new();
                for (index, item) in items.iter().enumerate() {
                    let item_expected = expected_items.and_then(|items| items.get(index));
                    checked.push(self.check_expr(item, item_expected));
                }
                let ty = Type::Tuple(checked.iter().map(|item| item.ty.clone()).collect());
                self.finish(TExpr::new(TExprKind::Tuple(checked), ty, span), expected)
            }
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
            } => {
                if let ExprKind::Field {
                    base: root,
                    name: _,
                    ..
                } = &receiver.kind
                {
                    if let ExprKind::Ident(module) = &root.kind {
                        if self.modules.contains(module) && self.lookup(module).is_none() {
                            let callee =
                                ast::Expr::new(ExprKind::Ident(method.clone()), *method_span);
                            return self.check_call(&callee, args, span, expected);
                        }
                    }
                }
                if let ExprKind::Ident(alias) = &receiver.kind {
                    if self.modules.contains(alias) && self.lookup(alias).is_none() {
                        let callee = ast::Expr::new(ExprKind::Ident(method.clone()), *method_span);
                        return self.check_call(&callee, args, span, expected);
                    }
                }
                self.check_method(receiver, method, *method_span, args, span, expected)
            }
            ExprKind::Field {
                base,
                name,
                name_span,
            } => {
                if let ExprKind::Field {
                    base: root,
                    name: _,
                    ..
                } = &base.kind
                {
                    if let ExprKind::Ident(module) = &root.kind {
                        if self.modules.contains(module) && self.lookup(module).is_none() {
                            return self.check_ident(name, *name_span, expected);
                        }
                    }
                }
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
            ExprKind::TupleField {
                base,
                index,
                index_span,
            } => {
                let checked_base = self.check_expr(base, None);
                let ty = match &checked_base.ty {
                    Type::Tuple(items) => match items.get(*index) {
                        Some(ty) => ty.clone(),
                        None => {
                            self.diagnostics.push(Diagnostic::error(
                                "E0205",
                                format!(
                                    "tuple has {} element(s), so `.{index}` is out of range",
                                    items.len()
                                ),
                                *index_span,
                            ));
                            Type::Error
                        }
                    },
                    Type::Error => Type::Error,
                    other => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0205",
                            format!("tuple field access requires a tuple, found `{other}`"),
                            *index_span,
                        ));
                        Type::Error
                    }
                };
                self.finish(
                    TExpr::new(
                        TExprKind::TupleField {
                            base: Box::new(checked_base),
                            index: *index,
                        },
                        ty,
                        span,
                    ),
                    expected,
                )
            }
            ExprKind::Index { base, index } => {
                let checked_base = self.check_expr(base, None);
                let index = self.check_expr(index, Some(&Type::Int));
                let ty = match &checked_base.ty {
                    Type::List(elem) | Type::Array(elem, _) | Type::Slice(elem) => (**elem).clone(),
                    Type::Error => Type::Error,
                    other => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0205",
                                format!("indexing is not defined for `{other}`"),
                                span,
                            )
                            .with_note("indexing supports `List<T>`, `[T; N]` and `Slice<T>`"),
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
                if let Some(Type::Array(inner, len)) = expected {
                    if items.len() != *len {
                        self.diagnostics.push(Diagnostic::error(
                            "E0308",
                            format!(
                                "array length mismatch: expected {len} element(s), found {}",
                                items.len()
                            ),
                            span,
                        ));
                    }
                    let checked: Vec<TExpr> = items
                        .iter()
                        .map(|item| self.check_expr(item, Some(inner)))
                        .collect();
                    return TExpr::new(
                        TExprKind::Array(checked),
                        Type::Array(inner.clone(), *len),
                        span,
                    );
                }
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
        if let Some(Type::Enum(spec)) = expected {
            if let Some((tag, fields)) = self.enum_variant(spec, name) {
                if !fields.is_empty() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0205",
                        format!(
                            "`{name}` carries {} payload field{} and requires an argument list",
                            fields.len(),
                            if fields.len() == 1 { "" } else { "s" }
                        ),
                        span,
                    ));
                    return TExpr::new(TExprKind::Unit, Type::Error, span);
                }
                return self.finish(
                    TExpr::new(
                        TExprKind::EnumVariant {
                            enum_name: spec.clone(),
                            tag,
                            fields: Vec::new(),
                        },
                        Type::Enum(spec.clone()),
                        span,
                    ),
                    expected,
                );
            }
        }
        if self.generic_variants.contains_key(name) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0210",
                    format!("cannot infer type arguments for variant `{name}`"),
                    span,
                )
                .with_note(format!(
                    "annotate the binding, e.g. `let value: SomeEnum<Int> = {name}`"
                )),
            );
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        if let Some((enum_name, tag, fields)) = self.lookup_variant(name) {
            if !fields.is_empty() {
                self.diagnostics.push(Diagnostic::error(
                    "E0205",
                    format!(
                        "`{name}` carries {} payload field{} and requires an argument list",
                        fields.len(),
                        if fields.len() == 1 { "" } else { "s" }
                    ),
                    span,
                ));
                return TExpr::new(TExprKind::Unit, Type::Error, span);
            }
            let ty = Type::Enum(enum_name.clone());
            return self.finish(
                TExpr::new(
                    TExprKind::EnumVariant {
                        enum_name,
                        tag,
                        fields: Vec::new(),
                    },
                    ty,
                    span,
                ),
                expected,
            );
        }
        if self.externs.iter().any(|external| external.name == name) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0205",
                    format!("extern function `{name}` cannot be used as a function value"),
                    span,
                )
                .with_note("call it directly, or wrap it in a Nova function"),
            );
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        if let Some(sig) = self.funcs.get(name).cloned() {
            let ty = Type::Fn(sig.params.clone(), Box::new(sig.ret.clone()));
            return self.finish(
                TExpr::new(
                    TExprKind::FunctionRef {
                        name: name.to_string(),
                    },
                    ty,
                    span,
                ),
                expected,
            );
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
        if let Some((module, variant)) = Self::qualified_variant(callee) {
            if self.modules.contains(&module) && self.lookup(&module).is_none() {
                let rewritten = ast::Expr::new(ExprKind::Ident(variant), callee.span);
                return self.check_call(&rewritten, args, span, expected);
            }
        }
        let ExprKind::Ident(name) = &callee.kind else {
            return self.check_fn_value_call(callee, args, span, expected);
        };
        if self.lookup(name).is_some() {
            return self.check_fn_value_call(callee, args, span, expected);
        }

        if name == "Some" || name == "Ok" || name == "Err" || name == "None" {
            return self.check_constructor(name, args, span, expected);
        }

        if let Some((base, _tag, _fields)) = self.generic_variants.get(name).cloned() {
            if let Some(Type::Enum(spec)) = expected {
                if let Some((spec_base, spec_args)) = self.specializations.get(spec).cloned() {
                    if spec_base == base {
                        let key = self.specialize_enum(&base, &spec_args, span);
                        if let Some((tag, fields)) = self.enum_variant(&key, name) {
                            return self
                                .check_enum_variant(name, &key, tag, fields, args, span, expected);
                        }
                    }
                }
            }
            self.diagnostics.push(
                Diagnostic::error(
                    "E0210",
                    format!("cannot infer type arguments for variant `{name}`"),
                    span,
                )
                .with_note(format!(
                    "annotate the binding, e.g. `let value: SomeEnum<Int> = {name}(...)`"
                )),
            );
            for arg in args {
                self.check_expr(&arg.value, None);
            }
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        if let Some((enum_name, tag, fields)) = self.lookup_variant(name) {
            return self.check_enum_variant(name, &enum_name, tag, fields, args, span, expected);
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

        if let Some(mac) = self.macros.get(name).cloned() {
            if args.len() != mac.params.len() {
                self.diagnostics.push(Diagnostic::error(
                    "E0208",
                    format!(
                        "macro `{name}` expects {} argument{}, found {}",
                        mac.params.len(),
                        if mac.params.len() == 1 { "" } else { "s" },
                        args.len()
                    ),
                    span,
                ));
                return TExpr::new(TExprKind::Unit, Type::Error, span);
            }
            if self.macro_depth > 32 {
                self.diagnostics.push(Diagnostic::error(
                    "E0208",
                    "macro expansion exceeded the depth limit",
                    span,
                ));
                return TExpr::new(TExprKind::Unit, Type::Error, span);
            }
            let substitutions: HashMap<String, ast::Expr> = mac
                .params
                .iter()
                .zip(args.iter())
                .map(|((param, _), arg)| (param.clone(), arg.value.clone()))
                .collect();
            let expanded = substitute_macro_block(&mac.body, &substitutions);
            let expanded = ast::Expr::new(ast::ExprKind::Block(expanded), span);
            self.macro_depth += 1;
            let result = self.check_expr(&expanded, expected);
            self.macro_depth -= 1;
            return result;
        }

        if !self.funcs.contains_key(name) && (name == "map" || name == "set") {
            if !args.is_empty() {
                self.diagnostics.push(Diagnostic::error(
                    "E0208",
                    format!("`{name}()` takes no arguments"),
                    span,
                ));
            }
            match (name.as_str(), expected) {
                ("map", Some(Type::Map(key, value))) => {
                    return TExpr::new(
                        TExprKind::MapEmpty,
                        Type::Map(key.clone(), value.clone()),
                        span,
                    )
                }
                ("set", Some(Type::Set(inner))) => {
                    return TExpr::new(TExprKind::SetEmpty, Type::Set(inner.clone()), span)
                }
                _ => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0210",
                            format!("cannot infer the type of `{name}()`"),
                            span,
                        )
                        .with_note(format!(
                            "annotate the binding, e.g. `let value: {} = {name}()`",
                            if name == "map" {
                                "Map<String, Int>"
                            } else {
                                "Set<Int>"
                            }
                        )),
                    );
                    return TExpr::new(TExprKind::Unit, Type::Error, span);
                }
            }
        }

        let builtin = BuiltinFn::from_name(name).or_else(|| self.aliases.get(name).copied());
        if let Some(builtin) = builtin {
            return self.check_builtin(builtin, args, span, expected);
        }

        if let Some(generic) = self.generic_fns.get(name).cloned() {
            return self.check_generic_call(&generic, args, span, expected);
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

    fn check_generic_call(
        &mut self,
        decl: &ast::FnDecl,
        args: &[ast::Arg],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let mut checked_args = Vec::new();
        for arg in args {
            checked_args.push(self.check_expr(&arg.value, None));
        }
        let generic_names: HashSet<String> = decl.generics.iter().cloned().collect();
        let mut substitutions: HashMap<String, Type> = HashMap::new();
        for (param, arg) in decl.params.iter().zip(checked_args.iter()) {
            self.infer_generic_type(
                &param.ty,
                &arg.ty,
                &generic_names,
                &mut substitutions,
                param.span,
            );
        }
        let mut complete = true;
        for name in &decl.generics {
            if !substitutions.contains_key(name) {
                complete = false;
                self.diagnostics.push(Diagnostic::error(
                    "E0210",
                    format!(
                        "cannot infer generic parameter `{name}` for `{}`",
                        decl.name
                    ),
                    span,
                ));
                substitutions.insert(name.clone(), Type::Error);
            }
        }
        let suffix = decl
            .generics
            .iter()
            .map(|name| {
                substitutions
                    .get(name)
                    .map(|ty| type_name_fragment(ty))
                    .unwrap_or_else(|| "error".into())
            })
            .collect::<Vec<_>>()
            .join("_");
        let specialized = format!("{}__{}", decl.name, suffix);
        if !self.funcs.contains_key(&specialized) {
            let mut concrete = decl.clone();
            concrete.name = specialized.clone();
            concrete.generics.clear();
            for param in &mut concrete.params {
                param.ty = substitute_type_expr(&param.ty, &substitutions);
            }
            concrete.ret = concrete
                .ret
                .as_ref()
                .map(|ret| substitute_type_expr(ret, &substitutions));
            let params = concrete
                .params
                .iter()
                .map(|param| self.resolve_type(&param.ty))
                .collect();
            let ret = concrete
                .ret
                .as_ref()
                .map(|ty| self.resolve_type(ty))
                .unwrap_or(Type::Unit);
            self.funcs.insert(
                specialized.clone(),
                FnSig {
                    params,
                    ret,
                    receiver: None,
                    is_test: false,
                    span: decl.name_span,
                },
            );
            self.func_order.push(specialized.clone());
            self.generic_queue.push(concrete);
        }
        let Some(sig) = self.funcs.get(&specialized).cloned() else {
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        };
        if checked_args.len() != sig.params.len() {
            self.diagnostics.push(Diagnostic::error(
                "E0208",
                format!(
                    "`{}` expects {} argument{}, found {}",
                    decl.name,
                    sig.params.len(),
                    if sig.params.len() == 1 { "" } else { "s" },
                    checked_args.len()
                ),
                span,
            ));
        }
        for (index, arg) in checked_args.iter().enumerate() {
            if let Some(expected_arg) = sig.params.get(index) {
                self.expect_type(&arg.ty, expected_arg, arg.span);
            }
        }
        if !complete {
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        self.finish(
            TExpr::new(
                TExprKind::Call {
                    name: specialized,
                    args: checked_args,
                },
                sig.ret.clone(),
                span,
            ),
            expected,
        )
    }

    fn infer_generic_type(
        &mut self,
        ty: &ast::TypeExpr,
        actual: &Type,
        generics: &HashSet<String>,
        substitutions: &mut HashMap<String, Type>,
        span: Span,
    ) {
        if actual.is_error() {
            return;
        }
        match &ty.kind {
            ast::TypeExprKind::Named { path, args } if path.len() == 1 => {
                if generics.contains(&path[0]) {
                    if let Some(previous) = substitutions.get(&path[0]) {
                        if previous != actual {
                            self.diagnostics.push(Diagnostic::error(
                                "E0308",
                                format!(
                                    "generic parameter `{}` was inferred as both `{previous}` and `{actual}`",
                                    path[0]
                                ),
                                span,
                            ));
                        }
                    } else {
                        substitutions.insert(path[0].clone(), actual.clone());
                    }
                    return;
                }
                match (path[0].as_str(), args.as_slice(), actual) {
                    ("List", [inner], Type::List(actual_inner)) => {
                        self.infer_generic_type(inner, actual_inner, generics, substitutions, span)
                    }
                    ("Option", [inner], Type::Option(actual_inner)) => {
                        self.infer_generic_type(inner, actual_inner, generics, substitutions, span)
                    }
                    ("Slice", [inner], Type::Slice(actual_inner)) => {
                        self.infer_generic_type(inner, actual_inner, generics, substitutions, span)
                    }
                    ("Set", [inner], Type::Set(actual_inner)) => {
                        self.infer_generic_type(inner, actual_inner, generics, substitutions, span)
                    }
                    ("Result", [ok, err], Type::Result(actual_ok, actual_err)) => {
                        self.infer_generic_type(ok, actual_ok, generics, substitutions, span);
                        self.infer_generic_type(err, actual_err, generics, substitutions, span);
                    }
                    ("Map", [key, value], Type::Map(actual_key, actual_value)) => {
                        self.infer_generic_type(key, actual_key, generics, substitutions, span);
                        self.infer_generic_type(value, actual_value, generics, substitutions, span);
                    }
                    _ => {}
                }
            }
            ast::TypeExprKind::Tuple(items) => {
                if let Type::Tuple(actual_items) = actual {
                    for (item, actual_item) in items.iter().zip(actual_items.iter()) {
                        self.infer_generic_type(item, actual_item, generics, substitutions, span);
                    }
                }
            }
            ast::TypeExprKind::Array { elem, .. } => {
                if let Type::Array(actual_elem, _) = actual {
                    self.infer_generic_type(elem, actual_elem, generics, substitutions, span);
                }
            }
            ast::TypeExprKind::Fn { params, ret } => {
                if let Type::Fn(actual_params, actual_ret) = actual {
                    for (param, actual_param) in params.iter().zip(actual_params.iter()) {
                        self.infer_generic_type(param, actual_param, generics, substitutions, span);
                    }
                    self.infer_generic_type(ret, actual_ret, generics, substitutions, span);
                }
            }
            _ => {}
        }
    }

    fn check_fn_value_call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Arg],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let checked_callee = self.check_expr(callee, None);
        let Type::Fn(params, ret) = checked_callee.ty.clone() else {
            if !checked_callee.ty.is_error() {
                self.diagnostics.push(Diagnostic::error(
                    "E0205",
                    format!(
                        "only functions can be called, found `{}`",
                        checked_callee.ty
                    ),
                    callee.span,
                ));
            }
            for arg in args {
                self.check_expr(&arg.value, None);
            }
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        };
        if args.len() != params.len() {
            self.diagnostics.push(Diagnostic::error(
                "E0208",
                format!(
                    "function value expects {} argument{}, found {}",
                    params.len(),
                    if params.len() == 1 { "" } else { "s" },
                    args.len()
                ),
                span,
            ));
        }
        let mut checked = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            let expected_arg = params.get(index);
            let value = self.check_expr(&arg.value, expected_arg);
            if let Some(expected_arg) = expected_arg {
                if value.ty != *expected_arg && !value.ty.is_error() && !expected_arg.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0308",
                        format!(
                            "argument {} has type `{}`, expected `{expected_arg}`",
                            index + 1,
                            value.ty
                        ),
                        value.span,
                    ));
                }
            }
            checked.push(value);
        }
        self.finish(
            TExpr::new(
                TExprKind::FnCall {
                    callee: Box::new(checked_callee),
                    args: checked,
                },
                (*ret).clone(),
                span,
            ),
            expected,
        )
    }

    fn check_enum_variant(
        &mut self,
        variant_name: &str,
        enum_name: &str,
        tag: u32,
        fields: Vec<Type>,
        args: &[ast::Arg],
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        for arg in args {
            if let Some(name_span) = arg.name_span {
                self.diagnostics.push(Diagnostic::error(
                    "E0208",
                    "named arguments are not implemented in this build",
                    name_span,
                ));
            }
        }
        let ty = Type::Enum(enum_name.to_string());
        if args.len() != fields.len() {
            self.diagnostics.push(Diagnostic::error(
                "E0208",
                format!(
                    "`{variant_name}` expects {} payload argument{}, found {}",
                    fields.len(),
                    if fields.len() == 1 { "" } else { "s" },
                    args.len()
                ),
                span,
            ));
            for arg in args {
                self.check_expr(&arg.value, None);
            }
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
        let mut checked = Vec::new();
        for (arg, field_ty) in args.iter().zip(fields.iter()) {
            let value = self.check_expr(&arg.value, Some(field_ty));
            if value.ty != *field_ty && !value.ty.is_error() && !field_ty.is_error() {
                self.diagnostics.push(Diagnostic::error(
                    "E0308",
                    format!(
                        "argument {} to `{variant_name}` has type `{}`, expected `{field_ty}`",
                        checked.len() + 1,
                        value.ty
                    ),
                    value.span,
                ));
            }
            checked.push(value);
        }
        self.finish(
            TExpr::new(
                TExprKind::EnumVariant {
                    enum_name: enum_name.to_string(),
                    tag,
                    fields: checked,
                },
                ty,
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
            BuiltinFn::TermPaste => 0,
            BuiltinFn::JsonString => 2,
            BuiltinFn::HttpListen => 1,
            BuiltinFn::HttpHeader => 1,
            BuiltinFn::HttpForm => 1,
            BuiltinFn::HttpRespond => 3,
            BuiltinFn::HttpAccept
            | BuiltinFn::HttpMethod
            | BuiltinFn::HttpPath
            | BuiltinFn::HttpQuery
            | BuiltinFn::HttpBody
            | BuiltinFn::HttpClose => 0,
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
            BuiltinFn::TermPaste => self.finish(
                TExpr::new(
                    TExprKind::Builtin {
                        kind: builtin,
                        args: Vec::new(),
                    },
                    Type::String,
                    span,
                ),
                expected,
            ),
            BuiltinFn::CharString => {
                let value = args
                    .first()
                    .map(|a| self.check_expr(&a.value, Some(&Type::Int)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: vec![value],
                        },
                        Type::String,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::HttpGet => {
                let url = args
                    .first()
                    .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: vec![url],
                        },
                        Type::String,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::JsonString => {
                let source = args
                    .first()
                    .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                let key = args
                    .get(1)
                    .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: vec![source, key],
                        },
                        Type::String,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::HttpListen => {
                let port = args
                    .first()
                    .map(|a| self.check_expr(&a.value, Some(&Type::Int)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: vec![port],
                        },
                        Type::Int,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::HttpAccept | BuiltinFn::HttpClose => self.finish(
                TExpr::new(
                    TExprKind::Builtin {
                        kind: builtin,
                        args: Vec::new(),
                    },
                    Type::Int,
                    span,
                ),
                expected,
            ),
            BuiltinFn::HttpMethod
            | BuiltinFn::HttpPath
            | BuiltinFn::HttpQuery
            | BuiltinFn::HttpBody => self.finish(
                TExpr::new(
                    TExprKind::Builtin {
                        kind: builtin,
                        args: Vec::new(),
                    },
                    Type::String,
                    span,
                ),
                expected,
            ),
            BuiltinFn::HttpHeader | BuiltinFn::HttpForm => {
                let name = args
                    .first()
                    .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: vec![name],
                        },
                        Type::String,
                        span,
                    ),
                    expected,
                )
            }
            BuiltinFn::HttpRespond => {
                let status = args
                    .first()
                    .map(|a| self.check_expr(&a.value, Some(&Type::Int)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                let content_type = args
                    .get(1)
                    .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                let body = args
                    .get(2)
                    .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                    .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                self.finish(
                    TExpr::new(
                        TExprKind::Builtin {
                            kind: builtin,
                            args: vec![status, content_type, body],
                        },
                        Type::Int,
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
        if let Some(full) = self.lookup_method(&recv_ty, method) {
            return self.check_user_method(recv, &full, args, method_span, span, expected);
        }
        if let Type::TraitObject(trait_name) = &recv_ty {
            let sigs = self.trait_sigs.get(trait_name).cloned().unwrap_or_default();
            if let Some((index, (_, params, ret))) = sigs
                .iter()
                .enumerate()
                .find(|(_, (name, _, _))| name == method)
            {
                if args.len() != params.len() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0208",
                        format!(
                            "`{method}` expects {} argument{}, found {}",
                            params.len(),
                            if params.len() == 1 { "" } else { "s" },
                            args.len()
                        ),
                        span,
                    ));
                }
                let mut checked = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let value = self.check_expr(&arg.value, params.get(i));
                    if let Some(expected_arg) = params.get(i) {
                        self.expect_type(&value.ty, expected_arg, value.span);
                    }
                    checked.push(value);
                }
                return self.finish(
                    TExpr::new(
                        TExprKind::DynCall {
                            receiver: Box::new(recv),
                            trait_name: trait_name.clone(),
                            index,
                            args: checked,
                        },
                        ret.clone(),
                        span,
                    ),
                    expected,
                );
            }
            self.diagnostics.push(Diagnostic::error(
                "E0206",
                format!("trait `{trait_name}` has no method `{method}`"),
                method_span,
            ));
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        }
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
                "as_slice" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ListToSlice,
                            args: Vec::new(),
                        },
                        Type::Slice(elem.clone()),
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
            Type::Map(key, value) => match method {
                "len" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::MapLen,
                            args: Vec::new(),
                        },
                        Type::Int,
                        span,
                    );
                    self.finish(te, expected)
                }
                "get" => {
                    self.require_args(method, args, 1, span);
                    let key_expr = args
                        .first()
                        .map(|arg| self.check_expr(&arg.value, Some(key)))
                        .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                    self.require_map_key(key, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::MapGet,
                            args: vec![key_expr],
                        },
                        Type::option((**value).clone()),
                        span,
                    );
                    self.finish(te, expected)
                }
                "contains_key" => {
                    self.require_args(method, args, 1, span);
                    let key_expr = args
                        .first()
                        .map(|arg| self.check_expr(&arg.value, Some(key)))
                        .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                    self.require_map_key(key, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::MapContainsKey,
                            args: vec![key_expr],
                        },
                        Type::Bool,
                        span,
                    );
                    self.finish(te, expected)
                }
                "insert" => {
                    self.require_args(method, args, 2, span);
                    self.require_map_key(key, span);
                    if !self.root_is_mutable(receiver) {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0301",
                                "`insert` requires a mutable map binding",
                                receiver.span,
                            )
                            .with_note("declare the map with `var`"),
                        );
                    }
                    let key_expr = args
                        .first()
                        .map(|arg| self.check_expr(&arg.value, Some(key)))
                        .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                    let value_expr = args
                        .get(1)
                        .map(|arg| self.check_expr(&arg.value, Some(value)))
                        .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                    match self.convert_lvalue(receiver) {
                        Some((map, _)) => TExpr::new(
                            TExprKind::MapInsert {
                                map,
                                key: Box::new(key_expr),
                                value: Box::new(value_expr),
                            },
                            Type::Unit,
                            span,
                        ),
                        None => TExpr::new(TExprKind::Unit, Type::Error, span),
                    }
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::Set(inner) => match method {
                "len" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::SetLen,
                            args: Vec::new(),
                        },
                        Type::Int,
                        span,
                    );
                    self.finish(te, expected)
                }
                "contains" => {
                    self.require_args(method, args, 1, span);
                    self.require_map_key(inner, span);
                    let value = args
                        .first()
                        .map(|arg| self.check_expr(&arg.value, Some(inner)))
                        .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::SetContains,
                            args: vec![value],
                        },
                        Type::Bool,
                        span,
                    );
                    self.finish(te, expected)
                }
                "insert" => {
                    self.require_args(method, args, 1, span);
                    self.require_map_key(inner, span);
                    if !self.root_is_mutable(receiver) {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0301",
                                "`insert` requires a mutable set binding",
                                receiver.span,
                            )
                            .with_note("declare the set with `var`"),
                        );
                    }
                    let value = args
                        .first()
                        .map(|arg| self.check_expr(&arg.value, Some(inner)))
                        .unwrap_or_else(|| TExpr::new(TExprKind::Unit, Type::Error, span));
                    match self.convert_lvalue(receiver) {
                        Some((set, _)) => TExpr::new(
                            TExprKind::SetInsert {
                                set,
                                value: Box::new(value),
                            },
                            Type::Bool,
                            span,
                        ),
                        None => TExpr::new(TExprKind::Unit, Type::Error, span),
                    }
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::Array(elem, _) => match method {
                "len" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ArrayLen,
                            args: Vec::new(),
                        },
                        Type::Int,
                        span,
                    );
                    self.finish(te, expected)
                }
                "as_slice" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::ArrayToSlice,
                            args: Vec::new(),
                        },
                        Type::Slice(elem.clone()),
                        span,
                    );
                    self.finish(te, expected)
                }
                other => self.unknown_method(other, &recv_ty, method_span, span),
            },
            Type::Slice(_) => match method {
                "len" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::SliceLen,
                            args: Vec::new(),
                        },
                        Type::Int,
                        span,
                    );
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
                "contains" | "starts_with" | "ends_with" => {
                    self.require_args(method, args, 1, span);
                    let argument = args
                        .first()
                        .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                        .map(|e| vec![e])
                        .unwrap_or_default();
                    let builtin = match method {
                        "contains" => BuiltinMethod::StringContains,
                        "starts_with" => BuiltinMethod::StringStartsWith,
                        _ => BuiltinMethod::StringEndsWith,
                    };
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: builtin,
                            args: argument,
                        },
                        Type::Bool,
                        span,
                    );
                    self.finish(te, expected)
                }
                "replace" => {
                    self.require_args(method, args, 2, span);
                    let argument = args
                        .iter()
                        .map(|a| self.check_expr(&a.value, Some(&Type::String)))
                        .collect();
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::StringReplace,
                            args: argument,
                        },
                        Type::String,
                        span,
                    );
                    self.finish(te, expected)
                }
                "lower" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::StringLower,
                            args: Vec::new(),
                        },
                        Type::String,
                        span,
                    );
                    self.finish(te, expected)
                }
                "trim" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::StringTrim,
                            args: Vec::new(),
                        },
                        Type::String,
                        span,
                    );
                    self.finish(te, expected)
                }
                "display_width" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::StringDisplayWidth,
                            args: Vec::new(),
                        },
                        Type::Int,
                        span,
                    );
                    self.finish(te, expected)
                }
                "clip" | "wrap" => {
                    self.require_args(method, args, 1, span);
                    let argument = args
                        .first()
                        .map(|a| self.check_expr(&a.value, Some(&Type::Int)))
                        .map(|e| vec![e])
                        .unwrap_or_default();
                    let (builtin, result_ty) = if method == "clip" {
                        (BuiltinMethod::StringClip, Type::String)
                    } else {
                        (
                            BuiltinMethod::StringWrap,
                            Type::List(Box::new(Type::String)),
                        )
                    };
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: builtin,
                            args: argument,
                        },
                        result_ty,
                        span,
                    );
                    self.finish(te, expected)
                }
                "graphemes" => {
                    self.require_args(method, args, 0, span);
                    let te = TExpr::new(
                        TExprKind::Method {
                            receiver: Box::new(recv),
                            method: BuiltinMethod::StringGraphemes,
                            args: Vec::new(),
                        },
                        Type::List(Box::new(Type::String)),
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

    fn check_user_method(
        &mut self,
        recv: TExpr,
        full: &str,
        args: &[ast::Arg],
        method_span: Span,
        span: Span,
        expected: Option<&Type>,
    ) -> TExpr {
        let Some(sig) = self.funcs.get(full).cloned() else {
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        };
        if args.len() != sig.params.len() {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0208",
                    format!(
                        "`{full}` expects {} argument{}, found {}",
                        sig.params.len(),
                        if sig.params.len() == 1 { "" } else { "s" },
                        args.len()
                    ),
                    span,
                )
                .with_label(method_span, "method declared here"),
            );
        }
        let mut checked = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            let ty = sig.params.get(index);
            let value = self.check_expr(&arg.value, ty);
            if let Some(expected_ty) = ty {
                if value.ty != *expected_ty && !value.ty.is_error() && !expected_ty.is_error() {
                    self.diagnostics.push(Diagnostic::error(
                        "E0308",
                        format!(
                            "argument {} to `{full}` has type `{}`, expected `{expected_ty}`",
                            index + 1,
                            value.ty
                        ),
                        value.span,
                    ));
                }
            }
            checked.push(value);
        }
        self.finish(
            TExpr::new(
                TExprKind::UserMethod {
                    receiver: Box::new(recv),
                    name: full.to_string(),
                    args: checked,
                },
                sig.ret.clone(),
                span,
            ),
            expected,
        )
    }

    fn require_map_key(&mut self, ty: &Type, span: Span) {
        if !matches!(ty, Type::Int | Type::Bool | Type::Char | Type::String) && !ty.is_error() {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0306",
                    format!("`{ty}` cannot be used as a map/set key in this build"),
                    span,
                )
                .with_note("supported key types: Int, Bool, Char and String"),
            );
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
        let name = self.strip_module_path(name);
        let target = if self.structs.contains_key(name) {
            Some(name.to_string())
        } else if self.struct_templates.contains_key(name) {
            match expected {
                Some(Type::Struct(spec))
                    if self
                        .specializations
                        .get(spec)
                        .map(|(base, _)| base == name)
                        .unwrap_or(false) =>
                {
                    Some(spec.clone())
                }
                _ => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0210",
                            format!("cannot infer type arguments for struct `{name}`"),
                            name_span,
                        )
                        .with_note(format!(
                            "annotate the binding, e.g. `let value: {name}<Int> = {name} {{ ... }}`"
                        )),
                    );
                    for field in fields {
                        self.check_expr(&field.value, None);
                    }
                    return TExpr::new(TExprKind::Unit, Type::Error, span);
                }
            }
        } else {
            None
        };
        let Some(target) = target else {
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
        let Some(def) = self.structs.get(&target).cloned() else {
            return TExpr::new(TExprKind::Unit, Type::Error, span);
        };
        if let Some(expected) = expected {
            if !expected.is_error()
                && *expected != Type::Struct(target.clone())
                && !matches!(expected, Type::TraitObject(_))
            {
                self.diagnostics.push(Diagnostic::error(
                    "E0308",
                    format!("expected `{expected}`, found `{target}`"),
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
        self.finish(
            TExpr::new(
                TExprKind::StructLit {
                    name: target.clone(),
                    fields: checked_fields,
                },
                Type::Struct(target),
                span,
            ),
            expected,
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
                if let Type::Enum(enum_name) = scrutinee {
                    if let Some((tag, fields)) = self.enum_variant(enum_name, name) {
                        if !fields.is_empty() {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    "E0311",
                                    format!(
                                        "`{name}` carries {} payload field{}; write `{name}(...)`",
                                        fields.len(),
                                        if fields.len() == 1 { "" } else { "s" }
                                    ),
                                    pattern.span,
                                )
                                .with_note("use `_` for each payload field you want to ignore"),
                            );
                            return TPattern::Wildcard;
                        }
                        coverage.variants.insert(name.clone());
                        return TPattern::Enum {
                            tag,
                            bindings: Vec::new(),
                        };
                    }
                }
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
            PatternKind::Tuple(items) => {
                coverage.wildcard = true;
                let item_tys = match scrutinee {
                    Type::Tuple(items) => Some(items),
                    Type::Error => None,
                    other => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0311",
                            format!("tuple pattern cannot match `{other}`"),
                            pattern.span,
                        ));
                        None
                    }
                };
                let mut bindings = Vec::new();
                for (index, item) in items.iter().enumerate() {
                    let ty = item_tys.and_then(|tys| tys.get(index));
                    bindings.push(ty.and_then(|ty| {
                        self.bind_payload(Some(item), ty, pattern.span, pattern.span, "tuple")
                    }));
                }
                if let Some(tys) = item_tys {
                    if tys.len() != items.len() {
                        self.diagnostics.push(Diagnostic::error(
                            "E0311",
                            format!(
                                "tuple pattern has {} element(s), expected {}",
                                items.len(),
                                tys.len()
                            ),
                            pattern.span,
                        ));
                    }
                }
                TPattern::Tuple(bindings)
            }
            PatternKind::Variant {
                name,
                name_span,
                fields,
            } => match scrutinee {
                Type::Option(payload) => match last_segment(name) {
                    "Some" => {
                        coverage.some = true;
                        let id = self.bind_payload(
                            fields.first(),
                            payload,
                            *name_span,
                            pattern.span,
                            "Some",
                        );
                        TPattern::Some(id)
                    }
                    "None" => {
                        coverage.none = true;
                        if !fields.is_empty() {
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
                Type::Result(ok, err) => match last_segment(name) {
                    "Ok" => {
                        coverage.ok = true;
                        let id =
                            self.bind_payload(fields.first(), ok, *name_span, pattern.span, "Ok");
                        TPattern::Ok(id)
                    }
                    "Err" => {
                        coverage.err = true;
                        let id =
                            self.bind_payload(fields.first(), err, *name_span, pattern.span, "Err");
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
                Type::Enum(enum_name) => {
                    let variant = last_segment(name).to_string();
                    let found = self.enum_variant(enum_name, &variant);
                    let Some((tag, payload_tys)) = found else {
                        self.diagnostics.push(Diagnostic::error(
                            "E0311",
                            format!("`{variant}` is not a variant of `{enum_name}`"),
                            *name_span,
                        ));
                        return TPattern::Wildcard;
                    };
                    coverage.variants.insert(variant.clone());
                    if fields.len() != payload_tys.len() {
                        self.diagnostics.push(Diagnostic::error(
                            "E0311",
                            format!(
                                "`{variant}` pattern expects {} payload field{}, found {}",
                                payload_tys.len(),
                                if payload_tys.len() == 1 { "" } else { "s" },
                                fields.len()
                            ),
                            pattern.span,
                        ));
                    }
                    let mut bindings = Vec::new();
                    for (index, payload_ty) in payload_tys.iter().enumerate() {
                        bindings.push(self.bind_payload(
                            fields.get(index),
                            payload_ty,
                            *name_span,
                            pattern.span,
                            &variant,
                        ));
                    }
                    TPattern::Enum { tag, bindings }
                }
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
        inner: Option<&ast::Pattern>,
        payload_ty: &Type,
        variant_span: Span,
        pattern_span: Span,
        variant: &str,
    ) -> Option<LocalId> {
        match inner {
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
            Type::TraitObject(_) => Vec::new(),
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
            Type::Enum(name) => {
                let Some(def) = self.enums.get(name) else {
                    return;
                };
                let missing: Vec<String> = def
                    .variants
                    .iter()
                    .filter(|variant| !coverage.variants.contains(&variant.name))
                    .map(|variant| {
                        if !variant.fields.is_empty() {
                            format!(
                                "{}({})",
                                variant.name,
                                std::iter::repeat("_")
                                    .take(variant.fields.len())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        } else {
                            variant.name.clone()
                        }
                    })
                    .collect();
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
                return;
            }
            Type::Int | Type::Char | Type::String => {
                vec!["_ (a wildcard or binding)"]
            }
            Type::Struct(_)
            | Type::List(_)
            | Type::Tuple(_)
            | Type::Array(_, _)
            | Type::Slice(_)
            | Type::Map(_, _)
            | Type::Set(_)
            | Type::Fn(_, _)
            | Type::Unit
            | Type::Float
            | Type::Never => {
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
            ExprKind::TupleField {
                base,
                index,
                index_span,
            } => {
                let (base_lv, base_ty) = self.convert_lvalue(base)?;
                if let Type::Tuple(items) = base_ty {
                    if let Some(ty) = items.get(*index) {
                        Some((LValue::TupleField(Box::new(base_lv), *index), ty.clone()))
                    } else {
                        self.diagnostics.push(Diagnostic::error(
                            "E0205",
                            format!(
                                "tuple has {} element(s), so `.{index}` is out of range",
                                items.len()
                            ),
                            *index_span,
                        ));
                        None
                    }
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        "E0303",
                        format!("tuple field assignment requires a tuple, found `{base_ty}`"),
                        *index_span,
                    ));
                    None
                }
            }
            ExprKind::Index { base, index } => {
                let (base_lv, base_ty) = self.convert_lvalue(base)?;
                let index = self.check_expr(index, Some(&Type::Int));
                match base_ty {
                    Type::List(elem) | Type::Array(elem, _) | Type::Slice(elem) => Some((
                        LValue::Index(Box::new(base_lv), Box::new(index)),
                        (*elem).clone(),
                    )),
                    other => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0303",
                            format!(
                                "index assignment requires List, array or slice, found `{other}`"
                            ),
                            expr.span,
                        ));
                        None
                    }
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
            ExprKind::Field { base, .. }
            | ExprKind::TupleField { base, .. }
            | ExprKind::Index { base, .. } => self.root_is_mutable(base),
            _ => true,
        }
    }

    // ---------------------------------------------------------------------
    // Scope and type helpers
    // ---------------------------------------------------------------------

    fn finish(&mut self, expr: TExpr, expected: Option<&Type>) -> TExpr {
        if let Some(Type::TraitObject(trait_name)) = expected {
            if expr.ty != Type::TraitObject(trait_name.clone()) && !expr.ty.is_error() {
                let concrete = expr.ty.clone();
                if self
                    .trait_impls
                    .contains_key(&(trait_name.clone(), concrete.to_string()))
                {
                    let span = expr.span;
                    return TExpr::new(
                        TExprKind::TraitObject {
                            value: Box::new(expr),
                            trait_name: trait_name.clone(),
                            value_ty: concrete,
                        },
                        expected.cloned().unwrap(),
                        span,
                    );
                }
            }
        }
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

fn substitute_macro_expr(
    expr: &ast::Expr,
    substitutions: &HashMap<String, ast::Expr>,
) -> ast::Expr {
    let kind = match &expr.kind {
        ast::ExprKind::Ident(name) => {
            if let Some(replacement) = substitutions.get(name) {
                return replacement.clone();
            }
            expr.kind.clone()
        }
        ast::ExprKind::Unary { op, expr: inner } => ast::ExprKind::Unary {
            op: *op,
            expr: Box::new(substitute_macro_expr(inner, substitutions)),
        },
        ast::ExprKind::Binary { op, lhs, rhs } => ast::ExprKind::Binary {
            op: *op,
            lhs: Box::new(substitute_macro_expr(lhs, substitutions)),
            rhs: Box::new(substitute_macro_expr(rhs, substitutions)),
        },
        ast::ExprKind::Call { callee, args } => ast::ExprKind::Call {
            callee: Box::new(substitute_macro_expr(callee, substitutions)),
            args: args
                .iter()
                .map(|arg| ast::Arg {
                    name: arg.name.clone(),
                    name_span: arg.name_span,
                    value: substitute_macro_expr(&arg.value, substitutions),
                })
                .collect(),
        },
        ast::ExprKind::MethodCall {
            receiver,
            method,
            method_span,
            args,
        } => ast::ExprKind::MethodCall {
            receiver: Box::new(substitute_macro_expr(receiver, substitutions)),
            method: method.clone(),
            method_span: *method_span,
            args: args
                .iter()
                .map(|arg| ast::Arg {
                    name: arg.name.clone(),
                    name_span: arg.name_span,
                    value: substitute_macro_expr(&arg.value, substitutions),
                })
                .collect(),
        },
        ast::ExprKind::Field {
            base,
            name,
            name_span,
        } => ast::ExprKind::Field {
            base: Box::new(substitute_macro_expr(base, substitutions)),
            name: name.clone(),
            name_span: *name_span,
        },
        ast::ExprKind::TupleField {
            base,
            index,
            index_span,
        } => ast::ExprKind::TupleField {
            base: Box::new(substitute_macro_expr(base, substitutions)),
            index: *index,
            index_span: *index_span,
        },
        ast::ExprKind::Index { base, index } => ast::ExprKind::Index {
            base: Box::new(substitute_macro_expr(base, substitutions)),
            index: Box::new(substitute_macro_expr(index, substitutions)),
        },
        ast::ExprKind::StructLit {
            name,
            name_span,
            fields,
        } => ast::ExprKind::StructLit {
            name: name.clone(),
            name_span: *name_span,
            fields: fields
                .iter()
                .map(|field| ast::FieldInit {
                    name: field.name.clone(),
                    name_span: field.name_span,
                    value: substitute_macro_expr(&field.value, substitutions),
                })
                .collect(),
        },
        ast::ExprKind::If {
            cond,
            then_block,
            else_branch,
        } => ast::ExprKind::If {
            cond: Box::new(substitute_macro_expr(cond, substitutions)),
            then_block: substitute_macro_block(then_block, substitutions),
            else_branch: else_branch.as_ref().map(|branch| match branch {
                ast::ElseBranch::Block(block) => {
                    ast::ElseBranch::Block(substitute_macro_block(block, substitutions))
                }
                ast::ElseBranch::If(expr) => {
                    ast::ElseBranch::If(Box::new(substitute_macro_expr(expr, substitutions)))
                }
            }),
        },
        ast::ExprKind::Match { scrutinee, arms } => ast::ExprKind::Match {
            scrutinee: Box::new(substitute_macro_expr(scrutinee, substitutions)),
            arms: arms
                .iter()
                .map(|arm| ast::MatchArm {
                    pattern: arm.pattern.clone(),
                    body: substitute_macro_expr(&arm.body, substitutions),
                    span: arm.span,
                })
                .collect(),
        },
        ast::ExprKind::Block(block) => {
            ast::ExprKind::Block(substitute_macro_block(block, substitutions))
        }
        ast::ExprKind::List(items) => ast::ExprKind::List(
            items
                .iter()
                .map(|item| substitute_macro_expr(item, substitutions))
                .collect(),
        ),
        ast::ExprKind::Tuple(items) => ast::ExprKind::Tuple(
            items
                .iter()
                .map(|item| substitute_macro_expr(item, substitutions))
                .collect(),
        ),
        ast::ExprKind::Closure { params, ret, body } => ast::ExprKind::Closure {
            params: params.clone(),
            ret: ret.clone(),
            body: substitute_macro_block(body, substitutions),
        },
        ast::ExprKind::Range {
            start,
            end,
            inclusive,
        } => ast::ExprKind::Range {
            start: Box::new(substitute_macro_expr(start, substitutions)),
            end: Box::new(substitute_macro_expr(end, substitutions)),
            inclusive: *inclusive,
        },
        ast::ExprKind::Paren(inner) => {
            ast::ExprKind::Paren(Box::new(substitute_macro_expr(inner, substitutions)))
        }
        ast::ExprKind::Try(inner) => {
            ast::ExprKind::Try(Box::new(substitute_macro_expr(inner, substitutions)))
        }
        ast::ExprKind::Await(inner) => {
            ast::ExprKind::Await(Box::new(substitute_macro_expr(inner, substitutions)))
        }
        other => other.clone(),
    };
    ast::Expr {
        kind,
        span: expr.span,
    }
}

fn substitute_macro_block(
    block: &ast::Block,
    substitutions: &HashMap<String, ast::Expr>,
) -> ast::Block {
    ast::Block {
        stmts: block
            .stmts
            .iter()
            .map(|stmt| ast::Stmt {
                kind: match &stmt.kind {
                    ast::StmtKind::Let {
                        mutable,
                        name,
                        name_span,
                        ty,
                        init,
                    } => ast::StmtKind::Let {
                        mutable: *mutable,
                        name: name.clone(),
                        name_span: *name_span,
                        ty: ty.clone(),
                        init: substitute_macro_expr(init, substitutions),
                    },
                    ast::StmtKind::Assign { target, op, value } => ast::StmtKind::Assign {
                        target: substitute_macro_expr(target, substitutions),
                        op: *op,
                        value: substitute_macro_expr(value, substitutions),
                    },
                    ast::StmtKind::Expr(expr) => {
                        ast::StmtKind::Expr(substitute_macro_expr(expr, substitutions))
                    }
                    ast::StmtKind::While { cond, body } => ast::StmtKind::While {
                        cond: substitute_macro_expr(cond, substitutions),
                        body: substitute_macro_block(body, substitutions),
                    },
                    ast::StmtKind::For {
                        name,
                        name_span,
                        iter,
                        body,
                    } => ast::StmtKind::For {
                        name: name.clone(),
                        name_span: *name_span,
                        iter: substitute_macro_expr(iter, substitutions),
                        body: substitute_macro_block(body, substitutions),
                    },
                    ast::StmtKind::Return(value) => ast::StmtKind::Return(
                        value
                            .as_ref()
                            .map(|expr| substitute_macro_expr(expr, substitutions)),
                    ),
                    ast::StmtKind::Break => ast::StmtKind::Break,
                    ast::StmtKind::Continue => ast::StmtKind::Continue,
                },
                span: stmt.span,
            })
            .collect(),
        span: block.span,
    }
}

fn type_name_fragment(ty: &Type) -> String {
    ty.to_string()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn type_to_type_expr(ty: &Type, span: Span) -> ast::TypeExpr {
    let named = |name: &str, args: Vec<ast::TypeExpr>| ast::TypeExpr {
        kind: ast::TypeExprKind::Named {
            path: vec![name.to_string()],
            args,
        },
        span,
    };
    match ty {
        Type::Unit => ast::TypeExpr {
            kind: ast::TypeExprKind::Unit,
            span,
        },
        Type::Bool => named("Bool", Vec::new()),
        Type::Int => named("Int", Vec::new()),
        Type::Float => named("Float", Vec::new()),
        Type::Char => named("Char", Vec::new()),
        Type::String => named("String", Vec::new()),
        Type::List(inner) => named("List", vec![type_to_type_expr(inner, span)]),
        Type::Slice(inner) => named("Slice", vec![type_to_type_expr(inner, span)]),
        Type::Set(inner) => named("Set", vec![type_to_type_expr(inner, span)]),
        Type::Option(inner) => named("Option", vec![type_to_type_expr(inner, span)]),
        Type::Map(key, value) => named(
            "Map",
            vec![type_to_type_expr(key, span), type_to_type_expr(value, span)],
        ),
        Type::Result(ok, err) => named(
            "Result",
            vec![type_to_type_expr(ok, span), type_to_type_expr(err, span)],
        ),
        Type::Array(inner, len) => ast::TypeExpr {
            kind: ast::TypeExprKind::Array {
                elem: Box::new(type_to_type_expr(inner, span)),
                len: *len,
            },
            span,
        },
        Type::Tuple(items) => ast::TypeExpr {
            kind: ast::TypeExprKind::Tuple(
                items
                    .iter()
                    .map(|item| type_to_type_expr(item, span))
                    .collect(),
            ),
            span,
        },
        Type::Fn(params, ret) => ast::TypeExpr {
            kind: ast::TypeExprKind::Fn {
                params: params
                    .iter()
                    .map(|param| type_to_type_expr(param, span))
                    .collect(),
                ret: Box::new(type_to_type_expr(ret, span)),
            },
            span,
        },
        Type::TraitObject(name) => ast::TypeExpr {
            kind: ast::TypeExprKind::Dyn(name.clone()),
            span,
        },
        Type::Struct(name) | Type::Enum(name) => named(name, Vec::new()),
        Type::Never => named("Never", Vec::new()),
        Type::Error => named("<error>", Vec::new()),
    }
}

fn substitute_type_expr(
    ty: &ast::TypeExpr,
    substitutions: &HashMap<String, Type>,
) -> ast::TypeExpr {
    match &ty.kind {
        ast::TypeExprKind::Named { path, args } => {
            if path.len() == 1 {
                if let Some(concrete) = substitutions.get(&path[0]) {
                    return type_to_type_expr(concrete, ty.span);
                }
            }
            ast::TypeExpr {
                kind: ast::TypeExprKind::Named {
                    path: path.clone(),
                    args: args
                        .iter()
                        .map(|arg| substitute_type_expr(arg, substitutions))
                        .collect(),
                },
                span: ty.span,
            }
        }
        ast::TypeExprKind::Tuple(items) => ast::TypeExpr {
            kind: ast::TypeExprKind::Tuple(
                items
                    .iter()
                    .map(|item| substitute_type_expr(item, substitutions))
                    .collect(),
            ),
            span: ty.span,
        },
        ast::TypeExprKind::Array { elem, len } => ast::TypeExpr {
            kind: ast::TypeExprKind::Array {
                elem: Box::new(substitute_type_expr(elem, substitutions)),
                len: *len,
            },
            span: ty.span,
        },
        ast::TypeExprKind::Fn { params, ret } => ast::TypeExpr {
            kind: ast::TypeExprKind::Fn {
                params: params
                    .iter()
                    .map(|param| substitute_type_expr(param, substitutions))
                    .collect(),
                ret: Box::new(substitute_type_expr(ret, substitutions)),
            },
            span: ty.span,
        },
        ast::TypeExprKind::Dyn(_) | ast::TypeExprKind::Unit => ty.clone(),
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
        ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } => lvalue_target_span(base),
        _ => expr.span,
    }
}

fn lvalue_target_name(expr: &ast::Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Paren(inner) => lvalue_target_name(inner),
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } => lvalue_target_name(base),
        _ => None,
    }
}

fn target_span(target: &LValue, fallback: Span) -> Span {
    match target {
        LValue::Local(_) => fallback,
        LValue::Field(base, _) | LValue::TupleField(base, _) | LValue::Index(base, _) => {
            target_span(base, fallback)
        }
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
    fn resolves_aliased_module_names() {
        let result = check_src(
            r#"import "util.nova" as util

struct User {
    name: String
}

fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn label(user: util.User) -> String {
    return user.name
}

fn main() {
    let user = User { name: "Ada" }
    println(util.add(2, 3))
    println(label(user))
    }
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn resolves_qualified_imported_items() {
        let result = check_src(
            r#"import "util.nova" as util

struct User {
    name: String
}

enum Mode {
    Idle
    Named(String)
}

fn main() {
    let user = util.User { name: "Ada" }
    let mode = util.Mode.Named(user.name)
    let value = match mode {
        util.Mode.Idle => 0,
        util.Mode.Named(name) => name.len()
    }
    println(value)
}
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn rejects_non_c_abi_extern_signatures() {
        let result = check_src(
            r#"extern "C" fn strlen(text: String) -> Int
fn main() {
    println(strlen("hi"))
}
"#,
        );
        let rendered = format!("{:#?}", result.diagnostics);
        assert!(rendered.contains("E0240"), "{rendered}");
        assert!(result.program.externs.is_empty());
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
    fn accepts_generic_function_declarations() {
        let result = check_src(
            r#"fn first<T>(items: List<T>) -> Option<T> {
    return items.get(0)
}
fn main() { }
"#,
        );
        assert!(errors(&result).is_empty(), "{:#?}", result.diagnostics);
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
