/*
 * Nova native runtime (MVP).
 *
 * Memory model: strings and lists are allocated for the lifetime of the
 * process. This is safe but does not reclaim memory; reference counting is a
 * later milestone. No allocation failure is recoverable (the process exits).
 */

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>

typedef struct {
    const char *ptr;
    int64_t len;
} nova_str;

typedef struct {
    void *data;
    int64_t len;
    int64_t cap;
} nova_list;

static void nova_die_oom(void) {
    fflush(stdout);
    fputs("nova: out of memory\n", stderr);
    exit(70);
}

static void *nova_alloc(int64_t size) {
    void *p = malloc(size <= 0 ? 1 : (size_t)size);
    if (!p) nova_die_oom();
    return p;
}

static nova_str nova_make_str(const char *data, int64_t len) {
    nova_str s;
    s.ptr = data;
    s.len = len;
    return s;
}

static nova_str nova_copy_str(const char *data, int64_t len) {
    nova_str s;
    char *p = (char *)nova_alloc(len + 1);
    if (len > 0) memcpy(p, data, (size_t)len);
    p[len] = '\0';
    s.ptr = p;
    s.len = len;
    return s;
}

void nova_panic_overflow(int64_t line) {
    fflush(stdout);
    fprintf(stderr, "panic: integer overflow at line %lld\n", (long long)line);
    exit(1);
}

void nova_panic_divzero(int64_t line) {
    fflush(stdout);
    fprintf(stderr, "panic: division by zero at line %lld\n", (long long)line);
    exit(1);
}

void nova_panic_neg_overflow(int64_t line) {
    fflush(stdout);
    fprintf(stderr, "panic: integer negation overflow at line %lld\n", (long long)line);
    exit(1);
}

void nova_panic_bounds(int64_t index, int64_t len, int64_t line) {
    fflush(stdout);
    fprintf(stderr, "panic: index %lld out of bounds for list of length %lld at line %lld\n",
            (long long)index, (long long)len, (long long)line);
    exit(1);
}

void nova_panic_str(nova_str msg, int64_t line) {
    fflush(stdout);
    fprintf(stderr, "panic at line %lld: %.*s\n", (long long)line,
            (int)msg.len, msg.ptr ? msg.ptr : "");
    exit(1);
}

void nova_assert_fail(int64_t line) {
    fflush(stdout);
    fprintf(stderr, "assertion failed at line %lld\n", (long long)line);
    exit(1);
}

void nova_assert_eq_fail(int64_t line) {
    fflush(stdout);
    fprintf(stderr, "assertion failed: values are not equal (line %lld)\n", (long long)line);
    exit(1);
}

static void nova_write(nova_str s, int32_t newline, int32_t to_stderr) {
    FILE *out = to_stderr ? stderr : stdout;
    if (s.len > 0 && s.ptr) fwrite(s.ptr, 1, (size_t)s.len, out);
    if (newline) fputc('\n', out);
    fflush(out);
}

void nova_print_str(nova_str s, int32_t newline, int32_t to_stderr) {
    nova_write(s, newline, to_stderr);
}

void nova_print_i64(int64_t v, int32_t newline, int32_t to_stderr) {
    FILE *out = to_stderr ? stderr : stdout;
    fprintf(out, "%lld", (long long)v);
    if (newline) fputc('\n', out);
    fflush(out);
}

static int nova_double_equal(double a, double b) {
    if (isnan(a) && isnan(b)) return 1;
    return a == b;
}

void nova_print_f64(double v, int32_t newline, int32_t to_stderr) {
    char buf[64];
    int n = snprintf(buf, sizeof(buf), "%.17g", v);
    if (n > 0 && (size_t)n < sizeof(buf)) {
        double parsed = strtod(buf, NULL);
        if (!nova_double_equal(parsed, v)) {
            n = snprintf(buf, sizeof(buf), "%.17g", v);
        }
    }
    FILE *out = to_stderr ? stderr : stdout;
    if (n > 0) fwrite(buf, 1, (size_t)n < sizeof(buf) ? (size_t)n : sizeof(buf) - 1, out);
    if (newline) fputc('\n', out);
    fflush(out);
}

