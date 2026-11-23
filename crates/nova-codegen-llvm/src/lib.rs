//! Native code generation through LLVM textual IR and the host C toolchain.
//!
//! # Why textual IR instead of a binding
//!
//! The project evaluated `inkwell`/`llvm-sys` and chose to emit LLVM IR text and
//! hand it to `clang`. This keeps the compiler free of a specific libLLVM
//! version and linker configuration, works with Homebrew LLVM versions 19/22
//! and Apple clang, and still uses the real LLVM optimizer, verifier and native
//! backends. See `docs/adr/0002-llvm-backend.md`.
//!
//! Codegen is intentionally conservative: integer arithmetic calls checked
//! helpers, list indexing is bounds-checked, and every generated module is
//! validated by clang before an executable is produced.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use nova_diag::{LineIndex, Span};
use nova_parser::ast::{AssignOp, BinaryOp};
use nova_runtime::RUNTIME_C;
use nova_sema::{
    BuiltinFn, BuiltinMethod, LValue, LocalId, TBlock, TExpr, TExprKind, TPattern, TStmt,
    TStmtKind, TStrPartKind, Type, TypedFn, TypedProgram,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O2,
    O3,
}

impl OptLevel {
    pub fn flag(self) -> &'static str {
        match self {
            OptLevel::O0 => "-O0",
            OptLevel::O2 => "-O2",
            OptLevel::O3 => "-O3",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub output: PathBuf,
    pub opt_level: OptLevel,
    pub emit_ir: Option<PathBuf>,
    pub debug: bool,
    pub lto: bool,
    /// Overrides the entry function. Used by the test runner so each `@test`
    /// is executed in its own process and a panic does not abort other tests.
    pub entry: Option<String>,
    pub target: Option<String>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            output: PathBuf::from("target/nova/app"),
            opt_level: OptLevel::O2,
            emit_ir: None,
            debug: false,
            lto: false,
            entry: None,
            target: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuildArtifact {
    pub executable: PathBuf,
    pub ir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct BuildError {
    pub message: String,
    pub details: String,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.details.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}\n{}", self.message, self.details)
        }
    }
}

impl std::error::Error for BuildError {}

#[derive(Clone, Debug)]
struct Val {
    ty: Type,
    repr: String,
}

impl Val {
    fn new(ty: Type, repr: impl Into<String>) -> Self {
        Self {
            ty,
            repr: repr.into(),
        }
    }
}

/// Emit textual LLVM IR for a checked program. `entry` overrides the entry
/// function name (used for test binaries).
pub fn generate_ir(
    program: &TypedProgram,
    source: &str,
    entry: Option<&str>,
) -> Result<String, String> {
    ModuleGen::new(program, source).generate(entry)
}

/// Compile a checked program to a native executable using clang.
pub fn compile(
    program: &TypedProgram,
    source: &str,
    options: &BuildOptions,
) -> Result<BuildArtifact, BuildError> {
    let entry = options
        .entry
        .as_deref()
        .or(program.main.as_deref())
        .ok_or_else(|| BuildError {
            message: "no entry point".into(),
            details: "no `main` function and no explicit entry were provided".into(),
        })?;
    let ir = generate_ir(program, source, Some(entry)).map_err(|message| BuildError {
        message: "internal compiler error while generating LLVM IR".into(),
        details: message,
    })?;

    let clang = find_clang()?;
    let UnixTime { dir, stamp } = make_work_dir()?;
    let _ = stamp;
    let ir_path = match &options.emit_ir {
        Some(path) => path.clone(),
        None => dir.join("program.ll"),
    };
    if let Some(parent) = ir_path.parent() {
        fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    }
    fs::write(&ir_path, &ir).map_err(|e| io_error(&ir_path, e))?;

    let runtime_c = dir.join("runtime.c");
    let runtime_o = dir.join("runtime.o");
    fs::write(&runtime_c, RUNTIME_C).map_err(|e| io_error(&runtime_c, e))?;

    let mut cc = Command::new(&clang);
    cc.arg("-O2")
        .arg("-std=c11")
        .arg("-c")
        .arg(&runtime_c)
        .arg("-o")
        .arg(&runtime_o);
    run_checked(&mut cc, "compiling the Nova runtime")?;

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    }

    let mut link = Command::new(&clang);
    link.arg(options.opt_level.flag())
        .arg("-Wno-override-module")
        .arg(&ir_path)
        .arg(&runtime_o)
        .arg("-o")
        .arg(&options.output)
        .arg("-lm");
    if options.debug {
        link.arg("-g");
    }
    if options.lto {
        link.arg("-flto");
    }
    if let Some(target) = &options.target {
        link.arg(format!("--target={target}"));
    }
    run_checked(&mut link, "linking the Nova executable")?;

    let ir_out = options.emit_ir.clone();
    if options.emit_ir.is_none() {
        let _ = fs::remove_dir_all(&dir);
    }
    Ok(BuildArtifact {
        executable: options.output.clone(),
        ir: ir_out,
    })
}

struct UnixTime {
    dir: PathBuf,
    stamp: u128,
}

static BUILD_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn make_work_dir() -> Result<UnixTime, BuildError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // The system clock can return the same nanosecond to concurrent builds;
    // the counter keeps each compilation in its own directory.
    let sequence = BUILD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "nova-build-{}-{}-{}",
        std::process::id(),
        stamp,
        sequence
    ));
    fs::create_dir_all(&dir).map_err(|e| io_error(&dir, e))?;
    Ok(UnixTime { dir, stamp })
}

fn io_error(path: &Path, error: std::io::Error) -> BuildError {
    BuildError {
        message: format!("could not access `{}`", path.display()),
        details: error.to_string(),
    }
}

fn find_clang() -> Result<String, BuildError> {
    let mut candidates = Vec::new();
    if let Ok(cc) = std::env::var("NOVA_CC") {
        if !cc.trim().is_empty() {
            candidates.push(cc);
        }
    }
    candidates.push("clang".to_string());
    candidates.push("cc".to_string());
    for candidate in candidates {
        if Command::new(&candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(candidate);
        }
    }
    Err(BuildError {
        message: "no C toolchain found".into(),
        details: "Nova links native programs with clang. Install Xcode command line tools, set NOVA_CC, or put clang on PATH.".into(),
    })
}

fn run_checked(command: &mut Command, what: &str) -> Result<(), BuildError> {
    let output = command.output().map_err(|e| BuildError {
        message: format!("could not start the toolchain while {what}"),
        details: e.to_string(),
    })?;
    if output.status.success() {
        return Ok(());
    }
    let mut details = String::new();
    if !output.stdout.is_empty() {
        details.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        if !details.is_empty() {
            details.push('\n');
        }
        details.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    Err(BuildError {
        message: format!("{what} failed"),
        details: details.trim_end().to_string(),
    })
}

struct ModuleGen<'a> {
    program: &'a TypedProgram,
    lines: LineIndex,
    out: String,
    struct_types: HashMap<String, String>,
    fn_symbols: HashMap<String, String>,
    string_globals: Vec<String>,
    string_counter: usize,
    tmp_counter: usize,
    label_counter: usize,
    locals: HashMap<LocalId, (String, Type)>,
    terminated: bool,
    loop_stack: Vec<(String, String)>,
    fn_ret: Type,
}

