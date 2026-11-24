//! `nova` command line driver.

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Parser, Subcommand};
use nova_codegen_llvm::{compile, BuildOptions, OptLevel};
use nova_diag::{render_all, Diagnostic};
use nova_parser::{parse, Item, Program};
use nova_pkg::{find_project_root, load, write_lock, Manifest, MANIFEST_NAME};
use nova_sema::{check, TypedProgram};

const EXIT_OK: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_UNSUPPORTED: i32 = 2;
const EXIT_INTERNAL: i32 = 70;

#[derive(Parser)]
#[command(
    name = "nova",
    version,
    about = "The Nova programming language toolchain",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Nova project
    New { name: String },
    /// Initialise a Nova project in the current directory
    Init,
    /// Compile and run the project binary
    Run {
        #[arg(long)]
        release: bool,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Compile the project binary
    Build {
        #[arg(long)]
        release: bool,
        #[arg(long)]
        emit_ir: bool,
        #[arg(long)]
        debug: bool,
        #[arg(long)]
        lto: bool,
        #[arg(long)]
        locked: bool,
    },
    /// Parse and type-check without producing an executable
    Check {
        #[arg(long)]
        json: bool,
    },
    /// Compile and run `tests/**/*.nova` test functions
    Test {
        #[arg(long)]
        filter: Option<String>,
    },
    /// Format Nova sources
    Fmt {
        #[arg(long)]
        check: bool,
        paths: Vec<PathBuf>,
    },
    /// Remove build artifacts for this project
    Clean,
    /// Diagnose the local toolchain
    Doctor,
    /// Start the language server on stdio
    Lsp,
    /// Start the compile-run REPL (no interpreter yet)
    Repl,
    /// Print the project dependency tree
    Tree,
    /// Add a dependency (not implemented)
    Add { package: String },
    /// Remove a dependency (not implemented)
    Remove { package: String },
    /// Update dependencies (not implemented)
    Update,
    /// Run benchmarks (not implemented)
    Bench,
    /// Run lints (not implemented)
    Lint,
    /// Generate documentation (not implemented)
    Doc,
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("nova: internal compiler error: {info}");
        eprintln!("nova: please report this with the input reduced if possible");
    }));
    let code = std::panic::catch_unwind(real_main).unwrap_or(EXIT_INTERNAL);
    std::process::exit(code);
}

fn real_main() -> i32 {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name } => cmd_new(&name),
        Commands::Init => cmd_init(),
        Commands::Run { release, args } => cmd_run(release, &args),
        Commands::Build {
            release,
            emit_ir,
            debug,
            lto,
            locked,
        } => cmd_build(release, emit_ir, debug, lto, locked),
        Commands::Check { json } => cmd_check(json),
        Commands::Test { filter } => cmd_test(filter.as_deref()),
        Commands::Fmt { check, paths } => cmd_fmt(check, &paths),
        Commands::Clean => cmd_clean(),
        Commands::Doctor => cmd_doctor(),
        Commands::Lsp => match nova_lsp::run_stdio() {
            Ok(()) => EXIT_OK,
            Err(error) => {
                eprintln!("nova lsp: {error}");
                EXIT_ERROR
            }
        },
        Commands::Repl => cmd_repl(),
        Commands::Tree => cmd_tree(),
        Commands::Add { package } => unsupported(&format!("nova add {package}")),
        Commands::Remove { package } => unsupported(&format!("nova remove {package}")),
        Commands::Update => unsupported("nova update"),
        Commands::Bench => unsupported("nova bench"),
        Commands::Lint => unsupported("nova lint"),
        Commands::Doc => unsupported("nova doc"),
    }
}

fn unsupported(command: &str) -> i32 {
    eprintln!(
        "{command}: not implemented in this milestone.\n\
         The command is listed in ROADMAP.md and is intentionally not advertised as working.\n\
         Implemented: new, init, run, build, check, test, fmt, clean, doctor, lsp, repl, tree."
    );
    EXIT_UNSUPPORTED
}

// ---------------------------------------------------------------------------
// Project helpers
// ---------------------------------------------------------------------------

struct Checked {
    source: String,
    program: TypedProgram,
}

