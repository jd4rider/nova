//! Nova native runtime.
//!
//! The runtime is small C code compiled with the system C toolchain and linked
//! into every native executable. It provides the process entry glue, strings,
//! dynamic lists, printing, checked-panic reporting and the UTF-8 char helpers.
//!
//! ## ABI
//!
//! * `String` is `{ const char *ptr; int64_t len; }` (UTF-8 bytes, not
//!   necessarily NUL-terminated for substrings; literals are NUL-terminated).
//! * `List<T>` is `{ void *data; int64_t len; int64_t cap; }` with an
//!   element stride supplied by generated code.
//! * `Option<T>` is `{ i1 tag; T payload; }`, `Result<T, E>` is
//!   `{ i1 tag; T ok; E err; }` with tag 1 meaning `Some`/`Ok`.
//! * All integers are two's-complement `i64`, floats are IEEE-754 `f64`,
//!   `Char` is a Unicode scalar stored in `i32`.
//!
//! ## Memory
//!
//! This milestone uses process-lifetime allocation for `String` and `List`
//! data. Memory is not reclaimed during execution; it is released by the OS at
//! exit. The allocator is safe (no use-after-free) but a long-running process
//! that builds many strings will grow. Reference counting and deterministic
//! destruction are explicitly not yet implemented; see `ROADMAP.md`.

/// C runtime source, embedded in the compiler binary so a checked-in/runtime
/// copy cannot drift from the compiler that emits calls into it.
pub const RUNTIME_C: &str = include_str!("runtime.c");

/// Version of the runtime ABI. Bump when a generated program's expectations
/// change; the CLI records this in `nova doctor` output.
pub const ABI_VERSION: u32 = 1;

/// Runtime symbols emitted by codegen and provided by `runtime.c`.
pub const RUNTIME_SYMBOLS: &[&str] = &[
    "nova_string_contains",
    "nova_string_starts_with",
    "nova_string_ends_with",
    "nova_string_trim",
    "nova_print_str",
    "nova_print_i64",
    "nova_print_f64",
    "nova_print_bool",
    "nova_print_char",
    "nova_string_concat",
    "nova_string_eq",
    "nova_i64_to_string",
    "nova_f64_to_string",
    "nova_bool_to_string",
    "nova_char_to_string",
    "nova_list_at",
    "nova_list_push",
    "nova_panic_str",
    "nova_assert_fail",
    "nova_assert_eq_fail",
    "nova_panic_overflow",
    "nova_panic_divzero",
    "nova_panic_neg_overflow",
];