impl<'a> ModuleGen<'a> {
    fn new(program: &'a TypedProgram, source: &str) -> Self {
        let struct_types = program
            .structs
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.clone(), format!("%nova.struct.{i}")))
            .collect();
        let fn_symbols = program
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.name.clone(), format!("nova_user_{i}")))
            .collect();
        Self {
            program,
            lines: LineIndex::new(source),
            out: String::new(),
            struct_types,
            fn_symbols,
            string_globals: Vec::new(),
            string_counter: 0,
            tmp_counter: 0,
            label_counter: 0,
            locals: HashMap::new(),
            terminated: false,
            loop_stack: Vec::new(),
            fn_ret: Type::Unit,
        }
    }

    fn generate(mut self, entry: Option<&str>) -> Result<String, String> {
        let entry_name = entry
            .or(self.program.main.as_deref())
            .ok_or_else(|| "no entry point: define `fn main()`".to_string())?;
        let entry_symbol = self
            .fn_symbols
            .get(entry_name)
            .cloned()
            .ok_or_else(|| format!("entry function `{entry_name}` was not found"))?;
        let entry_ret = self
            .program
            .functions
            .iter()
            .find(|f| f.name == entry_name)
            .map(|f| f.ret.clone())
            .unwrap_or(Type::Unit);

        self.out.push_str("; generated by nova-codegen-llvm\n");
        self.out.push_str("%nova.string = type { ptr, i64 }\n");
        self.out.push_str("%nova.list = type { ptr, i64, i64 }\n");
        let defs: Vec<String> = self
            .program
            .structs
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let fields: Vec<String> = s.fields.iter().map(|f| self.llvm_ty(&f.ty)).collect();
                format!("%nova.struct.{i} = type {{ {} }}", fields.join(", "))
            })
            .collect();
        for def in defs {
            self.out.push_str(&def);
            self.out.push('\n');
        }
        self.out.push('\n');
        self.out.push_str(RUNTIME_DECLS);
        self.out.push('\n');
        self.out.push_str(CHECKED_HELPERS);

        // LLVM rejects a `declare` immediately followed by a `define` of the
        // same symbol, so definitions are emitted directly; forward references
        // from calls are resolved at module scope.
        for f in self.program.functions.clone() {
            self.emit_function(&f);
        }

        let globals = std::mem::take(&mut self.string_globals);
        for global in globals {
            self.out.push_str(&global);
            self.out.push('\n');
        }
        self.out.push('\n');

        let ret_llvm = if entry_ret == Type::Unit {
            "void".to_string()
        } else {
            self.llvm_ty(&entry_ret)
        };
        self.out.push_str("define i32 @main() {\n");
        if ret_llvm == "void" {
            self.out
                .push_str(&format!("  call void @{entry_symbol}()\n"));
        } else {
            self.out.push_str(&format!(
                "  %nova_main_result = call {} @{entry_symbol}()\n",
                ret_llvm
            ));
        }
        self.out.push_str("  ret i32 0\n}\n");

        Ok(self.out)
    }

    // ------------------------------------------------------------------
    // Emit helpers
    // ------------------------------------------------------------------

    fn raw(&mut self, text: &str) {
        self.out.push_str(text);
        if !text.ends_with('\n') {
            self.out.push('\n');
        }
    }

    fn emit(&mut self, text: impl AsRef<str>) {
        self.out.push_str("  ");
        self.out.push_str(text.as_ref());
        self.out.push('\n');
    }

    fn fresh(&mut self) -> String {
        self.tmp_counter += 1;
        format!("%t{}", self.tmp_counter)
    }

    fn fresh_label(&mut self) -> String {
        self.label_counter += 1;
        format!("L{}", self.label_counter)
    }

    fn start_block(&mut self, label: &str) {
        self.out.push_str(label);
        self.out.push_str(":\n");
        self.terminated = false;
    }

    fn br(&mut self, label: &str) {
        self.emit(format!("br label %{label}"));
        self.terminated = true;
    }

    fn cond_br(&mut self, cond: &str, if_true: &str, if_false: &str) {
        self.emit(format!("br i1 {cond}, label %{if_true}, label %{if_false}"));
        self.terminated = true;
    }

    fn alloca(&mut self, llvm_ty: &str) -> String {
        let slot = self.fresh();
        self.emit(format!("{slot} = alloca {llvm_ty}"));
        slot
    }

    fn alloca_ty(&mut self, ty: &Type) -> String {
        let llvm = self.llvm_ty(ty);
        self.alloca(&llvm)
    }

    fn store(&mut self, llvm_ty: &str, value: &str, slot: &str) {
        self.emit(format!("store {llvm_ty} {value}, ptr {slot}"));
    }

    fn load(&mut self, llvm_ty: &str, slot: &str) -> String {
        let value = self.fresh();
        self.emit(format!("{value} = load {llvm_ty}, ptr {slot}"));
        value
    }

    fn insert_value(
        &mut self,
        agg_ty: &str,
        agg: &str,
        field_ty: &str,
        field: &str,
        index: usize,
    ) -> String {
        let value = self.fresh();
        self.emit(format!(
            "{value} = insertvalue {agg_ty} {agg}, {field_ty} {field}, {index}"
        ));
        value
    }

    fn extract_value(&mut self, agg_ty: &str, agg: &str, field_ty: &str, index: usize) -> String {
        let value = self.fresh();
        self.emit(format!("{value} = extractvalue {agg_ty} {agg}, {index}"));
        let _ = field_ty;
        value
    }

    fn call_value(&mut self, ret_ty: &str, name: &str, args: &[(String, String)]) -> String {
        let value = self.fresh();
        let args = args
            .iter()
            .map(|(ty, val)| format!("{ty} {val}"))
            .collect::<Vec<_>>()
            .join(", ");
        self.emit(format!("{value} = call {ret_ty} @{name}({args})"));
        value
    }

    fn call_void(&mut self, name: &str, args: &[(String, String)]) {
        let args = args
            .iter()
            .map(|(ty, val)| format!("{ty} {val}"))
            .collect::<Vec<_>>()
            .join(", ");
        self.emit(format!("call void @{name}({args})"));
    }

    fn line(&self, span: Span) -> usize {
        self.lines.line_of(span.start) + 1
    }

    fn llvm_ty(&self, ty: &Type) -> String {
        match ty {
            Type::Unit | Type::Never | Type::Error => "i8".into(),
            Type::Bool => "i1".into(),
            Type::Int => "i64".into(),
            Type::Float => "double".into(),
            Type::Char => "i32".into(),
            Type::String => "%nova.string".into(),
            Type::List(_) => "%nova.list".into(),
            Type::Option(inner) => format!("{{ i1, {} }}", self.llvm_ty(inner)),
            Type::Result(ok, err) => {
                format!("{{ i1, {}, {} }}", self.llvm_ty(ok), self.llvm_ty(err))
            }
            Type::Struct(name) => self
                .struct_types
                .get(name)
                .cloned()
                .unwrap_or_else(|| "i8".into()),
        }
    }

    fn zero_const(&self, ty: &Type) -> String {
        match ty {
            Type::Bool => "false".into(),
            Type::Int | Type::Char => "0".into(),
            Type::Float => "0.000000e+00".into(),
            Type::Unit | Type::Never | Type::Error => "0".into(),
            _ => "zeroinitializer".into(),
        }
    }

    fn unit(&self) -> Val {
        Val::new(Type::Unit, "0")
    }

    fn align_up(value: u64, align: u64) -> u64 {
        if align <= 1 {
            value
        } else {
            value.div_ceil(align) * align
        }
    }

    fn size_align(&self, ty: &Type) -> (u64, u64) {
        match ty {
            Type::Unit | Type::Never | Type::Error | Type::Bool => (1, 1),
            Type::Char => (4, 4),
            Type::Int | Type::Float => (8, 8),
            Type::String => (16, 8),
            Type::List(_) => (24, 8),
            Type::Option(inner) => {
                let (size, align) = self.size_align(inner);
                let align = align.max(1);
                (Self::align_up(1 + size, align), align)
            }
            Type::Result(ok, err) => {
                let (ok_size, ok_align) = self.size_align(ok);
                let (err_size, err_align) = self.size_align(err);
                let align = ok_align.max(err_align).max(1);
                let err_off = Self::align_up(1 + ok_size, err_align);
                (Self::align_up(err_off + err_size, align), align)
            }
            Type::Struct(name) => {
                let Some(def) = self.program.structs.iter().find(|s| &s.name == name) else {
                    return (1, 1);
                };
                let mut offset = 0u64;
                let mut align = 1u64;
                for field in &def.fields {
                    let (size, field_align) = self.size_align(&field.ty);
                    offset = Self::align_up(offset, field_align);
                    offset += size;
                    align = align.max(field_align);
                }
                (Self::align_up(offset, align).max(1), align)
            }
        }
    }

    // ------------------------------------------------------------------
    // Functions and statements
    // ------------------------------------------------------------------

    fn emit_function(&mut self, f: &TypedFn) {
        self.tmp_counter = 0;
        self.label_counter = 0;
        self.locals.clear();
        self.terminated = false;
        self.loop_stack.clear();
        self.fn_ret = f.ret.clone();

        let symbol = self
            .fn_symbols
            .get(&f.name)
            .cloned()
            .unwrap_or_else(|| "nova_user_unknown".into());
        let ret_llvm = if f.ret == Type::Unit {
            "void".to_string()
        } else {
            self.llvm_ty(&f.ret)
        };
        let params: Vec<String> = f
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                f.locals
                    .iter()
                    .find(|l| &l.id == id)
                    .map(|l| format!("{} %arg{i}", self.llvm_ty(&l.ty)))
            })
            .collect();
        self.out.push_str(&format!(
            "define {ret_llvm} @{symbol}({}) {{\n",
            params.join(", ")
        ));

        for local in &f.locals {
            let ty = self.llvm_ty(&local.ty);
            let slot = self.alloca(&ty);
            self.locals.insert(local.id, (slot, local.ty.clone()));
        }
        for (i, id) in f.params.iter().enumerate() {
            if let Some((slot, ty)) = self.locals.get(id).cloned() {
                let ty = self.llvm_ty(&ty);
                self.store(&ty, &format!("%arg{i}"), &slot);
            }
        }

        let tail = self.gen_block(&f.body);
        if !self.terminated {
            if f.ret == Type::Unit {
                self.emit("ret void");
            } else {
                let ret_ty = self.llvm_ty(&f.ret);
                let value = tail
                    .map(|v| v.repr)
                    .unwrap_or_else(|| self.zero_const(&f.ret));
                self.emit(format!("ret {ret_ty} {value}"));
            }
        }
        self.out.push_str("}\n\n");
    }

    fn gen_block(&mut self, block: &TBlock) -> Option<Val> {
        for stmt in &block.stmts {
            if self.terminated {
                break;
            }
            self.gen_stmt(stmt);
        }
        if self.terminated {
            return None;
        }
        if let Some(tail) = &block.tail {
            Some(self.gen_expr(tail))
        } else {
            None
        }
    }

    fn gen_stmt(&mut self, stmt: &TStmt) {
        match &stmt.kind {
            TStmtKind::Let { local, init } => {
                let value = self.gen_expr(init);
                if self.terminated {
                    return;
                }
                if let Some((slot, ty)) = self.locals.get(local).cloned() {
                    let llvm = self.llvm_ty(&ty);
                    self.store(&llvm, &value.repr, &slot);
                }
            }
            TStmtKind::Assign { target, op, value } => {
                let addr = self.gen_lvalue_addr(target);
                let ty = self.lvalue_ty(target);
                let llvm = self.llvm_ty(&ty);
                if *op == AssignOp::Assign {
                    let rhs = self.gen_expr(value);
                    if !self.terminated {
                        self.store(&llvm, &rhs.repr, &addr);
                    }
                } else {
                    let old_repr = self.load(&llvm, &addr);
                    let old = Val::new(ty.clone(), old_repr);
                    let rhs = self.gen_expr(value);
                    if self.terminated {
                        return;
                    }
                    let new = self.gen_assign_op(*op, &old, &rhs, &ty, stmt.span);
                    self.store(&llvm, &new.repr, &addr);
                }
            }
            TStmtKind::Expr(expr) => {
                self.gen_expr(expr);
            }
            TStmtKind::While { cond, body } => {
                let cond_label = self.fresh_label();
                let body_label = self.fresh_label();
                let exit_label = self.fresh_label();
                self.br(&cond_label);
                self.start_block(&cond_label);
                let cond_val = self.gen_expr(cond);
                if !self.terminated {
                    self.cond_br(&cond_val.repr, &body_label, &exit_label);
                }
                self.start_block(&body_label);
                let loop_labels = (exit_label.clone(), cond_label.clone());
                self.loop_stack.push(loop_labels);
                self.gen_block(body);
                self.loop_stack.pop();
                if !self.terminated {
                    self.br(&cond_label);
                }
                self.start_block(&exit_label);
            }
            TStmtKind::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
            } => self.gen_for_range(*var, start, end, *inclusive, body, stmt.span),
            TStmtKind::ForList { var, list, body } => {
                self.gen_for_list(*var, list, body, stmt.span)
            }
            TStmtKind::Return(value) => {
                if let Some(value) = value {
                    let val = self.gen_expr(value);
                    if self.terminated {
                        return;
                    }
                    if self.fn_ret == Type::Unit {
                        self.emit("ret void");
                    } else {
                        let llvm = self.llvm_ty(&self.fn_ret);
                        self.emit(format!("ret {llvm} {}", val.repr));
                    }
                } else {
                    self.emit("ret void");
                }
                self.terminated = true;
            }
            TStmtKind::Break => {
                if let Some((exit, _)) = self.loop_stack.last() {
                    let exit = exit.clone();
                    self.br(&exit);
                }
            }
            TStmtKind::Continue => {
                if let Some((_, cond)) = self.loop_stack.last() {
                    let cond = cond.clone();
                    self.br(&cond);
                }
            }
        }
    }

    fn gen_for_range(
        &mut self,
        var: LocalId,
        start: &TExpr,
        end: &TExpr,
        inclusive: bool,
        body: &TBlock,
        span: Span,
    ) {
        let start_val = self.gen_expr(start);
        let end_val = self.gen_expr(end);
        if self.terminated {
            return;
        }
        let end_slot = self.alloca("i64");
        self.store("i64", &end_val.repr, &end_slot);
        let Some((var_slot, _)) = self.locals.get(&var).cloned() else {
            return;
        };
        self.store("i64", &start_val.repr, &var_slot);

        let cond_label = self.fresh_label();
        let body_label = self.fresh_label();
        let exit_label = self.fresh_label();
        self.br(&cond_label);
        self.start_block(&cond_label);
        let current = self.load("i64", &var_slot);
        let limit = self.load("i64", &end_slot);
        let cmp = self.fresh();
        self.emit(format!(
            "{cmp} = icmp {} i64 {current}, {limit}",
            if inclusive { "sle" } else { "slt" }
        ));
        self.cond_br(&cmp, &body_label, &exit_label);

        self.start_block(&body_label);
        self.loop_stack
            .push((exit_label.clone(), cond_label.clone()));
        self.gen_block(body);
        self.loop_stack.pop();
        if !self.terminated {
            let current = self.load("i64", &var_slot);
            let next = self.call_value(
                "i64",
                "nova_rt_iadd",
                &[
                    ("i64".into(), current),
                    ("i64".into(), "1".into()),
                    ("i64".into(), self.line(span).to_string()),
                ],
            );
            self.store("i64", &next, &var_slot);
            self.br(&cond_label);
        }
        self.start_block(&exit_label);
    }

    fn gen_for_list(&mut self, var: LocalId, list: &TExpr, body: &TBlock, span: Span) {
        let list_val = self.gen_expr(list);
        if self.terminated {
            return;
        }
        let list_slot = self.alloca("%nova.list");
        self.store("%nova.list", &list_val.repr, &list_slot);
        let idx_slot = self.alloca("i64");
        self.store("i64", "0", &idx_slot);
        let elem_ty = match &list.ty {
            Type::List(inner) => (**inner).clone(),
            _ => return,
        };
        let (elem_size, _) = self.size_align(&elem_ty);
        let elem_llvm = self.llvm_ty(&elem_ty);
        let Some((var_slot, _)) = self.locals.get(&var).cloned() else {
            return;
        };

        let cond_label = self.fresh_label();
        let body_label = self.fresh_label();
        let exit_label = self.fresh_label();
        self.br(&cond_label);
        self.start_block(&cond_label);
        let idx = self.load("i64", &idx_slot);
        let list_load = self.load("%nova.list", &list_slot);
        let len = self.extract_value("%nova.list", &list_load, "i64", 1);
        let cmp = self.fresh();
        self.emit(format!("{cmp} = icmp slt i64 {idx}, {len}"));
        self.cond_br(&cmp, &body_label, &exit_label);

        self.start_block(&body_label);
        self.loop_stack
            .push((exit_label.clone(), cond_label.clone()));
        let idx = self.load("i64", &idx_slot);
        let ptr = self.call_value(
            "ptr",
            "nova_list_at",
            &[
                ("ptr".into(), list_slot.clone()),
                ("i64".into(), idx.clone()),
                ("i64".into(), elem_size.to_string()),
                ("i64".into(), self.line(span).to_string()),
            ],
        );
        let element = self.load(&elem_llvm, &ptr);
        self.store(&elem_llvm, &element, &var_slot);
        self.gen_block(body);
        self.loop_stack.pop();
        if !self.terminated {
            let next = self.fresh();
            self.emit(format!("{next} = add i64 {idx}, 1"));
            self.store("i64", &next, &idx_slot);
            self.br(&cond_label);
        }
        self.start_block(&exit_label);
    }

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------

    fn gen_expr(&mut self, expr: &TExpr) -> Val {
        match &expr.kind {
            TExprKind::Int(v) => Val::new(Type::Int, v.to_string()),
            TExprKind::Float(v) => Val::new(Type::Float, format!("0x{:016X}", v.to_bits())),
            TExprKind::Bool(v) => Val::new(Type::Bool, if *v { "true" } else { "false" }),
            TExprKind::Char(v) => Val::new(Type::Char, (*v as u32).to_string()),
            TExprKind::String(parts) => self.gen_string(parts),
            TExprKind::Local(id) => {
                if let Some((slot, ty)) = self.locals.get(id).cloned() {
                    let llvm = self.llvm_ty(&ty);
                    let repr = self.load(&llvm, &slot);
                    Val::new(ty, repr)
                } else {
                    Val::new(Type::Error, "0")
                }
            }
            TExprKind::Unary { op, expr: inner } => self.gen_unary(*op, inner, &expr.ty, expr.span),
            TExprKind::Binary { op, lhs, rhs } => match op {
                BinaryOp::And | BinaryOp::Or => self.gen_short_circuit(*op, lhs, rhs),
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq => self.gen_compare(*op, lhs, rhs),
                _ => {
                    let left = self.gen_expr(lhs);
                    let right = self.gen_expr(rhs);
                    self.gen_arith(*op, &left, &right, &expr.ty, expr.span)
                }
            },
            TExprKind::Call { name, args } => self.gen_call(name, args, &expr.ty),
            TExprKind::Builtin { kind, args } => self.gen_builtin(*kind, args, expr),
            TExprKind::Method {
                receiver,
                method,
                args,
            } => self.gen_method(receiver, *method, args, &expr.ty, expr.span),
            TExprKind::ListPush { list, value } => {
                let addr = self.gen_lvalue_addr(list);
                let val = self.gen_expr(value);
                if self.terminated {
                    return self.unit();
                }
                let elem_llvm = self.llvm_ty(&val.ty);
                let elem_slot = self.alloca(&elem_llvm);
                self.store(&elem_llvm, &val.repr, &elem_slot);
                let (size, _) = self.size_align(&val.ty);
                self.call_void(
                    "nova_list_push",
                    &[
                        ("ptr".into(), addr),
                        ("ptr".into(), elem_slot),
                        ("i64".into(), size.to_string()),
                    ],
                );
                self.unit()
            }
            TExprKind::Field { base, index } => {
                let base_val = self.gen_expr(base);
                let agg_ty = self.llvm_ty(&base.ty);
                let field_ty = match &base.ty {
                    Type::Struct(name) => self
                        .program
                        .structs
                        .iter()
                        .find(|s| &s.name == name)
                        .and_then(|s| s.fields.get(*index))
                        .map(|f| f.ty.clone())
                        .unwrap_or(Type::Error),
                    _ => Type::Error,
                };
                let field_llvm = self.llvm_ty(&field_ty);
                let value = self.extract_value(&agg_ty, &base_val.repr, &field_llvm, *index);
                Val::new(field_ty, value)
            }
            TExprKind::Index { base, index } => {
                let base_val = self.gen_expr(base);
                let index_val = self.gen_expr(index);
                if self.terminated {
                    return self.unit();
                }
                let list_slot = self.alloca("%nova.list");
                self.store("%nova.list", &base_val.repr, &list_slot);
                let elem_ty = match &base.ty {
                    Type::List(inner) => (**inner).clone(),
                    _ => Type::Error,
                };
                let (size, _) = self.size_align(&elem_ty);
                let ptr = self.call_value(
                    "ptr",
                    "nova_list_at",
                    &[
                        ("ptr".into(), list_slot),
                        ("i64".into(), index_val.repr),
                        ("i64".into(), size.to_string()),
                        ("i64".into(), self.line(expr.span).to_string()),
                    ],
                );
                let elem_llvm = self.llvm_ty(&elem_ty);
                let value = self.load(&elem_llvm, &ptr);
                Val::new(elem_ty, value)
            }
            TExprKind::StructLit { name, fields } => {
                let struct_ty = self
                    .struct_types
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| "i8".into());
                let mut agg = "undef".to_string();
                for (index, field) in fields {
                    let value = self.gen_expr(field);
                    let field_ty = self
                        .program
                        .structs
                        .iter()
                        .find(|s| &s.name == name)
                        .and_then(|s| s.fields.get(*index))
                        .map(|f| f.ty.clone())
                        .unwrap_or(Type::Error);
                    let field_llvm = self.llvm_ty(&field_ty);
                    agg = self.insert_value(&struct_ty, &agg, &field_llvm, &value.repr, *index);
                }
                Val::new(Type::Struct(name.clone()), agg)
            }
            TExprKind::If {
                cond,
                then_block,
                else_branch,
            } => self.gen_if(cond, then_block, else_branch.as_deref(), &expr.ty),
            TExprKind::Match { scrutinee, arms } => {
                self.gen_match(scrutinee, arms, &expr.ty, expr.span)
            }
            TExprKind::Block(block) => {
                if let Some(value) = self.gen_block(block) {
                    value
                } else {
                    self.unit()
                }
            }
            TExprKind::List(items) => self.gen_list(items, &expr.ty),
            TExprKind::OptionSome(inner) => {
                let value = self.gen_expr(inner);
                let inner_ty = match &expr.ty {
                    Type::Option(t) => (**t).clone(),
                    _ => value.ty.clone(),
                };
                self.make_option_some(&expr.ty, &inner_ty, &value)
            }
            TExprKind::OptionNone => self.make_option_none(&expr.ty),
            TExprKind::ResultOk(inner) => {
                let value = self.gen_expr(inner);
                let ok_ty = match &expr.ty {
                    Type::Result(ok, _) => (**ok).clone(),
                    _ => value.ty.clone(),
                };
                self.make_result_ok(&expr.ty, &ok_ty, &value)
            }
            TExprKind::ResultErr(inner) => {
                let value = self.gen_expr(inner);
                let err_ty = match &expr.ty {
                    Type::Result(_, err) => (**err).clone(),
                    _ => value.ty.clone(),
                };
                self.make_result_err(&expr.ty, &err_ty, &value)
            }
            TExprKind::Try(inner) => self.gen_try(inner, expr),
            TExprKind::Unit => self.unit(),
        }
    }

    fn gen_unary(
        &mut self,
        op: nova_parser::ast::UnaryOp,
        inner: &TExpr,
        ty: &Type,
        span: Span,
    ) -> Val {
        let value = self.gen_expr(inner);
        match op {
            nova_parser::ast::UnaryOp::Neg => match ty {
                Type::Int => {
                    let result = self.call_value(
                        "i64",
                        "nova_rt_ineg",
                        &[
                            ("i64".into(), value.repr),
                            ("i64".into(), self.line(span).to_string()),
                        ],
                    );
                    Val::new(Type::Int, result)
                }
                Type::Float => {
                    let result = self.fresh();
                    self.emit(format!("{result} = fneg double {}", value.repr));
                    Val::new(Type::Float, result)
                }
                _ => self.unit(),
            },
            nova_parser::ast::UnaryOp::Not => {
                let result = self.fresh();
                self.emit(format!("{result} = xor i1 {}, true", value.repr));
                Val::new(Type::Bool, result)
            }
        }
    }

    fn gen_short_circuit(&mut self, op: BinaryOp, lhs: &TExpr, rhs: &TExpr) -> Val {
        let slot = self.alloca("i1");
        let left = self.gen_expr(lhs);
        if self.terminated {
            return self.unit();
        }
        self.store("i1", &left.repr, &slot);
        let rhs_label = self.fresh_label();
        let done_label = self.fresh_label();
        if op == BinaryOp::And {
            self.cond_br(&left.repr, &rhs_label, &done_label);
        } else {
            self.cond_br(&left.repr, &done_label, &rhs_label);
        }
        self.start_block(&rhs_label);
        let right = self.gen_expr(rhs);
        self.store("i1", &right.repr, &slot);
        self.br(&done_label);
        self.start_block(&done_label);
        let result = self.load("i1", &slot);
        Val::new(Type::Bool, result)
    }

    fn gen_compare(&mut self, op: BinaryOp, lhs: &TExpr, rhs: &TExpr) -> Val {
        let left = self.gen_expr(lhs);
        let right = self.gen_expr(rhs);
        if self.terminated {
            return self.unit();
        }
        let result = self.fresh();
        match left.ty {
            Type::String => {
                let eq = self.call_value(
                    "i32",
                    "nova_string_eq",
                    &[
                        ("%nova.string".into(), left.repr),
                        ("%nova.string".into(), right.repr),
                    ],
                );
                let predicate = if op == BinaryOp::Eq { "ne" } else { "eq" };
                self.emit(format!("{result} = icmp {predicate} i32 {eq}, 0"));
            }
            Type::Float => {
                let predicate = match op {
                    BinaryOp::Eq => "oeq",
                    BinaryOp::NotEq => "one",
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    _ => "oeq",
                };
                self.emit(format!(
                    "{result} = fcmp {predicate} double {}, {}",
                    left.repr, right.repr
                ));
            }
            Type::Unit => {
                let constant = match op {
                    BinaryOp::Eq => "true",
                    _ => "false",
                };
                return Val::new(Type::Bool, constant);
            }
            _ => {
                let predicate = match op {
                    BinaryOp::Eq => "eq",
                    BinaryOp::NotEq => "ne",
                    BinaryOp::Lt => "slt",
                    BinaryOp::LtEq => "sle",
                    BinaryOp::Gt => "sgt",
                    BinaryOp::GtEq => "sge",
                    _ => "eq",
                };
                let llvm = self.llvm_ty(&left.ty);
                self.emit(format!(
                    "{result} = icmp {predicate} {llvm} {}, {}",
                    left.repr, right.repr
                ));
            }
        }
        Val::new(Type::Bool, result)
    }

    fn gen_arith(&mut self, op: BinaryOp, left: &Val, right: &Val, ty: &Type, span: Span) -> Val {
        let result = self.fresh();
        let line = self.line(span).to_string();
        match ty {
            Type::Int => {
                let helper = match op {
                    BinaryOp::Add => "nova_rt_iadd",
                    BinaryOp::Sub => "nova_rt_isub",
                    BinaryOp::Mul => "nova_rt_imul",
                    BinaryOp::Div => "nova_rt_idiv",
                    BinaryOp::Rem => "nova_rt_irem",
                    _ => return Val::new(Type::Error, "0"),
                };
                let value = self.call_value(
                    "i64",
                    helper,
                    &[
                        ("i64".into(), left.repr.clone()),
                        ("i64".into(), right.repr.clone()),
                        ("i64".into(), line),
                    ],
                );
                Val::new(Type::Int, value)
            }
            Type::Float => {
                let instruction = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => return Val::new(Type::Error, "0"),
                };
                self.emit(format!(
                    "{result} = {instruction} double {}, {}",
                    left.repr, right.repr
                ));
                Val::new(Type::Float, result)
            }
            Type::String if op == BinaryOp::Add => {
                let value = self.call_value(
                    "%nova.string",
                    "nova_string_concat",
                    &[
                        ("%nova.string".into(), left.repr.clone()),
                        ("%nova.string".into(), right.repr.clone()),
                    ],
                );
                Val::new(Type::String, value)
            }
            _ => Val::new(Type::Error, "0"),
        }
    }

    fn gen_assign_op(
        &mut self,
        op: AssignOp,
        left: &Val,
        right: &Val,
        ty: &Type,
        span: Span,
    ) -> Val {
        let binary = match op {
            AssignOp::Add => BinaryOp::Add,
            AssignOp::Sub => BinaryOp::Sub,
            AssignOp::Mul => BinaryOp::Mul,
            AssignOp::Div => BinaryOp::Div,
            AssignOp::Rem => BinaryOp::Rem,
            AssignOp::Assign => BinaryOp::Add,
        };
        self.gen_arith(binary, left, right, ty, span)
    }

    fn gen_call(&mut self, name: &str, args: &[TExpr], ret: &Type) -> Val {
        let symbol = self
            .fn_symbols
            .get(name)
            .cloned()
            .unwrap_or_else(|| "nova_user_unknown".into());
        let mut checked = Vec::new();
        for arg in args {
            checked.push(self.gen_expr(arg));
        }
        if self.terminated {
            return self.unit();
        }
        let arg_tys: Vec<String> = checked.iter().map(|v| self.llvm_ty(&v.ty)).collect();
        let call_args: Vec<(String, String)> = arg_tys
            .into_iter()
            .zip(checked.iter().map(|v| v.repr.clone()))
            .collect();
        if *ret == Type::Unit {
            self.call_void(&symbol, &call_args);
            self.unit()
        } else {
            let llvm = self.llvm_ty(ret);
            let value = self.call_value(&llvm, &symbol, &call_args);
            Val::new(ret.clone(), value)
        }
    }

    fn gen_builtin(&mut self, kind: BuiltinFn, args: &[TExpr], expr: &TExpr) -> Val {
        let line = self.line(expr.span).to_string();
        match kind {
            BuiltinFn::Println | BuiltinFn::Print | BuiltinFn::Eprintln => {
                let newline = if kind == BuiltinFn::Print { "0" } else { "1" };
                let stderr = if kind == BuiltinFn::Eprintln {
                    "1"
                } else {
                    "0"
                };
                let Some(arg) = args.first() else {
                    return self.unit();
                };
                let value = self.gen_expr(arg);
                if self.terminated {
                    return self.unit();
                }
                match value.ty {
                    Type::String => self.call_void(
                        "nova_print_str",
                        &[
                            ("%nova.string".into(), value.repr),
                            ("i32".into(), newline.into()),
                            ("i32".into(), stderr.into()),
                        ],
                    ),
                    Type::Int => self.call_void(
                        "nova_print_i64",
                        &[
                            ("i64".into(), value.repr),
                            ("i32".into(), newline.into()),
                            ("i32".into(), stderr.into()),
                        ],
                    ),
                    Type::Float => self.call_void(
                        "nova_print_f64",
                        &[
                            ("double".into(), value.repr),
                            ("i32".into(), newline.into()),
                            ("i32".into(), stderr.into()),
                        ],
                    ),
                    Type::Bool => {
                        let widened = self.fresh();
                        self.emit(format!("{widened} = zext i1 {} to i32", value.repr));
                        self.call_void(
                            "nova_print_bool",
                            &[
                                ("i32".into(), widened),
                                ("i32".into(), newline.into()),
                                ("i32".into(), stderr.into()),
                            ],
                        );
                    }
                    Type::Char => self.call_void(
                        "nova_print_char",
                        &[
                            ("i32".into(), value.repr),
                            ("i32".into(), newline.into()),
                            ("i32".into(), stderr.into()),
                        ],
                    ),
                    _ => {}
                }
                self.unit()
            }
            BuiltinFn::Assert => {
                if let Some(arg) = args.first() {
                    let value = self.gen_expr(arg);
                    let ok = self.fresh_label();
                    let fail = self.fresh_label();
                    self.cond_br(&value.repr, &ok, &fail);
                    self.start_block(&fail);
                    self.call_void("nova_assert_fail", &[("i64".into(), line)]);
                    self.emit("unreachable");
                    self.terminated = true;
                    self.start_block(&ok);
                }
                self.unit()
            }
            BuiltinFn::AssertEq => {
                if args.len() == 2 {
                    let left = self.gen_expr(&args[0]);
                    let right = self.gen_expr(&args[1]);
                    let ok = self.fresh_label();
                    let fail = self.fresh_label();
                    let condition = self.fresh();
                    match left.ty {
                        Type::String => {
                            let eq = self.call_value(
                                "i32",
                                "nova_string_eq",
                                &[
                                    ("%nova.string".into(), left.repr),
                                    ("%nova.string".into(), right.repr),
                                ],
                            );
                            self.emit(format!("{condition} = icmp ne i32 {eq}, 0"));
                        }
                        Type::Float => {
                            self.emit(format!(
                                "{condition} = fcmp oeq double {}, {}",
                                left.repr, right.repr
                            ));
                        }
                        Type::Unit => {
                            self.emit(format!("{condition} = icmp eq i8 0, 0"));
                        }
                        _ => {
                            let llvm = self.llvm_ty(&left.ty);
                            self.emit(format!(
                                "{condition} = icmp eq {llvm} {}, {}",
                                left.repr, right.repr
                            ));
                        }
                    }
                    self.cond_br(&condition, &ok, &fail);
                    self.start_block(&fail);
                    self.call_void("nova_assert_eq_fail", &[("i64".into(), line)]);
                    self.emit("unreachable");
                    self.terminated = true;
                    self.start_block(&ok);
                }
                self.unit()
            }
            BuiltinFn::Panic => {
                let Some(arg) = args.first() else {
                    return self.unit();
                };
                let message = self.gen_expr(arg);
                self.call_void(
                    "nova_panic_str",
                    &[("%nova.string".into(), message.repr), ("i64".into(), line)],
                );
                self.emit("unreachable");
                self.terminated = true;
                Val::new(Type::Never, "0")
            }
        }
    }

    fn gen_method(
        &mut self,
        receiver: &TExpr,
        method: BuiltinMethod,
        args: &[TExpr],
        ret_ty: &Type,
        span: Span,
    ) -> Val {
        let recv = self.gen_expr(receiver);
        if self.terminated {
            return self.unit();
        }
        let recv_llvm = self.llvm_ty(&recv.ty);
        match method {
            BuiltinMethod::ListLen => {
                let len = self.extract_value(&recv_llvm, &recv.repr, "i64", 1);
                Val::new(Type::Int, len)
            }
            BuiltinMethod::ListIsEmpty => {
                let len = self.extract_value(&recv_llvm, &recv.repr, "i64", 1);
                let result = self.fresh();
                self.emit(format!("{result} = icmp eq i64 {len}, 0"));
                Val::new(Type::Bool, result)
            }
            BuiltinMethod::ListGet => {
                let Some(elem_ty) = (match &recv.ty {
                    Type::List(inner) => Some((**inner).clone()),
                    _ => None,
                }) else {
                    return self.unit();
                };
                let Some(index_expr) = args.first() else {
                    return self.unit();
                };
                let index = self.gen_expr(index_expr);
                let list_slot = self.alloca("%nova.list");
                self.store("%nova.list", &recv.repr, &list_slot);
                let len = self.extract_value("%nova.list", &recv.repr, "i64", 1);
                let neg = self.fresh();
                let too_big = self.fresh();
                let oob = self.fresh();
                self.emit(format!("{neg} = icmp slt i64 {}, 0", index.repr));
                self.emit(format!("{too_big} = icmp sge i64 {}, {len}", index.repr));
                self.emit(format!("{oob} = or i1 {neg}, {too_big}"));
                let some_label = self.fresh_label();
                let none_label = self.fresh_label();
                let done_label = self.fresh_label();
                let option_llvm = self.llvm_ty(ret_ty);
                let option_slot = self.alloca(&option_llvm);
                let none = self.make_option_none(ret_ty);
                let zero = self.zero_const(ret_ty);
                self.store(&option_llvm, &zero, &option_slot);
                let _ = none;
                self.cond_br(&oob, &none_label, &some_label);

                self.start_block(&some_label);
                let (size, _) = self.size_align(&elem_ty);
                let ptr = self.call_value(
                    "ptr",
                    "nova_list_at",
                    &[
                        ("ptr".into(), list_slot.clone()),
                        ("i64".into(), index.repr.clone()),
                        ("i64".into(), size.to_string()),
                        ("i64".into(), self.line(span).to_string()),
                    ],
                );
                let elem_llvm = self.llvm_ty(&elem_ty);
                let loaded = self.load(&elem_llvm, &ptr);
                let element = Val::new(elem_ty.clone(), loaded);
                let some = self.make_option_some(ret_ty, &elem_ty, &element);
                self.store(&option_llvm, &some.repr, &option_slot);
                self.br(&done_label);

                self.start_block(&none_label);
                let none = self.make_option_none(ret_ty);
                self.store(&option_llvm, &none.repr, &option_slot);
                self.br(&done_label);

                self.start_block(&done_label);
                let result = self.load(&option_llvm, &option_slot);
                Val::new(ret_ty.clone(), result)
            }
            BuiltinMethod::ListPush => self.unit(),
            BuiltinMethod::StringLen | BuiltinMethod::StringIsEmpty => {
                let len = self.extract_value("%nova.string", &recv.repr, "i64", 1);
                if method == BuiltinMethod::StringLen {
                    Val::new(Type::Int, len)
                } else {
                    let result = self.fresh();
                    self.emit(format!("{result} = icmp eq i64 {len}, 0"));
                    Val::new(Type::Bool, result)
                }
            }
            BuiltinMethod::OptionIsSome | BuiltinMethod::OptionIsNone => {
                let tag = self.extract_value(&recv_llvm, &recv.repr, "i1", 0);
                if method == BuiltinMethod::OptionIsSome {
                    Val::new(Type::Bool, tag)
                } else {
                    let result = self.fresh();
                    self.emit(format!("{result} = xor i1 {tag}, true"));
                    Val::new(Type::Bool, result)
                }
            }
            BuiltinMethod::OptionUnwrapOr => {
                let Some(default_expr) = args.first() else {
                    return self.unit();
                };
                let default = self.gen_expr(default_expr);
                let inner_llvm = self.llvm_ty(ret_ty);
                let slot = self.alloca(&inner_llvm);
                self.store(&inner_llvm, &default.repr, &slot);
                let tag = self.extract_value(&recv_llvm, &recv.repr, "i1", 0);
                let some_label = self.fresh_label();
                let done_label = self.fresh_label();
                self.cond_br(&tag, &some_label, &done_label);
                self.start_block(&some_label);
                let payload = self.extract_value(&recv_llvm, &recv.repr, &inner_llvm, 1);
                self.store(&inner_llvm, &payload, &slot);
                self.br(&done_label);
                self.start_block(&done_label);
                let result = self.load(&inner_llvm, &slot);
                Val::new(ret_ty.clone(), result)
            }
            BuiltinMethod::ResultIsOk | BuiltinMethod::ResultIsErr => {
                let tag = self.extract_value(&recv_llvm, &recv.repr, "i1", 0);
                if method == BuiltinMethod::ResultIsOk {
                    Val::new(Type::Bool, tag)
                } else {
                    let result = self.fresh();
                    self.emit(format!("{result} = xor i1 {tag}, true"));
                    Val::new(Type::Bool, result)
                }
            }
            BuiltinMethod::ResultUnwrapOr => {
                let Some(default_expr) = args.first() else {
                    return self.unit();
                };
                let default = self.gen_expr(default_expr);
                let inner_llvm = self.llvm_ty(ret_ty);
                let slot = self.alloca(&inner_llvm);
                self.store(&inner_llvm, &default.repr, &slot);
                let tag = self.extract_value(&recv_llvm, &recv.repr, "i1", 0);
                let ok_label = self.fresh_label();
                let done_label = self.fresh_label();
                self.cond_br(&tag, &ok_label, &done_label);
                self.start_block(&ok_label);
                let payload = self.extract_value(&recv_llvm, &recv.repr, &inner_llvm, 1);
                self.store(&inner_llvm, &payload, &slot);
                self.br(&done_label);
                self.start_block(&done_label);
                let result = self.load(&inner_llvm, &slot);
                Val::new(ret_ty.clone(), result)
            }
            BuiltinMethod::ToString => self.to_string_value(recv),
        }
    }

    fn to_string_value(&mut self, value: Val) -> Val {
        match value.ty {
            Type::String => value,
            Type::Int => {
                let result = self.call_value(
                    "%nova.string",
                    "nova_i64_to_string",
                    &[("i64".into(), value.repr)],
                );
                Val::new(Type::String, result)
            }
            Type::Float => {
                let result = self.call_value(
                    "%nova.string",
                    "nova_f64_to_string",
                    &[("double".into(), value.repr)],
                );
                Val::new(Type::String, result)
            }
            Type::Bool => {
                let widened = self.fresh();
                self.emit(format!("{widened} = zext i1 {} to i32", value.repr));
                let result = self.call_value(
                    "%nova.string",
                    "nova_bool_to_string",
                    &[("i32".into(), widened)],
                );
                Val::new(Type::String, result)
            }
            Type::Char => {
                let result = self.call_value(
                    "%nova.string",
                    "nova_char_to_string",
                    &[("i32".into(), value.repr)],
                );
                Val::new(Type::String, result)
            }
            _ => self.string_literal(""),
        }
    }

    fn gen_string(&mut self, parts: &[nova_sema::TStrPart]) -> Val {
        let mut acc = self.string_literal("");
        for part in parts {
            let piece = match &part.kind {
                TStrPartKind::Literal(text) => self.string_literal(text),
                TStrPartKind::Expr(expr) => {
                    let value = self.gen_expr(expr);
                    self.to_string_value(value)
                }
            };
            let result = self.call_value(
                "%nova.string",
                "nova_string_concat",
                &[
                    ("%nova.string".into(), acc.repr),
                    ("%nova.string".into(), piece.repr),
                ],
            );
            acc = Val::new(Type::String, result);
        }
        acc
    }

    fn string_literal(&mut self, text: &str) -> Val {
        let bytes = text.as_bytes();
        let symbol = format!("@nova.str.{}", self.string_counter);
        self.string_counter += 1;
        let definition = if bytes.is_empty() {
            format!("{symbol} = private unnamed_addr constant [1 x i8] zeroinitializer, align 1")
        } else {
            format!(
                "{symbol} = private unnamed_addr constant [{} x i8] c\"{}\", align 1",
                bytes.len(),
                escape_llvm_bytes(bytes)
            )
        };
        self.string_globals.push(definition);
        let first = self.fresh();
        self.emit(format!(
            "{first} = insertvalue %nova.string {{ ptr null, i64 0 }}, ptr {symbol}, 0"
        ));
        let second = self.fresh();
        self.emit(format!(
            "{second} = insertvalue %nova.string {first}, i64 {}, 1",
            bytes.len()
        ));
        Val::new(Type::String, second)
    }

    fn make_option_some(&mut self, option_ty: &Type, elem_ty: &Type, value: &Val) -> Val {
        let option_llvm = self.llvm_ty(option_ty);
        let elem_llvm = self.llvm_ty(elem_ty);
        let first = self.insert_value(&option_llvm, "undef", &elem_llvm, &value.repr, 1);
        let second = self.insert_value(&option_llvm, &first, "i1", "true", 0);
        Val::new(option_ty.clone(), second)
    }

    fn make_option_none(&mut self, option_ty: &Type) -> Val {
        let option_llvm = self.llvm_ty(option_ty);
        let value = self.insert_value(&option_llvm, "zeroinitializer", "i1", "false", 0);
        Val::new(option_ty.clone(), value)
    }

    fn make_result_ok(&mut self, result_ty: &Type, ok_ty: &Type, value: &Val) -> Val {
        let result_llvm = self.llvm_ty(result_ty);
        let ok_llvm = self.llvm_ty(ok_ty);
        let first = self.insert_value(&result_llvm, "undef", &ok_llvm, &value.repr, 1);
        let second = self.insert_value(&result_llvm, &first, "i1", "true", 0);
        Val::new(result_ty.clone(), second)
    }

    fn make_result_err(&mut self, result_ty: &Type, err_ty: &Type, value: &Val) -> Val {
        let result_llvm = self.llvm_ty(result_ty);
        let err_llvm = self.llvm_ty(err_ty);
        let first = self.insert_value(&result_llvm, "undef", &err_llvm, &value.repr, 2);
        let second = self.insert_value(&result_llvm, &first, "i1", "false", 0);
        Val::new(result_ty.clone(), second)
    }

    fn gen_list(&mut self, items: &[TExpr], list_ty: &Type) -> Val {
        let slot = self.alloca("%nova.list");
        self.store("%nova.list", "zeroinitializer", &slot);
        let elem_ty = match list_ty {
            Type::List(inner) => (**inner).clone(),
            _ => return Val::new(Type::Error, "0"),
        };
        let elem_llvm = self.llvm_ty(&elem_ty);
        let (size, _) = self.size_align(&elem_ty);
        for item in items {
            let value = self.gen_expr(item);
            if self.terminated {
                return self.unit();
            }
            let elem_slot = self.alloca(&elem_llvm);
            self.store(&elem_llvm, &value.repr, &elem_slot);
            self.call_void(
                "nova_list_push",
                &[
                    ("ptr".into(), slot.clone()),
                    ("ptr".into(), elem_slot),
                    ("i64".into(), size.to_string()),
                ],
            );
        }
        let value = self.load("%nova.list", &slot);
        Val::new(list_ty.clone(), value)
    }

    fn gen_if(
        &mut self,
        cond: &TExpr,
        then_block: &TBlock,
        else_branch: Option<&TExpr>,
        ty: &Type,
    ) -> Val {
        let result_slot = if *ty == Type::Unit {
            None
        } else {
            let slot = self.alloca_ty(ty);
            let llvm = self.llvm_ty(ty);
            let zero = self.zero_const(ty);
            self.store(&llvm, &zero, &slot);
            Some(slot)
        };
        let cond_value = self.gen_expr(cond);
        if self.terminated {
            return self.unit();
        }
        let then_label = self.fresh_label();
        let else_label = self.fresh_label();
        let merge_label = self.fresh_label();
        self.cond_br(&cond_value.repr, &then_label, &else_label);

        self.start_block(&then_label);
        let then_value = self.gen_block(then_block);
        let mut reaches_merge = false;
        if !self.terminated {
            self.store_result(&result_slot, ty, then_value);
            self.br(&merge_label);
            reaches_merge = true;
        }

        self.start_block(&else_label);
        if let Some(else_expr) = else_branch {
            let else_value = self.gen_expr(else_expr);
            if !self.terminated {
                self.store_result(&result_slot, ty, Some(else_value));
                self.br(&merge_label);
                reaches_merge = true;
            }
        } else {
            self.br(&merge_label);
            reaches_merge = true;
        }

        if !reaches_merge {
            self.terminated = true;
            return self.unit();
        }
        self.start_block(&merge_label);
        self.load_result(&result_slot, ty)
    }

    fn store_result(&mut self, slot: &Option<String>, ty: &Type, value: Option<Val>) {
        let Some(slot) = slot else {
            return;
        };
        if *ty == Type::Unit {
            return;
        }
        let llvm = self.llvm_ty(ty);
        let repr = value.map(|v| v.repr).unwrap_or_else(|| self.zero_const(ty));
        self.store(&llvm, &repr, slot);
    }

    fn load_result(&mut self, slot: &Option<String>, ty: &Type) -> Val {
        match slot {
            Some(slot) => {
                let llvm = self.llvm_ty(ty);
                let value = self.load(&llvm, slot);
                Val::new(ty.clone(), value)
            }
            None => self.unit(),
        }
    }

    fn gen_match(
        &mut self,
        scrutinee: &TExpr,
        arms: &[nova_sema::TArm],
        ty: &Type,
        span: Span,
    ) -> Val {
        let scrut_value = self.gen_expr(scrutinee);
        if self.terminated {
            return self.unit();
        }
        let scrut_llvm = self.llvm_ty(&scrutinee.ty);
        let scrut_slot = self.alloca(&scrut_llvm);
        self.store(&scrut_llvm, &scrut_value.repr, &scrut_slot);
        let result_slot = if *ty == Type::Unit {
            None
        } else {
            let slot = self.alloca_ty(ty);
            let llvm = self.llvm_ty(ty);
            let zero = self.zero_const(ty);
            self.store(&llvm, &zero, &slot);
            Some(slot)
        };
        if arms.is_empty() {
            self.emit("unreachable");
            self.terminated = true;
            return self.unit();
        }
        let merge_label = self.fresh_label();
        let fail_label = self.fresh_label();
        let test_labels: Vec<String> = (0..arms.len()).map(|_| self.fresh_label()).collect();
        let body_labels: Vec<String> = (0..arms.len()).map(|_| self.fresh_label()).collect();
        let mut reaches_merge = false;
        let mut irrefutable = false;

        for index in 0..arms.len() {
            if index > 0 {
                self.start_block(&test_labels[index]);
            }
            let body = body_labels[index].clone();
            let next = if index + 1 < arms.len() {
                test_labels[index + 1].clone()
            } else {
                fail_label.clone()
            };
            let irrefutable_pattern = self.gen_pattern_test(
                &arms[index].pattern,
                &scrutinee.ty,
                &scrut_slot,
                &body,
                &next,
            );
            self.start_block(&body);
            self.bind_pattern(&arms[index].pattern, &scrutinee.ty, &scrut_slot);
            let value = self.gen_expr(&arms[index].body);
            if !self.terminated {
                self.store_result(&result_slot, ty, Some(value));
                self.br(&merge_label);
                reaches_merge = true;
            }
            if irrefutable_pattern {
                irrefutable = true;
                break;
            }
        }

        if !irrefutable {
            self.start_block(&fail_label);
            self.emit("unreachable");
            self.terminated = true;
        }
        let _ = span;
        if !reaches_merge {
            self.terminated = true;
            return self.unit();
        }
        self.start_block(&merge_label);
        self.load_result(&result_slot, ty)
    }

    fn gen_pattern_test(
        &mut self,
        pattern: &TPattern,
        scrutinee_ty: &Type,
        scrut_slot: &str,
        body: &str,
        next: &str,
    ) -> bool {
        let scrut_llvm = self.llvm_ty(scrutinee_ty);
        match pattern {
            TPattern::Wildcard | TPattern::Bind(_) => {
                self.br(body);
                true
            }
            TPattern::Bool(expected) => {
                let value = self.load("i1", scrut_slot);
                let condition = self.fresh();
                self.emit(format!(
                    "{condition} = icmp eq i1 {value}, {}",
                    if *expected { "true" } else { "false" }
                ));
                self.cond_br(&condition, body, next);
                false
            }
            TPattern::Int(expected) => {
                let value = self.load("i64", scrut_slot);
                let condition = self.fresh();
                self.emit(format!("{condition} = icmp eq i64 {value}, {expected}"));
                self.cond_br(&condition, body, next);
                false
            }
            TPattern::Char(expected) => {
                let value = self.load("i32", scrut_slot);
                let condition = self.fresh();
                self.emit(format!(
                    "{condition} = icmp eq i32 {value}, {}",
                    *expected as u32
                ));
                self.cond_br(&condition, body, next);
                false
            }
            TPattern::String(expected) => {
                let value = self.load("%nova.string", scrut_slot);
                let expected = self.string_literal(expected);
                let eq = self.call_value(
                    "i32",
                    "nova_string_eq",
                    &[
                        ("%nova.string".into(), value),
                        ("%nova.string".into(), expected.repr),
                    ],
                );
                let condition = self.fresh();
                self.emit(format!("{condition} = icmp ne i32 {eq}, 0"));
                self.cond_br(&condition, body, next);
                false
            }
            TPattern::Some(_) => {
                let value = self.load(&scrut_llvm, scrut_slot);
                let tag = self.extract_value(&scrut_llvm, &value, "i1", 0);
                self.cond_br(&tag, body, next);
                false
            }
            TPattern::None => {
                let value = self.load(&scrut_llvm, scrut_slot);
                let tag = self.extract_value(&scrut_llvm, &value, "i1", 0);
                self.cond_br(&tag, next, body);
                false
            }
            TPattern::Ok(_) => {
                let value = self.load(&scrut_llvm, scrut_slot);
                let tag = self.extract_value(&scrut_llvm, &value, "i1", 0);
                self.cond_br(&tag, body, next);
                false
            }
            TPattern::Err(_) => {
                let value = self.load(&scrut_llvm, scrut_slot);
                let tag = self.extract_value(&scrut_llvm, &value, "i1", 0);
                self.cond_br(&tag, next, body);
                false
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &TPattern, scrutinee_ty: &Type, scrut_slot: &str) {
        match pattern {
            TPattern::Bind(id) => {
                let value = self.load(&self.llvm_ty(scrutinee_ty), scrut_slot);
                if let Some((slot, ty)) = self.locals.get(id).cloned() {
                    let llvm = self.llvm_ty(&ty);
                    self.store(&llvm, &value, &slot);
                }
            }
            TPattern::Some(Some(id)) | TPattern::Ok(Some(id)) | TPattern::Err(Some(id)) => {
                let index = if matches!(pattern, TPattern::Some(_)) {
                    1
                } else if matches!(pattern, TPattern::Ok(_)) {
                    1
                } else {
                    2
                };
                let payload_ty = match (pattern, scrutinee_ty) {
                    (TPattern::Some(_), Type::Option(inner)) => (**inner).clone(),
                    (TPattern::Ok(_), Type::Result(ok, _)) => (**ok).clone(),
                    (TPattern::Err(_), Type::Result(_, err)) => (**err).clone(),
                    _ => Type::Error,
                };
                let agg_llvm = self.llvm_ty(scrutinee_ty);
                let payload_llvm = self.llvm_ty(&payload_ty);
                let scrut = self.load(&agg_llvm, scrut_slot);
                let payload = self.extract_value(&agg_llvm, &scrut, &payload_llvm, index);
                if let Some((slot, _)) = self.locals.get(id).cloned() {
                    self.store(&payload_llvm, &payload, &slot);
                }
            }
            _ => {}
        }
    }

    fn gen_try(&mut self, inner: &TExpr, expr: &TExpr) -> Val {
        let value = self.gen_expr(inner);
        if self.terminated {
            return self.unit();
        }
        let operand_llvm = self.llvm_ty(&inner.ty);
        let slot = self.alloca(&operand_llvm);
        self.store(&operand_llvm, &value.repr, &slot);
        let loaded = self.load(&operand_llvm, &slot);
        let tag = self.extract_value(&operand_llvm, &loaded, "i1", 0);
        let success = self.fresh_label();
        let failure = self.fresh_label();
        self.cond_br(&tag, &success, &failure);

        self.start_block(&failure);
        let ret = self.fn_ret.clone();
        match (&inner.ty, &ret) {
            (Type::Option(_), Type::Option(_)) => {
                let none = self.make_option_none(&ret);
                let llvm = self.llvm_ty(&ret);
                self.emit(format!("ret {llvm} {}", none.repr));
                self.terminated = true;
            }
            (Type::Result(_, err), Type::Result(_, ret_err)) => {
                let err_ty = (**err).clone();
                let ret_err_ty = (**ret_err).clone();
                let err_llvm = self.llvm_ty(&err_ty);
                let payload = self.extract_value(&operand_llvm, &loaded, &err_llvm, 2);
                let error = Val::new(err_ty, payload);
                let failure_value = self.make_result_err(&ret, &ret_err_ty, &error);
                let llvm = self.llvm_ty(&ret);
                self.emit(format!("ret {llvm} {}", failure_value.repr));
                self.terminated = true;
            }
            _ => {
                self.emit("unreachable");
                self.terminated = true;
            }
        }

        self.start_block(&success);
        let payload_ty = expr.ty.clone();
        let payload_llvm = self.llvm_ty(&payload_ty);
        let payload = self.extract_value(&operand_llvm, &loaded, &payload_llvm, 1);
        Val::new(payload_ty, payload)
    }

    // ------------------------------------------------------------------
    // LValues
    // ------------------------------------------------------------------

    fn lvalue_ty(&self, lvalue: &LValue) -> Type {
        match lvalue {
            LValue::Local(id) => self
                .locals
                .get(id)
                .map(|(_, ty)| ty.clone())
                .unwrap_or(Type::Error),
            LValue::Field(base, index) => match self.lvalue_ty(base) {
                Type::Struct(name) => self
                    .program
                    .structs
                    .iter()
                    .find(|s| s.name == name)
                    .and_then(|s| s.fields.get(*index))
                    .map(|f| f.ty.clone())
                    .unwrap_or(Type::Error),
                _ => Type::Error,
            },
            LValue::Index(base, _) => match self.lvalue_ty(base) {
                Type::List(inner) => (*inner).clone(),
                _ => Type::Error,
            },
        }
    }

    fn gen_lvalue_addr(&mut self, lvalue: &LValue) -> String {
        match lvalue {
            LValue::Local(id) => self
                .locals
                .get(id)
                .map(|(slot, _)| slot.clone())
                .unwrap_or_else(|| "%t0".into()),
            LValue::Field(base, index) => {
                let base_addr = self.gen_lvalue_addr(base);
                let base_ty = self.lvalue_ty(base);
                let struct_ty = match &base_ty {
                    Type::Struct(name) => self
                        .struct_types
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| "i8".into()),
                    _ => "i8".into(),
                };
                let pointer = self.fresh();
                self.emit(format!(
                    "{pointer} = getelementptr {struct_ty}, ptr {base_addr}, i32 0, i32 {index}"
                ));
                pointer
            }
            LValue::Index(base, index) => {
                let base_addr = self.gen_lvalue_addr(base);
                let base_ty = self.lvalue_ty(base);
                let elem_ty = match base_ty {
                    Type::List(inner) => (*inner).clone(),
                    _ => Type::Error,
                };
                let index_value = self.gen_expr(index);
                let (size, _) = self.size_align(&elem_ty);
                self.call_value(
                    "ptr",
                    "nova_list_at",
                    &[
                        ("ptr".into(), base_addr),
                        ("i64".into(), index_value.repr),
                        ("i64".into(), size.to_string()),
                        ("i64".into(), "0".into()),
                    ],
                )
            }
        }
    }
}

