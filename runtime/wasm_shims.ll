; Runtime ABI shims.
;
; Nova's generated IR passes %nova.string by value, while C compiled for
; wasm32 passes aggregates indirectly. These wrappers store the incoming
; structs, call the pointer-based C implementation and return the result, so
; both sides agree at the wasm level.
;
; The %nova.string type and the runtime declarations come from the program IR
; this text is appended to.

declare void @nova_print_str_impl(ptr, i32, i32)
declare void @nova_string_concat_impl(ptr, ptr, ptr)
declare i32 @nova_string_eq_impl(ptr, ptr)
declare void @nova_i64_to_string_impl(ptr, i64)

define void @nova_print_str(%nova.string %s, i32 %newline, i32 %stderr) {
entry:
  %slot = alloca %nova.string
  store %nova.string %s, ptr %slot
  call void @nova_print_str_impl(ptr %slot, i32 %newline, i32 %stderr)
  ret void
}

define %nova.string @nova_string_concat(%nova.string %a, %nova.string %b) {
entry:
  %out = alloca %nova.string
  %pa = alloca %nova.string
  %pb = alloca %nova.string
  store %nova.string %a, ptr %pa
  store %nova.string %b, ptr %pb
  call void @nova_string_concat_impl(ptr %out, ptr %pa, ptr %pb)
  %result = load %nova.string, ptr %out
  ret %nova.string %result
}

define %nova.string @nova_i64_to_string(i64 %value) {
entry:
  %out = alloca %nova.string
  call void @nova_i64_to_string_impl(ptr %out, i64 %value)
  %result = load %nova.string, ptr %out
  ret %nova.string %result
}

define i32 @nova_string_eq(%nova.string %a, %nova.string %b) {
entry:
  %pa = alloca %nova.string
  %pb = alloca %nova.string
  store %nova.string %a, ptr %pa
  store %nova.string %b, ptr %pb
  %result = call i32 @nova_string_eq_impl(ptr %pa, ptr %pb)
  ret i32 %result
}

declare void @nova_string_chars_impl(ptr, ptr)
declare void @nova_string_split_impl(ptr, ptr, ptr)
declare void @nova_string_join_impl(ptr, ptr, ptr)
declare void @nova_string_trim_impl(ptr, ptr)
declare void @nova_string_trim_start_impl(ptr, ptr)
declare void @nova_string_trim_end_impl(ptr, ptr)

define void @nova_string_chars(%nova.string %s, ptr %list) {
entry:
  %slot = alloca %nova.string
  store %nova.string %s, ptr %slot
  call void @nova_string_chars_impl(ptr %slot, ptr %list)
  ret void
}

define void @nova_string_split(%nova.string %s, %nova.string %sep, ptr %list) {
entry:
  %pa = alloca %nova.string
  %pb = alloca %nova.string
  store %nova.string %s, ptr %pa
  store %nova.string %sep, ptr %pb
  call void @nova_string_split_impl(ptr %pa, ptr %pb, ptr %list)
  ret void
}

define %nova.string @nova_string_join(ptr %list, %nova.string %sep) {
entry:
  %out = alloca %nova.string
  %slot = alloca %nova.string
  store %nova.string %sep, ptr %slot
  call void @nova_string_join_impl(ptr %out, ptr %list, ptr %slot)
  %result = load %nova.string, ptr %out
  ret %nova.string %result
}

define %nova.string @nova_string_trim(%nova.string %s) {
entry:
  %out = alloca %nova.string
  %slot = alloca %nova.string
  store %nova.string %s, ptr %slot
  call void @nova_string_trim_impl(ptr %out, ptr %slot)
  %result = load %nova.string, ptr %out
  ret %nova.string %result
}

define %nova.string @nova_string_trim_start(%nova.string %s) {
entry:
  %out = alloca %nova.string
  %slot = alloca %nova.string
  store %nova.string %s, ptr %slot
  call void @nova_string_trim_start_impl(ptr %out, ptr %slot)
  %result = load %nova.string, ptr %out
  ret %nova.string %result
}

define %nova.string @nova_string_trim_end(%nova.string %s) {
entry:
  %out = alloca %nova.string
  %slot = alloca %nova.string
  store %nova.string %s, ptr %slot
  call void @nova_string_trim_end_impl(ptr %out, ptr %slot)
  %result = load %nova.string, ptr %out
  ret %nova.string %result
}

declare void @nova_input_str_impl(ptr)

define %nova.string @nova_input_str() {
entry:
  %out = alloca %nova.string
  call void @nova_input_str_impl(ptr %out)
  %result = load %nova.string, ptr %out
  ret %nova.string %result
}

declare void @nova_output_write_impl(ptr)

define void @nova_output_write(%nova.string %s) {
entry:
  %slot = alloca %nova.string
  store %nova.string %s, ptr %slot
  call void @nova_output_write_impl(ptr %slot)
  ret void
}