fn project_root() -> Result<PathBuf, i32> {
    let cwd = std::env::current_dir().map_err(|e| {
        eprintln!("nova: could not read current directory: {e}");
        EXIT_ERROR
    })?;
    find_project_root(&cwd).ok_or_else(|| {
        eprintln!(
            "nova: no {MANIFEST_NAME} found in this directory or its parents.\n\
             Run `nova new <name>` or `nova init` first."
        );
        EXIT_ERROR
    })
}

fn analyze_file(path: &Path, json: bool) -> Result<Checked, i32> {
    let source = fs::read_to_string(path).map_err(|e| {
        eprintln!("nova: could not read {}: {e}", path.display());
        EXIT_ERROR
    })?;
    let parsed = parse(&source);
    let mut diagnostics: Vec<Diagnostic> = parsed.diagnostics;
    let mut stack = vec![path.to_path_buf()];
    let items = match expand_path_imports(path, parsed.program.items, &mut stack) {
        Ok(items) => items,
        Err(message) => {
            eprintln!("{message}");
            return Err(EXIT_ERROR);
        }
    };
    let program = Program {
        items,
        span: parsed.program.span,
    };
    let checked = check(&program);
    diagnostics.extend(checked.diagnostics);
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    if json {
        match serde_json::to_string_pretty(&diagnostics) {
            Ok(text) => println!("{text}"),
            Err(error) => eprintln!("nova: could not serialise diagnostics: {error}"),
        }
    } else if !diagnostics.is_empty() {
        eprint!(
            "{}",
            render_all(&path.display().to_string(), &source, &diagnostics)
        );
    }
    if errors > 0 {
        Err(EXIT_ERROR)
    } else {
        Ok(Checked {
            source,
            program: checked.program,
        })
    }
}