fn escape_llvm_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 8);
    for byte in bytes {
        match *byte {
            b'"' | b'\\' => out.push_str(&format!("\\{:02X}", byte)),
            0x20..=0x7e => out.push(*byte as char),
            _ => out.push_str(&format!("\\{:02X}", byte)),
        }
    }
    out
}

const RUNTIME_DECLS: &str = r#"declare void @nova_print_str(%nova.string, i32, i32)
declare void @nova_print_i64(i64, i32, i32)
declare void @nova_print_f64(double, i32, i32)
declare void @nova_print_bool(i32, i32, i32)
declare void @nova_print_char(i32, i32, i32)
declare %nova.string @nova_string_concat(%nova.string, %nova.string)
declare i32 @nova_string_eq(%nova.string, %nova.string)
declare %nova.string @nova_i64_to_string(i64)
declare %nova.string @nova_f64_to_string(double)
declare %nova.string @nova_bool_to_string(i32)
declare %nova.string @nova_char_to_string(i32)
declare ptr @nova_list_at(ptr, i64, i64, i64)
declare void @nova_list_push(ptr, ptr, i64)
declare void @nova_panic_str(%nova.string, i64)
declare void @nova_assert_fail(i64)
declare void @nova_assert_eq_fail(i64)
declare void @nova_panic_overflow(i64)
declare void @nova_panic_divzero(i64)
declare void @nova_panic_neg_overflow(i64)
declare void @nova_panic_bounds(i64, i64, i64)
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)
"#;

