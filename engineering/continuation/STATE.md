# Current Nova checkpoint

Updated: 2026-11-25
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: this checkpoint is committed by the accompanying code/doc commits; prior
`7518333`.
Working tree: unrelated `editor/` and playground changes remain untouched.

Active milestone: `BOOT-02` / `CODEGEN-01` — self-compilation and LLVM lowering
Active task: `BOOT-02.01`–`BOOT-02.03` — generation 1/2/3 of the Nova tool
Status: PARTIAL — the Nova tool builds itself through three generations and
compiles a native sample; full compiler/LSP parity, module/Box semantics and
Rust seed removal remain.

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 221, failed 0.
  - `List<Int>` literals allocate with libc `malloc`; empty lists supported.
  - List parameters and returns use by-value `%nova.list`; a literal survives
    return, identity and indexing (native 13).
  - Mutable local `.push(x)` calls `nova_list_push(list, &x, 8)`, which grows
    with `realloc` and copies the element; a grown list survives return and
    indexing (native 16).
  - `points[i]` and `points[i] = v` call `nova_list_at`, which performs the seed
    bounds check/panic and returns the element address; list/String `.len()` use
    `extractvalue` index 1. A mutated list survives return (native 45).
  - A 5,000-element push loop then indexes the final element; native 10.
  - `for n in start..end` is an exclusive integer range; `for x in list` walks a
    `List<Int>` by index. `break` and `continue` use a nested loop-label stack;
    `continue` in a `for` jumps to the increment block. Native results: range 8,
    list 30, nested 30.
  - `match` supports Int/Bool/enum scrutinees, integer and `true`/`false`
    literal patterns, `Variant`, `Variant(_)` and `Variant(binding)` patterns,
    `_`, expression arms and Unit block arms. Expression arms merge through a
    `phi`; an unmatched or empty match calls `nova_panic_str` and exits 1.
    Native results: Int match 60, Bool/Unit match 9, enum 36, String match 13,
    panic exit 1.
  - User structs are emitted as `%nova.struct.<Name>` aggregates with nested
    value fields. Literals use `insertvalue`, field reads use `extractvalue`,
    mutable local field assignment rebuilds and stores the aggregate, and
    struct parameters/returns pass by value. Native struct result: 14.
  - Enums are emitted as `%nova.enum.<Name>` aggregates with an `i32` tag and
    one `i64` slot per maximum payload arity. Bare/call construction, payload
    binding in match arms, wildcard arms and by-value parameters/returns are
    verified. Native enum result: 36.
  - Native results: assignment 45; push 16; push loop 10; list index+length
    23; list ABI 13; Int 9/2/17; Unit caller 4; Unit `main` 0; string 5.
  - `print`/`println` lower to `nova_print_i64`, `nova_print_bool` or
    `nova_print_str`; exact stdout and exit code are verified. `panic(String)`
    calls `nova_panic_str` and terminates. `assert(Bool)` branches to
    `nova_assert_fail`; passing and failing exits are verified.
  - String `trim`/`trim_start`/`trim_end` and `contains`/`starts_with`/
    `ends_with` call the runtime; a trimmed search program returns 5.
    `replace(from, to)` and `lower()` also call the runtime (native 16).
    `is_empty()` compares the length and `bytes()` fills a `List<Int>` (native 95).
    `upper()` is backed by `nova_string_upper` (native 72).
  - Enum payloads support Int, Bool and String. Bool is zero-extended/truncated
    in one slot; String occupies pointer/length slots with ptrtoint/inttoptr and
    extractvalue/insertvalue packing. A Text/Label fixture returns 10. Nested
    match expressions merge through per-arm completion blocks.
  - `List<Int> ==`/`!=` compare lengths and elements through `nova_list_at`;
    empty, equal and unequal lists are verified natively (12). `List + List`
    concatenates through `nova_list_at`/`nova_list_push` (11). Slices
    `values[start..end]` build a new list through the same runtime (10);
    `first()`/`last()` read through `nova_list_at` (native 49), and
    `contains(value)` scans with the same runtime check (native 16), and
    `reverse()` copies through the runtime (native 31), and `sort()` uses a
    native int64 comparator (native 13), and `sum()` accumulates values (native 10).
    `min()`/`max()` scan through the same runtime (native 29), and `index_of`
    returns the first index or -1 (native 11). Enum payloads may now be structs;
    recursive slot packing/unpacking handles nested struct fields (native 9).
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
  - Generic lists carry element metadata; `List<String>` and `List<Struct>`
    literals, indexing, assignment, `push`, `for` and `.len()` use the actual
    element size. Native `List<String>` slice returns 3.
  - Tuple types lower to LLVM literal structs; tuple returns and `.0`/`.1`
    extraction work (native 5).
  - `extern "C"` declarations emit `declare` lines and calls go directly to
    the host symbol; `nova_exit` exits 7 natively.
  - `&&`/`||` short-circuit through branch blocks and `!` lowers to `xor i1`;
    native logical result 3.
  - `String.chars`, `split`, `List<String>.join` and
    `Int`/`Bool`/`String.to_string` lower to runtime calls; native results 3,
    8 and 7.
  - Expression-valued `if`/`else` lowers through branch blocks and a merge
    value; native `if` fixture returns 13.
  - Module-qualified calls such as `gui.read(...)` and `math.double(...)` fall
    back to the flattened global function when the receiver is not a local;
    the CLI project uses an aliased import and exits 10.
  - `%` lowers to `srem`; the builtin fixture exercises it through `json`
    decoding and returns 5.
  - `char_string(Int)` and `json_string(String, String)` lower to
    `nova_char_to_string` and `nova_json_string`; the builtin fixture returns 5.
  - `http_get(String)` lowers to `nova_http_get`; the compile-only fixture
    checks the declaration and link without a network request.
  - The Nova driver detects `nova_gui_`/`nova_fs_`/`nova_process_`/`nova_env`
    symbols, compiles `runtime/gui.m` with `-fobjc-arc` on Darwin and links
    Cocoa/CoreText/CoreGraphics/ImageIO/QuartzCore. This is what lets the
    self-built tool link on macOS.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 28, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- `sh examples/nova-tool/tests/cli.sh` → passed 11, failed 0.