fn expand_path_imports(
    path: &Path,
    items: Vec<Item>,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<Item>, String> {
    let mut expanded = Vec::new();
    for item in items {
        match item {
            Item::Import(import) => match &import.path {
                Some(relative) => {
                    let child = path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(relative);
                    let canonical = fs::canonicalize(&child).map_err(|error| {
                        format!(
                            "nova: could not resolve module `{}` ({}): {error}",
                            relative,
                            child.display()
                        )
                    })?;
                    if stack.contains(&canonical) {
                        return Err(format!(
                            "nova: cyclic module import involving `{}`",
                            child.display()
                        ));
                    }
                    let child_source = fs::read_to_string(&canonical).map_err(|error| {
                        format!("nova: could not read {}: {error}", child.display())
                    })?;
                    let parsed = parse(&child_source);
                    let errors: Vec<&Diagnostic> = parsed
                        .diagnostics
                        .iter()
                        .filter(|diagnostic| diagnostic.is_error())
                        .collect();
                    if !errors.is_empty() {
                        return Err(render_all(
                            &child.display().to_string(),
                            &child_source,
                            &parsed.diagnostics,
                        ));
                    }
                    stack.push(canonical.clone());
                    let mut child_items =
                        expand_path_imports(&canonical, parsed.program.items, stack)?;
                    stack.pop();
                    expanded.append(&mut child_items);
                    if import.alias.is_some() {
                        expanded.push(Item::Import(import.clone()));
                    }
                }
                None => expanded.push(Item::Import(import)),
            },
            other => expanded.push(other),
        }
    }
    Ok(expanded)
}

fn analyze_source(source: &str, path: &Path, json: bool) -> Result<Checked, i32> {
    let parsed = parse(source);
    let checked = check(&parsed.program);
    let mut diagnostics: Vec<Diagnostic> = parsed.diagnostics;
    diagnostics.extend(checked.diagnostics);
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    if json {
        match serde_json::to_string_pretty(&diagnostics) {
            Ok(text) => println!("{text}"),
            Err(error) => eprintln!("nova: could not serialise diagnostics: {error}"),
        }
    } else if !diagnostics.is_empty() {
        eprint!(
            "{}",
            render_all(&path.display().to_string(), source, &diagnostics)
        );
    }
    if errors > 0 {
        Err(EXIT_ERROR)
    } else {
        Ok(Checked {
            source: source.to_string(),
            program: checked.program,
        })
    }
}

fn build_options(
    root: &Path,
    manifest: &Manifest,
    release: bool,
    emit_ir: bool,
    debug: bool,
    lto: bool,
) -> BuildOptions {
    let target = root.join("target/nova");
    let output = target.join(&manifest.name);
    BuildOptions {
        output: output.clone(),
        opt_level: if release { OptLevel::O3 } else { OptLevel::O2 },
        emit_ir: if emit_ir {
            Some(output.with_extension("ll"))
        } else {
            None
        },
        debug,
        lto,
        entry: None,
        target: None,
    }
}

fn build_project(
    root: &Path,
    manifest: &Manifest,
    release: bool,
    emit_ir: bool,
    debug: bool,
    lto: bool,
    locked: bool,
) -> Result<PathBuf, i32> {
    if locked && !root.join(nova_pkg::LOCK_NAME).is_file() {
        eprintln!("nova build: --locked was passed but nova.lock does not exist");
        return Err(EXIT_ERROR);
    }
    if !manifest.dependencies.is_empty() {
        eprintln!(
            "nova build: this project declares dependencies, but package resolution is not implemented yet.\n\
             Remove `[dependencies]` or use a path-independent project. See ROADMAP.md."
        );
        return Err(EXIT_UNSUPPORTED);
    }
    let source_path = root.join(&manifest.bin_path);
    if !source_path.is_file() {
        eprintln!(
            "nova build: binary source `{}` does not exist",
            source_path.display()
        );
        return Err(EXIT_ERROR);
    }
    let checked = analyze_file(&source_path, false)?;
    let options = build_options(root, manifest, release, emit_ir, debug, lto);
    match compile(&checked.program, &checked.source, &options) {
        Ok(artifact) => {
            if let Err(error) = write_lock(root, manifest) {
                eprintln!("nova: warning: could not write nova.lock: {error}");
            }
            println!("Built {}", artifact.executable.display());
            if let Some(ir) = artifact.ir {
                println!("LLVM IR: {}", ir.display());
            }
            Ok(artifact.executable)
        }
        Err(error) => {
            eprintln!("nova: native build failed: {error}");
            Err(EXIT_ERROR)
        }
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn template(name: &str) -> (String, String, String) {
    let manifest = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[bin]\npath = \"src/main.nova\"\n"
    );
    let main = "import std.io { println }\n\nfn main() {\n    println(\"Hello, Nova!\")\n}\n";
    let test =
        "import std.core { assert }\n\n@test\nfn arithmetic_works() {\n    assert(1 + 1 == 2)\n}\n";
    (manifest, main.to_string(), test.to_string())
}

fn write_project(root: &Path, name: &str) -> Result<(), i32> {
    fs::create_dir_all(root.join("src")).map_err(|e| {
        eprintln!("nova: could not create project: {e}");
        EXIT_ERROR
    })?;
    fs::create_dir_all(root.join("tests")).map_err(|e| {
        eprintln!("nova: could not create tests directory: {e}");
        EXIT_ERROR
    })?;
    let (manifest, main, test) = template(name);
    fs::write(root.join(MANIFEST_NAME), manifest).map_err(|e| {
        eprintln!("nova: could not write manifest: {e}");
        EXIT_ERROR
    })?;
    fs::write(root.join("src/main.nova"), main).map_err(|e| {
        eprintln!("nova: could not write source: {e}");
        EXIT_ERROR
    })?;
    fs::write(root.join("tests/hello_test.nova"), test).map_err(|e| {
        eprintln!("nova: could not write test: {e}");
        EXIT_ERROR
    })?;
    fs::write(root.join(".gitignore"), "/target\n").map_err(|e| {
        eprintln!("nova: could not write .gitignore: {e}");
        EXIT_ERROR
    })?;
    Ok(())
}

fn cmd_new(name: &str) -> i32 {
    if let Err(error) = nova_pkg::validate_name(name) {
        eprintln!("nova new: {error}");
        return EXIT_ERROR;
    }
    let root = PathBuf::from(name);
    if root.exists() {
        eprintln!("nova new: `{}` already exists", root.display());
        return EXIT_ERROR;
    }
    if let Err(code) = write_project(&root, name) {
        return code;
    }
    println!("Created Nova project `{name}`.");
    println!("Next:");
    println!("  cd {name}");
    println!("  nova run");
    EXIT_OK
}

fn cmd_init() -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("nova init: {error}");
            return EXIT_ERROR;
        }
    };
    if cwd.join(MANIFEST_NAME).exists() {
        eprintln!("nova init: {MANIFEST_NAME} already exists here");
        return EXIT_ERROR;
    }
    let name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let name = if name.is_empty() {
        "app".to_string()
    } else {
        name
    };
    if let Err(code) = write_project(&cwd, &name) {
        return code;
    }
    println!("Initialised Nova project `{name}`.");
    EXIT_OK
}