const CHECKED_HELPERS: &str = r#"define i64 @nova_rt_iadd(i64 %a, i64 %b, i64 %line) {
entry:
  %pair = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %a, i64 %b)
  %value = extractvalue { i64, i1 } %pair, 0
  %overflow = extractvalue { i64, i1 } %pair, 1
  br i1 %overflow, label %panic, label %ok
panic:
  call void @nova_panic_overflow(i64 %line)
  unreachable
ok:
  ret i64 %value
}

define i64 @nova_rt_isub(i64 %a, i64 %b, i64 %line) {
entry:
  %pair = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %a, i64 %b)
  %value = extractvalue { i64, i1 } %pair, 0
  %overflow = extractvalue { i64, i1 } %pair, 1
  br i1 %overflow, label %panic, label %ok
panic:
  call void @nova_panic_overflow(i64 %line)
  unreachable
ok:
  ret i64 %value
}

define i64 @nova_rt_imul(i64 %a, i64 %b, i64 %line) {
entry:
  %pair = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %a, i64 %b)
  %value = extractvalue { i64, i1 } %pair, 0
  %overflow = extractvalue { i64, i1 } %pair, 1
  br i1 %overflow, label %panic, label %ok
panic:
  call void @nova_panic_overflow(i64 %line)
  unreachable