- Self-build, generation 1:
  `./target/nova/nova-tool build` with `NOVA_TOOL_OUT=/tmp/nova-self`
  produced a native `/tmp/nova-self`; it built
  `tests/codegen-sample.nova` and the program exited 9.
- Self-build, generation 2: `/tmp/nova-self build` produced
  `/tmp/nova-self-2`; it built the sample and the program exited 9.
- Self-build, generation 3: `/tmp/nova-self-2 build` produced
  `/tmp/nova-self-3`; it built the sample and the program exited 9.
- The three binary SHA-256 values differ, so this is behavioral bootstrap
  evidence, not a byte-identical fixed-point claim.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- Qualified module calls after import flattening; sema and codegen agree.
- Expression-valued `if`/`else` lowering with line/kind diagnostics.
- Modulo lowering and native builtin support for `char_string`, `json_string`
  and `http_get`.
- GUI/fs/process host linking in the Nova driver, mirroring the Rust bootstrap.
- Mutable list bindings and list reassignment clone their backing storage via
  `nova_list_clone`; `cg_struct_cycle` builds a fresh `List<String>` before
  recursion instead of pushing through a shallow list-header copy.
- A regression fixture pushes through two copies of the same list and returns 7.
- New native fixtures: `codegen-if-expr.nova`, `codegen-builtins.nova`,
  `codegen-http-get.nova`; the CLI project now uses an aliased qualified call.
- Full codegen/CLI/parser/sema/sema-ast suites remain green at 221/11/50/28/4.

## Incomplete work / known failures

- `nova build` still runs the Rust CLI for stage 0; no Rust crate has been
  deleted. Do not claim “pure Nova” before `BOOT-02.04` and LSP parity.
- `BOOT-02.04` (build the Nova LSP with the self-hosted compiler) and
  `BOOT-02.05` (Studio uses it) remain TODO.
- Mutable list initialization and reassignment clone the backing array, but
  function arguments and `let` bindings still pass headers by value; nested
  list ownership and GC are unresolved.
- List literals allocate with libc `malloc`; storage is never freed and
  ownership/GC is unresolved.
- Box dependencies, standard-library module semantics and full module/import
  resolution remain.
- Sema lacks generics, traits, methods and full diagnostics; the Nova LSP does
  not exist.
- Overflow, division-by-zero and alignment checks remain.
- Match lacks nested enum/struct patterns and enum-typed payloads; maps, sets
  and arrays remain.
- No Rust/Nova differential harness or measured performance claim.

## Resume here

1. `BOOT-02.04`: build the Nova LSP with the self-hosted compiler, then
   `BOOT-02.05`.
2. Add Box dependency and std-module resolution to the Nova CLI.
3. Replace the remaining Rust stage-0 path only after full sema/codegen parity
   and a reproducible bootstrap comparison.
4. Keep the Rust seed and do not delete `bootstrap/` until those gates pass.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (221/0)
- Command: `sh examples/nova-tool/tests/cli.sh`
- Result: PASS (11/0)
- Command: `sh examples/nova-tool/tests/parser.sh`
- Result: PASS (50/0)
- Command: `sh examples/nova-tool/tests/sema-ast.sh`
- Result: PASS (28/0)
- Command: `sh examples/nova-tool/tests/sema.sh`
- Result: PASS (4/0)
- Command: stage 1/2/3 self-build + sample execution
- Result: PASS (all three binaries exited 9; hashes differ)

## Blockers and approvals

- No external blocker for the remaining self-hosting work.
- Push, publish, deployment and branch deletion require owner authorization.
- List ownership/copy semantics are a known correctness limit; do not claim full
  runtime parity.
- The LSP needs full sema, stdio and the parity slices above.