fn cmd_build(release: bool, emit_ir: bool, debug: bool, lto: bool, locked: bool) -> i32 {
    let root = match project_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let manifest = match load(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("nova build: {error}");
            return EXIT_ERROR;
        }
    };
    match build_project(&root, &manifest, release, emit_ir, debug, lto, locked) {
        Ok(_) => EXIT_OK,
        Err(code) => code,
    }
}

fn cmd_run(release: bool, args: &[String]) -> i32 {
    let root = match project_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let manifest = match load(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("nova run: {error}");
            return EXIT_ERROR;
        }
    };
    let executable = match build_project(&root, &manifest, release, false, false, false, false) {
        Ok(path) => path,
        Err(code) => return code,
    };
    if !args.is_empty() {
        eprintln!(
            "nova run: note: program arguments are accepted by the CLI but `main` cannot read them yet"
        );
    }
    match Command::new(&executable)
        .args(args)
        .current_dir(&root)
        .status()
    {
        Ok(status) => status.code().unwrap_or(EXIT_ERROR),
        Err(error) => {
            eprintln!(
                "nova run: could not start {}: {error}",
                executable.display()
            );
            EXIT_ERROR
        }
    }
}

fn cmd_check(json: bool) -> i32 {
    let root = match project_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let manifest = match load(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("nova check: {error}");
            return EXIT_ERROR;
        }
    };
    let source_path = root.join(&manifest.bin_path);
    match analyze_file(&source_path, json) {
        Ok(_) => {
            if !json {
                println!("Checked {} — no errors.", source_path.display());
            }
            EXIT_OK
        }
        Err(code) => code,
    }
}

fn collect_nova(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_nova(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("nova") {
            out.push(path);
        }
    }
}

fn cmd_test(filter: Option<&str>) -> i32 {
    let root = match project_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let manifest = match load(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("nova test: {error}");
            return EXIT_ERROR;
        }
    };
    let tests_dir = root.join("tests");
    if !tests_dir.is_dir() {
        eprintln!("nova test: no tests/ directory");
        return EXIT_ERROR;
    }
    let mut files = Vec::new();
    collect_nova(&tests_dir, &mut files);
    files.sort();
    if files.is_empty() {
        println!("No test files found under tests/.");
        return EXIT_OK;
    }
    let mut passed = 0usize;
    let mut failed = 0usize;
    for file in files {
        let checked = match analyze_file(&file, false) {
            Ok(checked) => checked,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        if checked.program.tests.is_empty() {
            println!(
                "note: {} has no @test functions (skipped)",
                file.strip_prefix(&root).unwrap_or(&file).display()
            );
            continue;
        }
        for test_name in checked.program.tests.clone() {
            if let Some(filter) = filter {
                if !test_name.contains(filter) {
                    continue;
                }
            }
            let output = root.join("target/nova/tests").join(format!(
                "{}_{}",
                file.file_stem().and_then(|s| s.to_str()).unwrap_or("test"),
                test_name
            ));
            let options = BuildOptions {
                output: output.clone(),
                opt_level: OptLevel::O2,
                emit_ir: None,
                debug: false,
                lto: false,
                entry: Some(test_name.clone()),
                target: None,
            };
            let shown = file.strip_prefix(&root).unwrap_or(&file).display();
            match compile(&checked.program, &checked.source, &options) {
                Ok(artifact) => match Command::new(&artifact.executable).status() {
                    Ok(status) if status.success() => {
                        println!("test {shown}::{test_name} ... ok");
                        passed += 1;
                    }
                    Ok(status) => {
                        println!(
                            "test {shown}::{test_name} ... FAILED (exit {})",
                            status.code().unwrap_or(-1)
                        );
                        failed += 1;
                    }
                    Err(error) => {
                        println!("test {shown}::{test_name} ... FAILED ({error})");
                        failed += 1;
                    }
                },
                Err(error) => {
                    println!("test {shown}::{test_name} ... FAILED (build)");
                    eprintln!("{error}");
                    failed += 1;
                }
            }
        }
    }
    println!("\n{} passed, {} failed", passed, failed);
    if failed == 0 && passed > 0 {
        EXIT_OK
    } else if failed == 0 {
        println!("note: filter matched no tests");
        EXIT_OK
    } else {
        EXIT_ERROR
    }
}