ok:
  ret i64 %value
}

define i64 @nova_rt_idiv(i64 %a, i64 %b, i64 %line) {
entry:
  %zero = icmp eq i64 %b, 0
  br i1 %zero, label %divzero, label %check
check:
  %min = icmp eq i64 %a, -9223372036854775808
  %neg1 = icmp eq i64 %b, -1
  %overflow = and i1 %min, %neg1
  br i1 %overflow, label %ovf, label %ok
divzero:
  call void @nova_panic_divzero(i64 %line)
  unreachable
ovf:
  call void @nova_panic_overflow(i64 %line)
  unreachable
ok:
  %result = sdiv i64 %a, %b
  ret i64 %result
}

define i64 @nova_rt_irem(i64 %a, i64 %b, i64 %line) {
entry:
  %zero = icmp eq i64 %b, 0
  br i1 %zero, label %divzero, label %check
check:
  %min = icmp eq i64 %a, -9223372036854775808
  %neg1 = icmp eq i64 %b, -1
  %overflow = and i1 %min, %neg1
  br i1 %overflow, label %zero_result, label %ok
divzero:
  call void @nova_panic_divzero(i64 %line)
  unreachable
zero_result:
  ret i64 0
ok:
  %result = srem i64 %a, %b
  ret i64 %result
}