void nova_print_bool(int32_t v, int32_t newline, int32_t to_stderr) {
    FILE *out = to_stderr ? stderr : stdout;
    fputs(v ? "true" : "false", out);
    if (newline) fputc('\n', out);
    fflush(out);
}

static int nova_encode_utf8(uint32_t cp, char out[4]) {
    if (cp <= 0x7F) {
        out[0] = (char)cp;
        return 1;
    } else if (cp <= 0x7FF) {
        out[0] = (char)(0xC0 | (cp >> 6));
        out[1] = (char)(0x80 | (cp & 0x3F));
        return 2;
    } else if (cp <= 0xFFFF) {
        out[0] = (char)(0xE0 | (cp >> 12));
        out[1] = (char)(0x80 | ((cp >> 6) & 0x3F));
        out[2] = (char)(0x80 | (cp & 0x3F));
        return 3;
    } else if (cp <= 0x10FFFF) {
        out[0] = (char)(0xF0 | (cp >> 18));
        out[1] = (char)(0x80 | ((cp >> 12) & 0x3F));
        out[2] = (char)(0x80 | ((cp >> 6) & 0x3F));
        out[3] = (char)(0x80 | (cp & 0x3F));
        return 4;
    }
    return 0;
}

void nova_print_char(uint32_t cp, int32_t newline, int32_t to_stderr) {
    char buf[4];
    int n = nova_encode_utf8(cp, buf);
    FILE *out = to_stderr ? stderr : stdout;
    if (n > 0) fwrite(buf, 1, (size_t)n, out);
    if (newline) fputc('\n', out);
    fflush(out);
}

nova_str nova_string_concat(nova_str a, nova_str b) {
    int64_t len = a.len + b.len;
    nova_str result = nova_copy_str("", 0);
    char *p = (char *)nova_alloc(len + 1);
    if (a.len > 0 && a.ptr) memcpy(p, a.ptr, (size_t)a.len);
    if (b.len > 0 && b.ptr) memcpy(p + a.len, b.ptr, (size_t)b.len);
    p[len] = '\0';
    result.ptr = p;
    result.len = len;
    return result;
}

int32_t nova_string_eq(nova_str a, nova_str b) {
    if (a.len != b.len) return 0;
    if (a.len == 0) return 1;
    if (!a.ptr || !b.ptr) return 0;
    return memcmp(a.ptr, b.ptr, (size_t)a.len) == 0;
}

nova_str nova_i64_to_string(int64_t v) {
    char buf[32];
    int n = snprintf(buf, sizeof(buf), "%lld", (long long)v);
    return nova_copy_str(buf, n);
}

nova_str nova_f64_to_string(double v) {
    char buf[64];
    int n = snprintf(buf, sizeof(buf), "%.17g", v);
    if (n > 0 && (size_t)n < sizeof(buf)) {
        double parsed = strtod(buf, NULL);
        if (!nova_double_equal(parsed, v)) {
            n = snprintf(buf, sizeof(buf), "%.17g", v);
        }
    }
    return nova_copy_str(buf, n);
}

nova_str nova_bool_to_string(int32_t v) {
    return v ? nova_copy_str("true", 4) : nova_copy_str("false", 5);
}

nova_str nova_char_to_string(uint32_t cp) {
    char buf[4];
    int n = nova_encode_utf8(cp, buf);
    return nova_copy_str(buf, n);
}

void *nova_list_at(nova_list *list, int64_t index, int64_t elem_size, int64_t line) {
    if (index < 0 || index >= list->len) {
        nova_panic_bounds(index, list->len, line);
    }
    return (char *)list->data + (size_t)index * (size_t)elem_size;
}

void nova_list_push(nova_list *list, void *elem, int64_t elem_size) {
    if (elem_size <= 0) return;
    if (list->len >= list->cap) {
        int64_t new_cap = list->cap > 0 ? list->cap * 2 : 4;
        size_t bytes = (size_t)new_cap * (size_t)elem_size;
        void *p = realloc(list->data, bytes);
        if (!p) nova_die_oom();
        list->data = p;
        list->cap = new_cap;
    }
    memcpy((char *)list->data + (size_t)list->len * (size_t)elem_size,
           elem, (size_t)elem_size);
    list->len += 1;
}