fn cmd_fmt(check: bool, paths: &[PathBuf]) -> i32 {
    let mut files = Vec::new();
    if paths.is_empty() {
        let root = match project_root() {
            Ok(root) => root,
            Err(code) => return code,
        };
        collect_nova(&root.join("src"), &mut files);
        collect_nova(&root.join("tests"), &mut files);
    } else {
        for path in paths {
            if path.is_dir() {
                collect_nova(path, &mut files);
            } else {
                files.push(path.clone());
            }
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!("nova fmt: no .nova files found");
        return EXIT_ERROR;
    }
    let mut changed = Vec::new();
    let mut errors = 0usize;
    for path in &files {
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("nova fmt: could not read {}: {error}", path.display());
                errors += 1;
                continue;
            }
        };
        let formatted = nova_fmt::format_source(&source);
        if formatted == source {
            continue;
        }
        changed.push(path.clone());
        if !check {
            let temporary = path.with_file_name(format!(
                ".{}.nova-tmp",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("source")
            ));
            if let Err(error) =
                fs::write(&temporary, &formatted).and_then(|_| fs::rename(&temporary, path))
            {
                eprintln!("nova fmt: could not write {}: {error}", path.display());
                errors += 1;
            }
        }
    }
    if check {
        for path in &changed {
            println!("would reformat {}", path.display());
        }
        if changed.is_empty() && errors == 0 {
            println!("All {} file(s) are formatted.", files.len());
        }
    } else {
        println!("Formatted {} file(s).", changed.len());
    }
    if errors > 0 {
        EXIT_ERROR
    } else if check && !changed.is_empty() {
        EXIT_ERROR
    } else {
        EXIT_OK
    }
}

fn cmd_clean() -> i32 {
    let root = match project_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let target = root.join("target/nova");
    if target.exists() {
        match fs::remove_dir_all(&target) {
            Ok(()) => println!("Removed {}", target.display()),
            Err(error) => {
                eprintln!("nova clean: {error}");
                return EXIT_ERROR;
            }
        }
    } else {
        println!("Nothing to clean.");
    }
    EXIT_OK
}

fn first_line(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    text.lines().next().map(|line| line.trim().to_string())
}

fn cmd_doctor() -> i32 {
    println!("Nova {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Executable: {}",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "<unknown>".into())
    );
    println!("OS: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!("Runtime ABI: {}", nova_runtime::ABI_VERSION);

    let mut ok = true;
    match first_line("clang", &["--version"]) {
        Some(version) => println!("C toolchain: {version}"),
        None => {
            println!("C toolchain: NOT FOUND (required for native builds)");
            ok = false;
        }
    }
    let llvm = ["llvm-config", "/opt/homebrew/opt/llvm/bin/llvm-config"]
        .iter()
        .find_map(|candidate| first_line(candidate, &["--version"]));
    match llvm {
        Some(version) => println!("LLVM: {version} (optional; clang drives IR)"),
        None => println!("LLVM: llvm-config not found (optional)"),
    }
    match first_line("node", &["--version"]) {
        Some(version) => println!("Node.js: {version} (IDE)"),
        None => println!("Node.js: not found (Nova Studio requires it)"),
    }
    match first_line("freebuff", &["--version"]) {
        Some(version) => println!(
            "FreeBuff: {version} (interactive terminal integration only; no structured chat protocol is documented)"
        ),
        None => println!("FreeBuff: not found (Nova Studio shows an unavailable state)"),
    }
    println!("Language server: `nova lsp`");
    println!("Formatter: `nova fmt --check`");
    println!(
        "FreeBuff integration: dedicated terminal only. The installed CLI exposes `--help`, `--version`, `--continue`, `--cwd` and `--trust-agents`; it has no documented JSON/headless chat protocol."
    );
    if ok {
        println!("\nDoctor: native builds should work.");
        EXIT_OK
    } else {
        println!("\nDoctor: install Xcode command line tools or clang, or set NOVA_CC.");
        EXIT_ERROR
    }
}