define i64 @nova_rt_ineg(i64 %a, i64 %line) {
entry:
  %pair = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 0, i64 %a)
  %value = extractvalue { i64, i1 } %pair, 0
  %overflow = extractvalue { i64, i1 } %pair, 1
  br i1 %overflow, label %panic, label %ok
panic:
  call void @nova_panic_neg_overflow(i64 %line)
  unreachable
ok:
  ret i64 %value
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use nova_parser::parse;
    use nova_sema::check;

    fn typed(source: &str) -> TypedProgram {
        let parsed = parse(source);
        assert!(
            parsed.diagnostics.is_empty(),
            "parse diagnostics: {:#?}",
            parsed.diagnostics
        );
        let checked = check(&parsed.program);
        assert!(
            !checked.has_errors(),
            "sema diagnostics: {:#?}",
            checked.diagnostics
        );
        checked.program
    }

    fn temp_output(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "nova-codegen-test-{}-{}-{}",
            std::process::id(),
            stamp,
            name
        ))
    }

    #[test]
    fn emits_valid_looking_ir_for_hello() {
        let source = "fn main() { println(\"Hello, World!\") }";
        let program = typed(source);
        let ir = generate_ir(&program, source, None).unwrap();
        assert!(ir.contains("define i32 @main"));
        assert!(ir.contains("nova_print_str"));
        assert!(ir.contains("Hello, World!"));
    }

    #[test]
    fn compiles_and_runs_hello_native() {
        let source = r#"import std.io { println }

fn greet(name: String) -> String {
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
        let program = typed(source);
        let output = temp_output("hello");
        let options = BuildOptions {
            output: output.clone(),
            opt_level: OptLevel::O2,
            ..Default::default()
        };
        let artifact = compile(&program, source, &options).expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(stdout.matches("Hello, World!").count(), 5);
        assert!(stdout.trim_end().ends_with("10"));
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_structs_options_and_match() {
        let source = r#"struct User {
    name: String
    age: Int
}

fn find(ok: Bool) -> Option<User> {
    if ok {
        return Some(User { name: "Ada", age: 36 })
    }
    return None
}

fn main() {
    match find(true) {
        Some(user) => println(user.name)
        None => println("none")
    }
}
"#;
        let program = typed(source);
        let output = temp_output("struct");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(run.status.success());
        assert_eq!(String::from_utf8_lossy(&run.stdout), "Ada\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_lists_and_checked_arith() {
        let source = r#"fn main() {
    var xs = [1, 2, 3]
    xs.push(4)
    for x in xs {
        print(x)
    }
    println("")
    println(xs.len())
}
"#;
        let program = typed(source);
        let output = temp_output("list");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "1234\n4\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn overflow_traps_at_runtime() {
        let source = r#"fn main() {
    var x = 9223372036854775807
    x += 1
    println(x)
}
"#;
        let program = typed(source);
        let output = temp_output("overflow");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(!run.status.success());
        assert!(String::from_utf8_lossy(&run.stderr).contains("overflow"));
        let _ = fs::remove_file(&artifact.executable);
    }
}
