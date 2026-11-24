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
    trait_vtables: HashMap<(String, String), String>,
}

impl<'a> ModuleGen<'a> {
    fn new(program: &'a TypedProgram, source: &str) -> Self {
        let struct_types = program
            .structs
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.clone(), format!("%nova.struct.{i}")))
            .collect();
        let mut fn_symbols: HashMap<String, String> = program
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.name.clone(), format!("nova_user_{i}")))
            .collect();
        for external in &program.externs {
            fn_symbols.insert(external.name.clone(), external.symbol.clone());
        }
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
            trait_vtables: HashMap::new(),
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
        self.out.push_str("%nova.slice = type { ptr, i64 }\n");
        self.out.push_str("%nova.enum = type { i32, ptr }\n");
        self.out.push_str("%nova.fn = type { ptr, ptr }\n");
        self.out.push_str("%nova.dyn = type { ptr, ptr }\n");
        self.out
            .push_str("%nova.map = type { ptr, ptr, i64, i64, i64, i32 }\n");
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
        for external in &self.program.externs {
            let ret = if external.ret == Type::Unit {
                "void".to_string()
            } else {
                self.llvm_ty(&external.ret)
            };
            let params = external
                .params
                .iter()
                .map(|ty| self.llvm_ty(ty))
                .collect::<Vec<_>>()
                .join(", ");
            self.out.push_str(&format!(
                "declare {ret} @{}({params}) nounwind\n",
                external.symbol
            ));
        }
        self.out.push_str(CHECKED_HELPERS);
        self.emit_trait_tables();

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
                .push_str(&format!("  call void @{entry_symbol}(ptr null)\n"));
        } else {
            self.out.push_str(&format!(
                "  %nova_main_result = call {} @{entry_symbol}(ptr null)\n",
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
            Type::Map(_, _) | Type::Set(_) => "%nova.map".into(),
            Type::Slice(_) => "%nova.slice".into(),
            Type::Fn(_, _) => "%nova.fn".into(),
            Type::TraitObject(_) => "%nova.dyn".into(),
            Type::Array(inner, len) => format!("[{len} x {}]", self.llvm_ty(inner)),
            Type::Tuple(items) => format!(
                "{{ {} }}",
                items
                    .iter()
                    .map(|item| self.llvm_ty(item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Option(inner) => format!("{{ i1, {} }}", self.llvm_ty(inner)),
            Type::Result(ok, err) => {
                format!("{{ i1, {}, {} }}", self.llvm_ty(ok), self.llvm_ty(err))
            }
            Type::Struct(name) => self
                .struct_types
                .get(name)
                .cloned()
                .unwrap_or_else(|| "i8".into()),
            Type::Enum(_) => "%nova.enum".into(),
        }
    }

    fn zero_const(&self, ty: &Type) -> String {
        match ty {
            Type::Bool => "false".into(),
            Type::Fn(_, _) => "zeroinitializer".into(),
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

    fn key_kind(ty: &Type) -> i32 {
        match ty {
            Type::Int => 0,
            Type::Bool => 1,
            Type::Char => 2,
            Type::String => 3,
            _ => 0,
        }
    }

    fn size_align(&self, ty: &Type) -> (u64, u64) {
        match ty {
            Type::Unit | Type::Never | Type::Error | Type::Bool => (1, 1),
            Type::Char => (4, 4),
            Type::Int | Type::Float => (8, 8),
            Type::String => (16, 8),
            Type::List(_) | Type::Slice(_) => (16, 8),
            Type::Map(_, _) | Type::Set(_) => (48, 8),
            Type::Fn(_, _) => (16, 8),
            Type::TraitObject(_) => (16, 8),
            Type::Array(inner, len) => {
                let (size, align) = self.size_align(inner);
                (size.saturating_mul(*len as u64).max(1), align)
            }
            Type::Tuple(items) => {
                let mut offset = 0u64;
                let mut align = 1u64;
                for item in items {
                    let (size, item_align) = self.size_align(item);
                    offset = Self::align_up(offset, item_align);
                    offset += size;
                    align = align.max(item_align);
                }
                (Self::align_up(offset, align).max(1), align)
            }
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
            Type::Enum(_) => (16, 8),
        }
    }

    // ------------------------------------------------------------------
    // Functions and statements
    // ------------------------------------------------------------------

    fn emit_trait_tables(&mut self) {
        let impls = self.program.trait_impls.clone();
        for (impl_index, imp) in impls.iter().enumerate() {
            let ty_llvm = self.llvm_ty(&imp.ty);
            let mut thunks = Vec::new();
            for (method_index, method_name) in imp.methods.iter().enumerate() {
                let Some(func) = self
                    .program
                    .functions
                    .iter()
                    .find(|f| &f.name == method_name)
                    .cloned()
                else {
                    continue;
                };
                let symbol = self
                    .fn_symbols
                    .get(method_name)
                    .cloned()
                    .unwrap_or_else(|| "nova_user_unknown".into());
                let thunk = format!("nova_thunk_{impl_index}_{method_index}");
                let ret_llvm = if func.ret == Type::Unit {
                    "void".to_string()
                } else {
                    self.llvm_ty(&func.ret)
                };
                let mut params = vec!["ptr %arg_env".to_string(), "ptr %self_ptr".to_string()];
                let mut call_args = vec![("ptr".to_string(), "null".to_string())];
                let mut arg_index = 1;
                for id in func.params.iter().skip(1) {
                    if let Some(local) = func.locals.iter().find(|l| &l.id == id) {
                        let ty = self.llvm_ty(&local.ty);
                        params.push(format!("{ty} %arg{arg_index}"));
                        call_args.push((ty, format!("%arg{arg_index}")));
                        arg_index += 1;
                    }
                }
                self.out.push_str(&format!(
                    "define {ret_llvm} @{thunk}({}) {{\n",
                    params.join(", ")
                ));
                let self_value = self.fresh();
                self.out
                    .push_str(&format!("  {self_value} = load {ty_llvm}, ptr %self_ptr\n"));
                call_args.insert(1, (ty_llvm.clone(), self_value));
                let args = call_args
                    .iter()
                    .map(|(ty, val)| format!("{ty} {val}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if func.ret == Type::Unit {
                    self.out.push_str(&format!(
                        "  call void @{symbol}({args})\n  ret void\n}}\n\n"
                    ));
                } else {
                    let value = self.fresh();
                    self.out.push_str(&format!(
                        "  {value} = call {ret_llvm} @{symbol}({args})\n  ret {ret_llvm} {value}\n}}\n\n"
                    ));
                }
                thunks.push(thunk);
            }
            let vtable = format!("nova_vtable_{impl_index}");
            if thunks.is_empty() {
                self.out
                    .push_str(&format!("@{vtable} = global [0 x ptr] zeroinitializer\n\n"));
            } else {
                let elements = thunks
                    .iter()
                    .map(|thunk| format!("ptr @{thunk}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.out.push_str(&format!(
                    "@{vtable} = global [{} x ptr] [{elements}]\n\n",
                    thunks.len()
                ));
            }
            self.trait_vtables
                .insert((imp.trait_name.clone(), imp.ty.to_string()), vtable);
        }
    }

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
        let mut params: Vec<String> = vec!["ptr %arg_env".into()];
        for (i, id) in f.params.iter().enumerate() {
            if let Some(local) = f.locals.iter().find(|l| &l.id == id) {
                params.push(format!("{} %arg{}", self.llvm_ty(&local.ty), i + 1));
            }
        }
        self.out.push_str(&format!(
            "define {ret_llvm} @{symbol}({}) {{\n",
            params.join(", ")
        ));

        for local in &f.locals {
            let ty = self.llvm_ty(&local.ty);
            let slot = self.alloca(&ty);
            self.locals.insert(local.id, (slot, local.ty.clone()));
        }
        let mut capture_layout: Vec<(u64, Type)> = Vec::new();
        if !f.captures.is_empty() {
            let mut offset = 0u64;
            let mut align = 1u64;
            for id in &f.captures {
                let ty = f
                    .locals
                    .iter()
                    .find(|l| &l.id == id)
                    .map(|l| l.ty.clone())
                    .unwrap_or(Type::Error);
                let (size, item_align) = self.size_align(&ty);
                offset = Self::align_up(offset, item_align);
                capture_layout.push((offset, ty));
                offset += size;
                align = align.max(item_align);
            }
            let _ = align;
        }
        for (i, id) in f.params.iter().enumerate() {
            if let Some((slot, ty)) = self.locals.get(id).cloned() {
                let ty = self.llvm_ty(&ty);
                self.store(&ty, &format!("%arg{}", i + 1), &slot);
            }
        }
        for (id, (offset, ty)) in f.captures.iter().zip(capture_layout.iter()) {
            let ptr = if *offset == 0 {
                "%arg_env".to_string()
            } else {
                let ptr = self.fresh();
                self.emit(format!(
                    "{ptr} = getelementptr i8, ptr %arg_env, i64 {offset}"
                ));
                ptr
            };
            let llvm = self.llvm_ty(ty);
            let value = self.load(&llvm, &ptr);
            if let Some((slot, _)) = self.locals.get(id).cloned() {
                self.store(&llvm, &value, &slot);
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
            TExprKind::FunctionRef { name } => {
                let symbol = self
                    .fn_symbols
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| "nova_user_unknown".into());
                let with_fn =
                    self.insert_value("%nova.fn", "undef", "ptr", &format!("@{symbol}"), 0);
                let value = self.insert_value("%nova.fn", &with_fn, "ptr", "null", 1);
                Val::new(expr.ty.clone(), value)
            }
            TExprKind::Closure { name, captures } => {
                let symbol = self
                    .fn_symbols
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| "nova_user_unknown".into());
                let mut values = Vec::new();
                for capture in captures {
                    values.push(self.gen_expr(capture));
                }
                if self.terminated {
                    return self.unit();
                }
                let mut offset = 0u64;
                let mut align = 1u64;
                let mut layout = Vec::new();
                for value in &values {
                    let (size, item_align) = self.size_align(&value.ty);
                    offset = Self::align_up(offset, item_align);
                    layout.push((offset, value.ty.clone()));
                    offset += size;
                    align = align.max(item_align);
                }
                let total = Self::align_up(offset, align).max(1);
                let env = if values.is_empty() {
                    "null".to_string()
                } else {
                    self.call_value("ptr", "malloc", &[("i64".into(), total.to_string())])
                };
                for (value, (offset, _)) in values.iter().zip(layout.iter()) {
                    let ptr = if *offset == 0 {
                        env.clone()
                    } else {
                        let ptr = self.fresh();
                        self.emit(format!("{ptr} = getelementptr i8, ptr {env}, i64 {offset}"));
                        ptr
                    };
                    let llvm = self.llvm_ty(&value.ty);
                    self.store(&llvm, &value.repr, &ptr);
                }
                let with_fn =
                    self.insert_value("%nova.fn", "undef", "ptr", &format!("@{symbol}"), 0);
                let result = self.insert_value("%nova.fn", &with_fn, "ptr", &env, 1);
                Val::new(expr.ty.clone(), result)
            }
            TExprKind::FnCall { callee, args } => self.gen_fn_call(callee, args, &expr.ty),
            TExprKind::TraitObject {
                value,
                trait_name,
                value_ty,
            } => {
                let value = self.gen_expr(value);
                if self.terminated {
                    return self.unit();
                }
                let ty_llvm = self.llvm_ty(value_ty);
                let (size, _) = self.size_align(value_ty);
                let data = self.call_value("ptr", "malloc", &[("i64".into(), size.to_string())]);
                self.store(&ty_llvm, &value.repr, &data);
                let vtable = self
                    .trait_vtables
                    .get(&(trait_name.clone(), value_ty.to_string()))
                    .cloned();
                let vtable_ptr = vtable
                    .map(|name| format!("@{name}"))
                    .unwrap_or_else(|| "null".into());
                let first = self.insert_value("%nova.dyn", "undef", "ptr", &data, 0);
                let result = self.insert_value("%nova.dyn", &first, "ptr", &vtable_ptr, 1);
                Val::new(expr.ty.clone(), result)
            }
            TExprKind::DynCall {
                receiver,
                trait_name: _,
                index,
                args,
            } => {
                let recv = self.gen_expr(receiver);
                let mut values = Vec::new();
                for arg in args {
                    values.push(self.gen_expr(arg));
                }
                if self.terminated {
                    return self.unit();
                }
                let slot = self.alloca("%nova.dyn");
                self.store("%nova.dyn", &recv.repr, &slot);
                let dyn_value = self.load("%nova.dyn", &slot);
                let data = self.extract_value("%nova.dyn", &dyn_value, "ptr", 0);
                let vtable = self.extract_value("%nova.dyn", &dyn_value, "ptr", 1);
                let entry = self.fresh();
                self.emit(format!(
                    "{entry} = getelementptr ptr, ptr {vtable}, i64 {index}"
                ));
                let fn_ptr = self.load("ptr", &entry);
                let mut call_args = vec!["ptr null".to_string(), format!("ptr {data}")];
                call_args.extend(
                    values
                        .iter()
                        .map(|value| format!("{} {}", self.llvm_ty(&value.ty), value.repr)),
                );
                let rendered = call_args.join(", ");
                if expr.ty == Type::Unit {
                    self.emit(format!("call void {fn_ptr}({rendered})"));
                    self.unit()
                } else {
                    let llvm = self.llvm_ty(&expr.ty);
                    let result = self.fresh();
                    self.emit(format!("{result} = call {llvm} {fn_ptr}({rendered})"));
                    Val::new(expr.ty.clone(), result)
                }
            }
            TExprKind::Builtin { kind, args } => self.gen_builtin(*kind, args, expr),
            TExprKind::Method {
                receiver,
                method,
                args,
            } => self.gen_method(receiver, *method, args, &expr.ty, expr.span),
            TExprKind::UserMethod {
                receiver,
                name,
                args,
            } => self.gen_user_method(receiver, name, args, &expr.ty),
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
                let elem_ty = match &base.ty {
                    Type::List(inner) | Type::Array(inner, _) | Type::Slice(inner) => {
                        (**inner).clone()
                    }
                    _ => Type::Error,
                };
                let elem_llvm = self.llvm_ty(&elem_ty);
                let ptr = match &base.ty {
                    Type::List(_) => {
                        let list_slot = self.alloca("%nova.list");
                        self.store("%nova.list", &base_val.repr, &list_slot);
                        let (size, _) = self.size_align(&elem_ty);
                        self.call_value(
                            "ptr",
                            "nova_list_at",
                            &[
                                ("ptr".into(), list_slot),
                                ("i64".into(), index_val.repr),
                                ("i64".into(), size.to_string()),
                                ("i64".into(), self.line(expr.span).to_string()),
                            ],
                        )
                    }
                    Type::Array(_, len) => {
                        let array_llvm = self.llvm_ty(&base.ty);
                        let slot = self.alloca(&array_llvm);
                        self.store(&array_llvm, &base_val.repr, &slot);
                        self.bounds_check(&index_val.repr, &len.to_string(), expr.span);
                        let ptr = self.fresh();
                        self.emit(format!(
                            "{ptr} = getelementptr {array_llvm}, ptr {slot}, i64 0, i64 {}",
                            index_val.repr
                        ));
                        ptr
                    }
                    Type::Slice(_) => {
                        let slice_slot = self.alloca("%nova.slice");
                        self.store("%nova.slice", &base_val.repr, &slice_slot);
                        let slice = self.load("%nova.slice", &slice_slot);
                        let data = self.extract_value("%nova.slice", &slice, "ptr", 0);
                        let len = self.extract_value("%nova.slice", &slice, "i64", 1);
                        self.bounds_check(&index_val.repr, &len, expr.span);
                        let ptr = self.fresh();
                        self.emit(format!(
                            "{ptr} = getelementptr {elem_llvm}, ptr {data}, i64 {}",
                            index_val.repr
                        ));
                        ptr
                    }
                    _ => return self.unit(),
                };
                let value = self.load(&elem_llvm, &ptr);
                Val::new(elem_ty, value)
            }
            TExprKind::Tuple(items) => {
                let tuple_llvm = self.llvm_ty(&expr.ty);
                let mut agg = "undef".to_string();
                for (index, item) in items.iter().enumerate() {
                    let value = self.gen_expr(item);
                    let item_llvm = self.llvm_ty(&value.ty);
                    agg = self.insert_value(&tuple_llvm, &agg, &item_llvm, &value.repr, index);
                }
                Val::new(expr.ty.clone(), agg)
            }
            TExprKind::Array(items) => {
                let array_llvm = self.llvm_ty(&expr.ty);
                let elem_ty = match &expr.ty {
                    Type::Array(inner, _) => (**inner).clone(),
                    _ => Type::Error,
                };
                let elem_llvm = self.llvm_ty(&elem_ty);
                let mut agg = "undef".to_string();
                for (index, item) in items.iter().enumerate() {
                    let value = self.gen_expr(item);
                    agg = self.insert_value(&array_llvm, &agg, &elem_llvm, &value.repr, index);
                }
                Val::new(expr.ty.clone(), agg)
            }
            TExprKind::TupleField { base, index } => {
                let base_val = self.gen_expr(base);
                let agg_ty = self.llvm_ty(&base.ty);
                let field_ty = match &base.ty {
                    Type::Tuple(items) => items.get(*index).cloned().unwrap_or(Type::Error),
                    _ => Type::Error,
                };
                let field_llvm = self.llvm_ty(&field_ty);
                let value = self.extract_value(&agg_ty, &base_val.repr, &field_llvm, *index);
                Val::new(field_ty, value)
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
            TExprKind::EnumVariant {
                enum_name,
                tag,
                fields,
            } => {
                let field_types: Vec<Type> = fields.iter().map(|field| field.ty.clone()).collect();
                let raw = if fields.is_empty() {
                    "null".into()
                } else {
                    let (offsets, total) = self.enum_payload_layout(&field_types);
                    let raw = self.call_value(
                        "ptr",
                        "malloc",
                        &[("i64".into(), total.max(1).to_string())],
                    );
                    for (index, value_expr) in fields.iter().enumerate() {
                        let value = self.gen_expr(value_expr);
                        let value_llvm = self.llvm_ty(&value.ty);
                        let offset = offsets[index];
                        let ptr = if offset == 0 {
                            raw.clone()
                        } else {
                            let ptr = self.fresh();
                            self.emit(format!("{ptr} = getelementptr i8, ptr {raw}, i64 {offset}"));
                            ptr
                        };
                        self.store(&value_llvm, &value.repr, &ptr);
                    }
                    raw
                };
                let tagged =
                    self.insert_value("%nova.enum", "zeroinitializer", "i32", &tag.to_string(), 0);
                let value = self.insert_value("%nova.enum", &tagged, "ptr", &raw, 1);
                Val::new(Type::Enum(enum_name.clone()), value)
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
            TExprKind::MapEmpty | TExprKind::SetEmpty => {
                let (entry_ty, key_ty) = match &expr.ty {
                    Type::Map(key, value) => (
                        Type::Tuple(vec![(**key).clone(), (**value).clone()]),
                        (**key).clone(),
                    ),
                    Type::Set(inner) => ((**inner).clone(), (**inner).clone()),
                    _ => (Type::Error, Type::Error),
                };
                let (entry_size, _) = self.size_align(&entry_ty);
                let kind = Self::key_kind(&key_ty);
                let with_size = self.insert_value(
                    "%nova.map",
                    "zeroinitializer",
                    "i64",
                    &entry_size.to_string(),
                    4,
                );
                let value = self.insert_value("%nova.map", &with_size, "i32", &kind.to_string(), 5);
                Val::new(expr.ty.clone(), value)
            }
            TExprKind::MapInsert { map, key, value } => {
                self.gen_map_insert(map, key, value, expr.span)
            }
            TExprKind::SetInsert { set, value } => self.gen_set_insert(set, value, expr.span),
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
        let is_extern = self.program.externs.iter().any(|e| e.name == name);
        let mut checked = Vec::new();
        for arg in args {
            checked.push(self.gen_expr(arg));
        }
        if self.terminated {
            return self.unit();
        }
        let arg_tys: Vec<String> = checked.iter().map(|v| self.llvm_ty(&v.ty)).collect();
        let mut call_args: Vec<(String, String)> = arg_tys
            .into_iter()
            .zip(checked.iter().map(|v| v.repr.clone()))
            .collect();
        if !is_extern {
            call_args.insert(0, ("ptr".into(), "null".into()));
        }
        if *ret == Type::Unit {
            self.call_void(&symbol, &call_args);
            self.unit()
        } else {
            let llvm = self.llvm_ty(ret);
            let value = self.call_value(&llvm, &symbol, &call_args);
            Val::new(ret.clone(), value)
        }
    }

    fn map_entry_type(&self, map_ty: &Type) -> Type {
        match map_ty {
            Type::Map(key, value) => Type::Tuple(vec![(**key).clone(), (**value).clone()]),
            _ => Type::Error,
        }
    }

    fn gen_map_insert(
        &mut self,
        map: &LValue,
        key_expr: &TExpr,
        value_expr: &TExpr,
        _span: Span,
    ) -> Val {
        let map_addr = self.gen_lvalue_addr(map);
        let map_ty = self.lvalue_ty(map);
        let key = self.gen_expr(key_expr);
        let value = self.gen_expr(value_expr);
        if self.terminated {
            return self.unit();
        }
        let entry_ty = self.map_entry_type(&map_ty);
        let entry_llvm = self.llvm_ty(&entry_ty);
        let key_llvm = self.llvm_ty(&key.ty);
        let value_llvm = self.llvm_ty(&value.ty);
        let first = self.insert_value(&entry_llvm, "undef", &key_llvm, &key.repr, 0);
        let entry = self.insert_value(&entry_llvm, &first, &value_llvm, &value.repr, 1);
        let entry_slot = self.alloca(&entry_llvm);
        self.store(&entry_llvm, &entry, &entry_slot);
        self.call_void(
            "nova_map_insert",
            &[("ptr".into(), map_addr), ("ptr".into(), entry_slot)],
        );
        self.unit()
    }

    fn gen_map_get(&mut self, map: &Val, args: &[TExpr], ret_ty: &Type, _span: Span) -> Val {
        let key_ty = match &map.ty {
            Type::Map(key, _) => (**key).clone(),
            _ => return self.unit(),
        };
        let value_ty = match ret_ty {
            Type::Option(value) => (**value).clone(),
            _ => return self.unit(),
        };
        let Some(key_expr) = args.first() else {
            return self.unit();
        };
        let key = self.gen_expr(key_expr);
        let key_llvm = self.llvm_ty(&key_ty);
        let key_slot = self.alloca(&key_llvm);
        self.store(&key_llvm, &key.repr, &key_slot);
        let map_slot = self.alloca("%nova.map");
        self.store("%nova.map", &map.repr, &map_slot);
        let entry_ty = self.map_entry_type(&map.ty);
        let entry_llvm = self.llvm_ty(&entry_ty);
        let option_llvm = self.llvm_ty(ret_ty);
        let result_slot = self.alloca(&option_llvm);
        let found = self.call_value(
            "ptr",
            "nova_map_get",
            &[("ptr".into(), map_slot), ("ptr".into(), key_slot)],
        );
        let some_label = self.fresh_label();
        let none_label = self.fresh_label();
        let done = self.fresh_label();
        let missing = self.fresh();
        self.emit(format!("{missing} = icmp eq ptr {found}, null"));
        self.cond_br(&missing, &none_label, &some_label);
        self.start_block(&some_label);
        let entry = self.load(&entry_llvm, &found);
        let value_llvm = self.llvm_ty(&value_ty);
        let value = Val::new(
            value_ty.clone(),
            self.extract_value(&entry_llvm, &entry, &value_llvm, 1),
        );
        let some = self.make_option_some(ret_ty, &value_ty, &value);
        self.store(&option_llvm, &some.repr, &result_slot);
        self.br(&done);
        self.start_block(&none_label);
        let none = self.make_option_none(ret_ty);
        self.store(&option_llvm, &none.repr, &result_slot);
        self.br(&done);
        self.start_block(&done);
        let result = self.load(&option_llvm, &result_slot);
        Val::new(ret_ty.clone(), result)
    }

    fn gen_map_contains(&mut self, map: &Val, args: &[TExpr], _span: Span) -> Val {
        let key_ty = match &map.ty {
            Type::Map(key, _) => (**key).clone(),
            _ => return self.unit(),
        };
        let Some(key_expr) = args.first() else {
            return self.unit();
        };
        let key = self.gen_expr(key_expr);
        let key_llvm = self.llvm_ty(&key_ty);
        let key_slot = self.alloca(&key_llvm);
        self.store(&key_llvm, &key.repr, &key_slot);
        let map_slot = self.alloca("%nova.map");
        self.store("%nova.map", &map.repr, &map_slot);
        let found = self.call_value(
            "ptr",
            "nova_map_get",
            &[("ptr".into(), map_slot), ("ptr".into(), key_slot)],
        );
        let result = self.fresh();
        self.emit(format!("{result} = icmp ne ptr {found}, null"));
        Val::new(Type::Bool, result)
    }

    fn gen_set_insert(&mut self, set: &LValue, value_expr: &TExpr, _span: Span) -> Val {
        let set_addr = self.gen_lvalue_addr(set);
        let set_ty = self.lvalue_ty(set);
        let elem_ty = match set_ty {
            Type::Set(inner) => (*inner).clone(),
            _ => Type::Error,
        };
        let value = self.gen_expr(value_expr);
        let elem_llvm = self.llvm_ty(&elem_ty);
        let value_slot = self.alloca(&elem_llvm);
        self.store(&elem_llvm, &value.repr, &value_slot);
        let inserted = self.call_value(
            "i32",
            "nova_map_add",
            &[("ptr".into(), set_addr), ("ptr".into(), value_slot)],
        );
        let result = self.fresh();
        self.emit(format!("{result} = icmp ne i32 {inserted}, 0"));
        Val::new(Type::Bool, result)
    }

    fn gen_set_contains(&mut self, set: &Val, args: &[TExpr], _span: Span) -> Val {
        let elem_ty = match &set.ty {
            Type::Set(inner) => (**inner).clone(),
            _ => return self.unit(),
        };
        let Some(value_expr) = args.first() else {
            return self.unit();
        };
        let value = self.gen_expr(value_expr);
        let elem_llvm = self.llvm_ty(&elem_ty);
        let value_slot = self.alloca(&elem_llvm);
        self.store(&elem_llvm, &value.repr, &value_slot);
        let set_slot = self.alloca("%nova.map");
        self.store("%nova.map", &set.repr, &set_slot);
        let found = self.call_value(
            "ptr",
            "nova_map_get",
            &[("ptr".into(), set_slot), ("ptr".into(), value_slot)],
        );
        let result = self.fresh();
        self.emit(format!("{result} = icmp ne ptr {found}, null"));
        Val::new(Type::Bool, result)
    }

    fn gen_fn_call(&mut self, callee: &TExpr, args: &[TExpr], ret: &Type) -> Val {
        let function = self.gen_expr(callee);
        if self.terminated {
            return self.unit();
        }
        let mut values = Vec::new();
        for arg in args {
            values.push(self.gen_expr(arg));
        }
        if self.terminated {
            return self.unit();
        }
        let slot = self.alloca("%nova.fn");
        self.store("%nova.fn", &function.repr, &slot);
        let loaded = self.load("%nova.fn", &slot);
        let fn_ptr = self.extract_value("%nova.fn", &loaded, "ptr", 0);
        let env = self.extract_value("%nova.fn", &loaded, "ptr", 1);
        let mut call_args = vec![format!("ptr {env}")];
        call_args.extend(
            values
                .iter()
                .map(|value| format!("{} {}", self.llvm_ty(&value.ty), value.repr)),
        );
        let call_args = call_args.join(", ");
        if *ret == Type::Unit {
            self.emit(format!("call void {fn_ptr}({call_args})"));
            self.unit()
        } else {
            let llvm = self.llvm_ty(ret);
            let value = self.fresh();
            self.emit(format!("{value} = call {llvm} {fn_ptr}({call_args})"));
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
            BuiltinMethod::MapLen | BuiltinMethod::SetLen => {
                let map_slot = self.alloca("%nova.map");
                self.store("%nova.map", &recv.repr, &map_slot);
                let len = self.call_value("i64", "nova_map_len", &[("ptr".into(), map_slot)]);
                Val::new(Type::Int, len)
            }
            BuiltinMethod::MapGet => self.gen_map_get(&recv, args, ret_ty, span),
            BuiltinMethod::MapContainsKey => self.gen_map_contains(&recv, args, span),
            BuiltinMethod::SetContains => self.gen_set_contains(&recv, args, span),
            BuiltinMethod::ListToSlice => {
                let data = self.extract_value("%nova.list", &recv.repr, "ptr", 0);
                let len = self.extract_value("%nova.list", &recv.repr, "i64", 1);
                let first = self.insert_value("%nova.slice", "undef", "ptr", &data, 0);
                let value = self.insert_value("%nova.slice", &first, "i64", &len, 1);
                Val::new(ret_ty.clone(), value)
            }
            BuiltinMethod::ArrayLen => {
                let len = match recv.ty {
                    Type::Array(_, len) => len,
                    _ => 0,
                };
                Val::new(Type::Int, len.to_string())
            }
            BuiltinMethod::ArrayToSlice => {
                let (elem_ty, len) = match &recv.ty {
                    Type::Array(inner, len) => ((**inner).clone(), *len),
                    _ => (Type::Error, 0),
                };
                let array_llvm = self.llvm_ty(&recv.ty);
                let slot = self.alloca(&array_llvm);
                self.store(&array_llvm, &recv.repr, &slot);
                let _ = elem_ty;
                let first = self.insert_value("%nova.slice", "undef", "ptr", &slot, 0);
                let value = self.insert_value("%nova.slice", &first, "i64", &len.to_string(), 1);
                Val::new(ret_ty.clone(), value)
            }
            BuiltinMethod::SliceLen => {
                let len = self.extract_value("%nova.slice", &recv.repr, "i64", 1);
                Val::new(Type::Int, len)
            }
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
            BuiltinMethod::StringContains
            | BuiltinMethod::StringStartsWith
            | BuiltinMethod::StringEndsWith => {
                let Some(argument) = args.first() else {
                    return self.unit();
                };
                let needle = self.gen_expr(argument);
                if self.terminated {
                    return self.unit();
                }
                let symbol = match method {
                    BuiltinMethod::StringContains => "nova_string_contains",
                    BuiltinMethod::StringStartsWith => "nova_string_starts_with",
                    _ => "nova_string_ends_with",
                };
                let raw = self.call_value(
                    "i32",
                    symbol,
                    &[
                        ("%nova.string".into(), recv.repr.clone()),
                        ("%nova.string".into(), needle.repr.clone()),
                    ],
                );
                let result = self.fresh();
                self.emit(format!("{result} = icmp ne i32 {raw}, 0"));
                Val::new(Type::Bool, result)
            }
            BuiltinMethod::StringTrim => {
                let result = self.call_value(
                    "%nova.string",
                    "nova_string_trim",
                    &[("%nova.string".into(), recv.repr.clone())],
                );
                Val::new(Type::String, result)
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
            TPattern::Enum { tag, .. } => {
                let value = self.load(&scrut_llvm, scrut_slot);
                let actual = self.extract_value(&scrut_llvm, &value, "i32", 0);
                let condition = self.fresh();
                self.emit(format!("{condition} = icmp eq i32 {actual}, {tag}"));
                self.cond_br(&condition, body, next);
                false
            }
            TPattern::Tuple(_) => {
                self.br(body);
                true
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
            TPattern::Tuple(bindings) => {
                let Type::Tuple(items) = scrutinee_ty else {
                    return;
                };
                let tuple_llvm = self.llvm_ty(scrutinee_ty);
                let aggregate = self.load(&tuple_llvm, scrut_slot);
                for (index, binding) in bindings.iter().enumerate() {
                    let Some(id) = binding else {
                        continue;
                    };
                    let Some(item_ty) = items.get(index) else {
                        continue;
                    };
                    let item_llvm = self.llvm_ty(item_ty);
                    let value = self.extract_value(&tuple_llvm, &aggregate, &item_llvm, index);
                    if let Some((slot, _)) = self.locals.get(id).cloned() {
                        self.store(&item_llvm, &value, &slot);
                    }
                }
            }
            TPattern::Enum { tag, bindings } => {
                let Type::Enum(enum_name) = scrutinee_ty else {
                    return;
                };
                let Some(def) = self.program.enums.iter().find(|e| &e.name == enum_name) else {
                    return;
                };
                let Some(variant) = def.variants.get(*tag as usize) else {
                    return;
                };
                let (offsets, _) = self.enum_payload_layout(&variant.fields);
                let agg = self.load("%nova.enum", scrut_slot);
                let raw = self.extract_value("%nova.enum", &agg, "ptr", 1);
                for (index, binding) in bindings.iter().enumerate() {
                    let Some(id) = binding else {
                        continue;
                    };
                    let Some(payload_ty) = variant.fields.get(index) else {
                        continue;
                    };
                    let offset = offsets[index];
                    let ptr = if offset == 0 {
                        raw.clone()
                    } else {
                        let ptr = self.fresh();
                        self.emit(format!("{ptr} = getelementptr i8, ptr {raw}, i64 {offset}"));
                        ptr
                    };
                    let payload_llvm = self.llvm_ty(payload_ty);
                    let value = self.load(&payload_llvm, &ptr);
                    if let Some((slot, _)) = self.locals.get(id).cloned() {
                        self.store(&payload_llvm, &value, &slot);
                    }
                }
            }
            _ => {}
        }
    }

    fn bounds_check(&mut self, index: &str, len: &str, span: Span) {
        let neg = self.fresh();
        let too_big = self.fresh();
        let oob = self.fresh();
        self.emit(format!("{neg} = icmp slt i64 {index}, 0"));
        self.emit(format!("{too_big} = icmp sge i64 {index}, {len}"));
        self.emit(format!("{oob} = or i1 {neg}, {too_big}"));
        let ok = self.fresh_label();
        let panic = self.fresh_label();
        self.cond_br(&oob, &panic, &ok);
        self.start_block(&panic);
        self.call_void(
            "nova_panic_bounds",
            &[
                ("i64".into(), index.to_string()),
                ("i64".into(), len.to_string()),
                ("i64".into(), self.line(span).to_string()),
            ],
        );
        self.emit("unreachable");
        self.terminated = true;
        self.start_block(&ok);
    }

    fn enum_payload_layout(&self, fields: &[Type]) -> (Vec<u64>, u64) {
        let mut offset = 0u64;
        let mut offsets = Vec::with_capacity(fields.len());
        let mut max_align = 1u64;
        for field in fields {
            let (size, align) = self.size_align(field);
            let align = align.max(1);
            offset = Self::align_up(offset, align);
            offsets.push(offset);
            offset += size;
            max_align = max_align.max(align);
        }
        (offsets, Self::align_up(offset, max_align).max(1))
    }

    fn gen_user_method(&mut self, receiver: &TExpr, name: &str, args: &[TExpr], ret: &Type) -> Val {
        let symbol = self
            .fn_symbols
            .get(name)
            .cloned()
            .unwrap_or_else(|| "nova_user_unknown".into());
        let recv = self.gen_expr(receiver);
        let mut checked = vec![recv];
        for arg in args {
            checked.push(self.gen_expr(arg));
        }
        if self.terminated {
            return self.unit();
        }
        let mut call_args: Vec<(String, String)> = checked
            .iter()
            .map(|value| (self.llvm_ty(&value.ty), value.repr.clone()))
            .collect();
        call_args.insert(0, ("ptr".into(), "null".into()));
        if *ret == Type::Unit {
            self.call_void(&symbol, &call_args);
            self.unit()
        } else {
            let llvm = self.llvm_ty(ret);
            let value = self.call_value(&llvm, &symbol, &call_args);
            Val::new(ret.clone(), value)
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
            LValue::TupleField(base, index) => match self.lvalue_ty(base) {
                Type::Tuple(items) => items.get(*index).cloned().unwrap_or(Type::Error),
                _ => Type::Error,
            },
            LValue::Index(base, _) => match self.lvalue_ty(base) {
                Type::List(inner) | Type::Array(inner, _) | Type::Slice(inner) => (*inner).clone(),
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
            LValue::TupleField(base, index) => {
                let base_addr = self.gen_lvalue_addr(base);
                let tuple_llvm = self.llvm_ty(&self.lvalue_ty(base));
                let pointer = self.fresh();
                self.emit(format!(
                    "{pointer} = getelementptr {tuple_llvm}, ptr {base_addr}, i32 0, i32 {index}"
                ));
                pointer
            }
            LValue::Index(base, index) => {
                let base_addr = self.gen_lvalue_addr(base);
                let base_ty = self.lvalue_ty(base);
                let (elem_ty, elem_llvm) = match &base_ty {
                    Type::List(inner) | Type::Array(inner, _) | Type::Slice(inner) => {
                        ((**inner).clone(), self.llvm_ty(inner))
                    }
                    _ => (Type::Error, "i8".into()),
                };
                let index_value = self.gen_expr(index);
                match base_ty {
                    Type::List(_) => {
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
                    Type::Array(_, len) => {
                        let array_llvm = self.llvm_ty(&base_ty);
                        self.bounds_check(&index_value.repr, &len.to_string(), Span::default());
                        let pointer = self.fresh();
                        self.emit(format!(
                            "{pointer} = getelementptr {array_llvm}, ptr {base_addr}, i64 0, i64 {}",
                            index_value.repr
                        ));
                        pointer
                    }
                    Type::Slice(_) => {
                        let slice = self.load("%nova.slice", &base_addr);
                        let data = self.extract_value("%nova.slice", &slice, "ptr", 0);
                        let len = self.extract_value("%nova.slice", &slice, "i64", 1);
                        self.bounds_check(&index_value.repr, &len, Span::default());
                        let pointer = self.fresh();
                        self.emit(format!(
                            "{pointer} = getelementptr {elem_llvm}, ptr {data}, i64 {}",
                            index_value.repr
                        ));
                        pointer
                    }
                    _ => "%t0".into(),
                }
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

const RUNTIME_DECLS: &str = r#"declare ptr @malloc(i64)
declare void @nova_print_str(%nova.string, i32, i32)
declare void @nova_print_i64(i64, i32, i32)
declare void @nova_print_f64(double, i32, i32)
declare void @nova_print_bool(i32, i32, i32)
declare void @nova_print_char(i32, i32, i32)
declare %nova.string @nova_string_concat(%nova.string, %nova.string)
declare i32 @nova_string_eq(%nova.string, %nova.string)
declare i32 @nova_string_contains(%nova.string, %nova.string)
declare i32 @nova_string_starts_with(%nova.string, %nova.string)
declare i32 @nova_string_ends_with(%nova.string, %nova.string)
declare %nova.string @nova_string_trim(%nova.string)
declare %nova.string @nova_i64_to_string(i64)
declare %nova.string @nova_f64_to_string(double)
declare %nova.string @nova_bool_to_string(i32)
declare %nova.string @nova_char_to_string(i32)
declare ptr @nova_list_at(ptr, i64, i64, i64)
declare void @nova_list_push(ptr, ptr, i64)
declare void @nova_map_insert(ptr, ptr)
declare i32 @nova_map_add(ptr, ptr)
declare ptr @nova_map_get(ptr, ptr)
declare i64 @nova_map_len(ptr)
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
    fn compiles_string_predicates_and_trim() {
        let source = r#"import std.io { println }

fn main() {
    let text = "  hello nova  "
    let trimmed = text.trim()
    println(trimmed.len())
    println(trimmed.contains("nova"))
    println(trimmed.starts_with("hello"))
    println(trimmed.ends_with("nova"))
    println(trimmed.contains("zzz"))
    println(text.contains(""))
}
"#;
        let program = typed(source);
        let output = temp_output("string-methods");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(stdout.trim_end(), "10\ntrue\ntrue\ntrue\nfalse\ntrue");
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
    fn compiles_user_enums_and_exhaustive_match() {
        let source = r#"import std.io { println }

enum Shape {
    Circle(Int)
    Square(Int)
    Empty
}

fn area(shape: Shape) -> Int {
    match shape {
        Circle(radius) => radius * radius * 3
        Square(side) => side * side
        Empty => 0
    }
}

fn main() {
    println(area(Circle(2)))
    println(area(Square(3)))
    println(area(Empty))
    let shapes = [Circle(1), Empty]
    for shape in shapes {
        println(area(shape))
    }
}
"#;
        let program = typed(source);
        let output = temp_output("enum");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "12\n9\n0\n3\n0\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_multi_payload_enums_and_impl_methods() {
        let source = r#"import std.io { println }

enum Shape {
    Rect(Int, Int)
    Circle(Int)
    Empty
}

impl Shape {
    fn area(self) -> Int {
        match self {
            Rect(width, height) => width * height
            Circle(radius) => radius * radius * 3
            Empty => 0
        }
    }

    fn label(self) -> String {
        match self {
            Rect(_, _) => "rect"
            Circle(_) => "circle"
            Empty => "empty"
        }
    }
}

fn main() {
    let rect = Rect(2, 3)
    let circle = Circle(2)
    println(rect.area())
    println(circle.area())
    println(rect.label())
    println(circle.label())
    println(Empty.area())
    match rect {
        Rect(width, height) => println(width + height)
        _ => println(0)
    }
}
"#;
        let program = typed(source);
        let output = temp_output("enum-payloads");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            "6\n12\nrect\ncircle\n0\n5\n"
        );
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_tuples_arrays_and_slices() {
        let source = r#"import std.io { println }

fn sum(values: Slice<Int>) -> Int {
    var index = 0
    var total = 0
    while index < values.len() {
        total += values[index]
        index += 1
    }
    return total
}

fn pair() -> (Int, String) {
    return (7, "seven")
}

fn main() {
    let item = (1, "two")
    println(item.0)
    println(item.1)
    let made = pair()
    println(made.0)
    println(made.1)
    match (3, 4) {
        (left, right) => println(left + right)
    }
    var numbers: [Int; 3] = [1, 2, 3]
    println(numbers.len())
    println(numbers[1])
    numbers[1] = 9
    println(numbers[1])
    let slice = numbers.as_slice()
    println(slice.len())
    println(slice[2])
    println(sum(slice))
    var point = (1, 2)
    point.0 = 5
    println(point.0)
}
"#;
        let program = typed(source);
        let output = temp_output("tuples-arrays");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            "1\ntwo\n7\nseven\n7\n3\n2\n9\n3\n3\n13\n5\n"
        );
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_closures_and_function_values() {
        let source = r#"import std.io { println }

fn add(left: Int, right: Int) -> Int {
    return left + right
}

fn apply(f: fn(Int) -> Int, value: Int) -> Int {
    return f(value)
}

fn main() {
    let increment = |x: Int| x + 1
    println(increment(2))
    let add_ref = add
    println(add_ref(2, 3))
    let double: fn(Int) -> Int = |x: Int| -> Int { x * 2 }
    println(double(4))
    println(apply(increment, 10))
    println(apply(double, 10))
}
"#;
        let program = typed(source);
        let output = temp_output("closures");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "3\n5\n8\n11\n20\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_capturing_closures() {
        let source = r#"import std.io { println }

fn make_adder(delta: Int) -> fn(Int) -> Int {
    return |x: Int| x + delta
}

fn apply(f: fn(Int) -> Int, value: Int) -> Int {
    return f(value)
}

fn main() {
    let base = 10
    let add_base = |x: Int| x + base
    println(add_base(7))
    let twice = |x: Int| x + base + base
    println(twice(1))
    let shadow = |base: Int| base + 1
    println(shadow(4))
    let add_five = make_adder(5)
    println(add_five(3))
    println(apply(add_five, 10))
    let add_ten = make_adder(10)
    println(add_ten(1))
    let outer = |a: Int| {
        let inner = |b: Int| a + b
        inner(2)
    }
    println(outer(5))
}
"#;
        let program = typed(source);
        let output = temp_output("capturing-closures");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            "17\n21\n5\n8\n15\n11\n7\n"
        );
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_maps_and_sets() {
        let source = r#"import std.io { println }

fn main() {
    var scores: Map<String, Int> = map()
    scores.insert("a", 1)
    scores.insert("b", 2)
    scores.insert("a", 9)
    println(scores.len())
    println(scores.contains_key("a"))
    println(scores.contains_key("z"))
    match scores.get("a") {
        Some(value) => println(value)
        None => println(0)
    }
    match scores.get("z") {
        Some(value) => println(value)
        None => println(-1)
    }

    var seen: Set<Int> = set()
    println(seen.insert(1))
    println(seen.insert(1))
    println(seen.contains(1))
    println(seen.contains(2))
    println(seen.len())

    var table: Map<Int, Int> = map()
    var i = 0
    while i < 100 {
        table.insert(i, i * 2)
        i += 1
    }
    println(table.len())
    match table.get(77) {
        Some(value) => println(value)
        None => println(-1)
    }
    println(table.contains_key(99))
    println(table.contains_key(100))

    var words: Set<String> = set()
    words.insert("alpha")
    words.insert("beta")
    words.insert("alpha")
    println(words.len())
    println(words.contains("beta"))
    println(words.contains("gamma"))
}
"#;
        let program = typed(source);
        let output = temp_output("maps-sets");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            "2\ntrue\nfalse\n9\n-1\ntrue\nfalse\ntrue\nfalse\n1\n100\n154\ntrue\nfalse\n2\ntrue\nfalse\n"
        );
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_generics_and_static_traits() {
        let source = r#"import std.io { println }

trait Describe {
    fn describe(self) -> String
}

struct User {
    name: String
}

impl Describe for User {
    fn describe(self) -> String {
        return self.name
    }
}

fn identity<T>(value: T) -> T {
    return value
}

fn first<A, B>(left: A, right: B) -> A {
    return left
}

fn main() {
    let user = User { name: "Ada" }
    println(user.describe())
    println(identity(7))
    println(identity("ok"))
    println(first(3, "ignored"))
}
"#;
        let program = typed(source);
        let output = temp_output("generics-traits");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "Ada\n7\nok\n3\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_generic_structs_enums_and_methods() {
        let source = r#"import std.io { println }

struct Pair<T> {
    first: T,
    second: T
}

impl<T> Pair<T> {
    fn sum(self) -> T {
        return self.first + self.second
    }
}

enum Boxed<T> {
    Full(T),
    Blank
}

fn main() {
    let pair: Pair<Int> = Pair { first: 2, second: 3 }
    println(pair.sum())
    let words: Pair<String> = Pair { first: "ab", second: "cd" }
    println(words.first)
    let value: Boxed<Int> = Full(9)
    match value {
        Full(inner) => println(inner)
        Blank => println(0)
    }
    let empty: Boxed<String> = Blank
    match empty {
        Full(text) => println(text)
        Blank => println("blank")
    }
}
"#;
        let program = typed(source);
        let output = temp_output("generic-types");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "5\nab\n9\nblank\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_trait_objects() {
        let source = r#"
trait Describe {
    fn describe(self) -> String;
    fn shout(self) -> String;
}

struct User {
    name: String,
}

impl Describe for User {
    fn describe(self) -> String {
        self.name
    }
    fn shout(self) -> String {
        "user"
    }
}

struct Robot {
    id: Int,
}

impl Describe for Robot {
    fn describe(self) -> String {
        "unit"
    }
    fn shout(self) -> String {
        "robot"
    }
}

fn show(item: dyn Describe) -> String {
    item.describe()
}

fn main() -> Unit {
    let user: dyn Describe = User { name: "Ada" };
    let robot: dyn Describe = Robot { id: 7 };
    println(show(user));
    println(show(robot));
    println(user.shout());
}
"#;
        let program = typed(source);
        let output = temp_output("trait-objects");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "Ada\nunit\nuser\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_async_ffi_and_macros() {
        let source = r#"import std.io { println }

extern "C" fn llabs(value: Int) -> Int

macro square(value) {
    value * value
}

async fn increment(value: Int) -> Int {
    return value + 1
}

async fn run() -> Int {
    let value = await increment(4)
    return await increment(value)
}

fn main() {
    println(llabs(-9))
    println(square(3 + 1))
    println(run())
}
"#;
        let program = typed(source);
        let ir = generate_ir(&program, source, Some("main")).expect("ir");
        assert!(ir.contains("nounwind"), "{ir}");
        let output = temp_output("async-ffi-macros");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "9\n16\n6\n");
        let _ = fs::remove_file(&artifact.executable);
    }

    #[test]
    fn compiles_statement_macros() {
        let source = r#"import std.io { println }

macro announce(value) {
    println("start")
    println(value)
}

macro twice(value) {
    announce(value)
    announce(value)
}

macro square(value) {
    value * value
}

fn main() {
    announce(4)
    println(square(3))
    twice(5)
}
"#;
        let program = typed(source);
        let output = temp_output("statement-macros");
        let artifact = compile(
            &program,
            source,
            &BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                ..Default::default()
            },
        )
        .expect("native compile");
        let run = Command::new(&artifact.executable).output().unwrap();
        assert!(
            run.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            "start\n4\n9\nstart\n5\nstart\n5\n"
        );
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