fn cmd_tree() -> i32 {
    let root = match project_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let manifest = match load(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("nova tree: {error}");
            return EXIT_ERROR;
        }
    };
    println!("{} v{}", manifest.name, manifest.version);
    print_dependencies(&root, &manifest, 1, &mut Vec::new());
    if manifest.dependencies.is_empty() {
        println!("  (no dependencies)");
    }
    EXIT_OK
}

fn print_dependencies(root: &Path, manifest: &Manifest, depth: usize, seen: &mut Vec<PathBuf>) {
    for dependency in &manifest.dependencies {
        let indent = "  ".repeat(depth);
        if let Some(path) = &dependency.path {
            let path = root.join(path);
            println!("{indent}{} (path: {})", dependency.name, path.display());
            if seen.contains(&path) {
                println!("{indent}  (cycle detected)");
                continue;
            }
            seen.push(path.clone());
            if let Ok(inner) = load(&path) {
                print_dependencies(&path, &inner, depth + 1, seen);
            } else {
                println!("{indent}  (manifest missing or unreadable)");
            }
        } else {
            println!(
                "{indent}{} {} (registry resolution not implemented)",
                dependency.name,
                dependency.version.as_deref().unwrap_or("*")
            );
        }
    }
}

fn cmd_repl() -> i32 {
    println!("Nova compile-run REPL. Each line is compiled natively and executed.");
    println!("The reference interpreter is not implemented yet; previous lines are replayed.");
    println!("Use :quit to exit, :clear to reset.");
    let mut buffer = String::new();
    let mut counter = 0usize;
    let stdin = io::stdin();
    loop {
        print!("nova> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("nova repl: {error}");
                return EXIT_ERROR;
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if matches!(trimmed, ":quit" | ":q" | "exit") {
            break;
        }
        if trimmed == ":clear" {
            buffer.clear();
            continue;
        }
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("import ")
        {
            println!("Declarations are not supported in the REPL yet; put them in a .nova file.");
            continue;
        }
        let candidate = if buffer.is_empty() {
            format!("{trimmed}\n")
        } else {
            format!("{buffer}{trimmed}\n")
        };
        let program_source = format!("fn main() {{\n{candidate}}}\n");
        let mut parsed = parse(&program_source);
        let mut statement = trimmed.to_string();
        if parsed.diagnostics.iter().any(|d| d.is_error()) && !trimmed.ends_with('}') {
            let wrapped = if buffer.is_empty() {
                format!("println({trimmed})\n")
            } else {
                format!("{buffer}println({trimmed})\n")
            };
            let wrapped_source = format!("fn main() {{\n{wrapped}}}\n");
            let alternative = parse(&wrapped_source);
            if !alternative.diagnostics.iter().any(|d| d.is_error()) {
                parsed = alternative;
                statement = format!("println({trimmed})");
            }
        }
        let checked = check(&parsed.program);
        let mut diagnostics = parsed.diagnostics.clone();
        diagnostics.extend(checked.diagnostics.clone());
        if diagnostics.iter().any(|d| d.is_error()) {
            eprint!("{}", render_all("<repl>", &program_source, &diagnostics));
            continue;
        }
        counter += 1;
        let output =
            std::env::temp_dir().join(format!("nova-repl-{}-{counter}", std::process::id()));
        let options = BuildOptions {
            output: output.clone(),
            opt_level: OptLevel::O0,
            emit_ir: None,
            debug: false,
            lto: false,
            entry: None,
            target: None,
        };
        match compile(&checked.program, &program_source, &options) {
            Ok(artifact) => {
                let status = Command::new(&artifact.executable).status();
                let _ = fs::remove_file(&artifact.executable);
                match status {
                    Ok(status) if status.success() => {
                        if buffer.is_empty() {
                            buffer = format!("{statement}\n");
                        } else {
                            buffer.push_str(&format!("{statement}\n"));
                        }
                    }
                    Ok(status) => println!("process exited with {}", status.code().unwrap_or(-1)),
                    Err(error) => println!("could not run snippet: {error}"),
                }
            }
            Err(error) => {
                eprintln!("{error}");
            }
        }
    }
    EXIT_OK
}
