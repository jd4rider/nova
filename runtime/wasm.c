/* Freestanding wasm32 runtime for Nova.
 *
 * No libc, no POSIX and no allocator beyond a bump arena. Output goes into a
 * fixed buffer that JavaScript reads through nova_output_ptr/nova_output_len.
 * This is the first browser target: enough for strings, integers, booleans and
 * concatenation, which is what the component runtime will use.
 */

typedef struct {
    const char *ptr;
    long long len;
} nova_str;

typedef unsigned long long usize;

typedef struct {
    void *data;
    long long len;
    long long cap;
} nova_list;

#define NOVA_STATE_SLOTS 256

static long long nova_state[NOVA_STATE_SLOTS];
static unsigned char nova_heap[4 * 1024 * 1024];
static long long nova_heap_top = 0;
static char nova_out[256 * 1024];
static long long nova_out_len = 0;

void *memcpy(void *dst, const void *src, usize n) {
    unsigned char *d = (unsigned char *)dst;
    const unsigned char *s = (const unsigned char *)src;
    for (usize i = 0; i < n; i++) d[i] = s[i];
    return dst;
}

void *memset(void *dst, int value, usize n) {
    unsigned char *d = (unsigned char *)dst;
    for (usize i = 0; i < n; i++) d[i] = (unsigned char)value;
    return dst;
}

static void nova_emit(const char *data, long long len) {
    if (!data || len <= 0) return;
    long long room = (long long)sizeof(nova_out) - nova_out_len;
    if (len > room) len = room;
    if (len > 0) memcpy(nova_out + nova_out_len, data, (usize)len);
    nova_out_len += len;
}

void *nova_alloc(long long size);

void *nova_list_at(nova_list *list, long long index, long long elem_size, long long line) {
    (void)line;
    if (index < 0 || index >= list->len) return list->data;
    return (char *)list->data + index * elem_size;
}

void nova_list_push(nova_list *list, void *elem, long long elem_size) {
    if (list->len == list->cap) {
        long long next = list->cap == 0 ? 8 : list->cap * 2;
        void *grown = nova_alloc(next * elem_size);
        if (list->len > 0) memcpy(grown, list->data, (usize)(list->len * elem_size));
        list->data = grown;
        list->cap = next;
    }
    memcpy((char *)list->data + list->len * elem_size, elem, (usize)elem_size);
    list->len += 1;
}

void *nova_alloc(long long size) {
    if (size <= 0) size = 1;
    long long start = (nova_heap_top + 7) & ~7LL;
    if (start + size > (long long)sizeof(nova_heap)) return nova_heap;
    nova_heap_top = start + size;
    return nova_heap + start;
}

void nova_i64_to_string_impl(nova_str *out, long long value) {
    char digits[24];
    int at = 0;
    unsigned long long magnitude;
    int negative = value < 0;
    if (negative) {
        magnitude = (unsigned long long)(-(value + 1)) + 1ULL;
    } else {
        magnitude = (unsigned long long)value;
    }
    if (magnitude == 0) digits[at++] = '0';
    while (magnitude > 0) {
        digits[at++] = (char)('0' + (magnitude % 10ULL));
        magnitude /= 10ULL;
    }
    char *buffer = (char *)nova_alloc((long long)at + 1);
    long long written = 0;
    if (negative) buffer[written++] = '-';
    while (at > 0) buffer[written++] = digits[--at];
    out->ptr = buffer;
    out->len = written;
}

void nova_print_i64(long long value, int newline, int to_err);

void nova_panic_overflow(long long line) {
    nova_emit("panic: integer overflow at line ", 29);
    nova_print_i64(line, 1, 0);
}

void nova_panic_divzero(long long line) {
    nova_emit("panic: division by zero at line ", 31);
    nova_print_i64(line, 1, 0);
}

void nova_panic_neg_overflow(long long line) {
    nova_emit("panic: negation overflow at line ", 32);
    nova_print_i64(line, 1, 0);
}

void nova_print_str_impl(nova_str *value, int newline, int to_err) {
    (void)to_err;
    nova_emit(value->ptr, value->len);
    if (newline) nova_emit("\n", 1);
}

void nova_print_i64(long long value, int newline, int to_err) {
    (void)to_err;
    char digits[24];
    int at = 0;
    unsigned long long magnitude;
    int negative = value < 0;
    if (negative) {
        magnitude = (unsigned long long)(-(value + 1)) + 1ULL;
    } else {
        magnitude = (unsigned long long)value;
    }
    if (magnitude == 0) digits[at++] = '0';
    while (magnitude > 0) {
        digits[at++] = (char)('0' + (magnitude % 10ULL));
        magnitude /= 10ULL;
    }
    char out[26];
    int written = 0;
    if (negative) out[written++] = '-';
    while (at > 0) out[written++] = digits[--at];
    nova_emit(out, written);
    if (newline) nova_emit("\n", 1);
}

void nova_print_bool(int value, int newline, int to_err) {
    (void)to_err;
    if (value) {
        nova_emit("true", 4);
    } else {
        nova_emit("false", 5);
    }
    if (newline) nova_emit("\n", 1);
}

void nova_print_char(int code, int newline, int to_err) {
    (void)to_err;
    char bytes[4];
    int n = 0;
    unsigned int cp = (unsigned int)code;
    if (cp < 0x80) {
        bytes[n++] = (char)cp;
    } else if (cp < 0x800) {
        bytes[n++] = (char)(0xC0 | (cp >> 6));
        bytes[n++] = (char)(0x80 | (cp & 0x3F));
    } else {
        bytes[n++] = (char)(0xE0 | (cp >> 12));
        bytes[n++] = (char)(0x80 | ((cp >> 6) & 0x3F));
        bytes[n++] = (char)(0x80 | (cp & 0x3F));
    }
    nova_emit(bytes, n);
    if (newline) nova_emit("\n", 1);
}

void nova_print_f64(double value, int newline, int to_err) {
    (void)to_err;
    long long whole = (long long)value;
    nova_print_i64(whole, 0, 0);
    nova_emit(".0", 2);
    if (newline) nova_emit("\n", 1);
}

void nova_string_concat_impl(nova_str *out, nova_str *a, nova_str *b) {
    char *buffer = (char *)nova_alloc(a->len + b->len);
    if (a->len > 0) memcpy(buffer, a->ptr, (usize)a->len);
    if (b->len > 0) memcpy(buffer + a->len, b->ptr, (usize)b->len);
    out->ptr = buffer;
    out->len = a->len + b->len;
}

int nova_string_eq_impl(nova_str *a, nova_str *b) {
    if (a->len != b->len) return 0;
    for (long long i = 0; i < a->len; i++) {
        if (a->ptr[i] != b->ptr[i]) return 0;
    }
    return 1;
}

/* ------------------------------------------------------------------ */
/* String helpers needed by the formatter and other text tooling      */
/* ------------------------------------------------------------------ */

static nova_str nova_wcopy(const char *data, long long len) {
    char *buffer = (char *)nova_alloc(len + 1);
    if (len > 0) memcpy(buffer, data, (usize)len);
    buffer[len] = '\0';
    nova_str out;
    out.ptr = buffer;
    out.len = len;
    return out;
}

static int nova_wspace(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r';
}

static long long nova_wstep(const char *text, long long len, long long at) {
    unsigned char c = (unsigned char)text[at];
    long long width = 1;
    if (c >= 0xF0) {
        width = 4;
    } else if (c >= 0xE0) {
        width = 3;
    } else if (c >= 0xC0) {
        width = 2;
    }
    if (at + width > len) width = 1;
    return width;
}

void nova_string_chars_impl(nova_str *value, nova_list *result) {
    result->data = 0;
    result->len = 0;
    result->cap = 0;
    if (!value->ptr || value->len <= 0) return;
    long long i = 0;
    while (i < value->len) {
        long long width = nova_wstep(value->ptr, value->len, i);
        nova_str piece = nova_wcopy(value->ptr + i, width);
        nova_list_push(result, &piece, (long long)sizeof(nova_str));
        i += width;
    }
}

void nova_string_trim_impl(nova_str *out, nova_str *value) {
    long long start = 0;
    long long end = value->len;
    if (!value->ptr || value->len <= 0) {
        *out = nova_wcopy("", 0);
        return;
    }
    while (start < end && nova_wspace((unsigned char)value->ptr[start])) start++;
    while (end > start && nova_wspace((unsigned char)value->ptr[end - 1])) end--;
    *out = nova_wcopy(value->ptr + start, end - start);
}

void nova_string_trim_start_impl(nova_str *out, nova_str *value) {
    long long start = 0;
    if (!value->ptr || value->len <= 0) {
        *out = nova_wcopy("", 0);
        return;
    }
    while (start < value->len && nova_wspace((unsigned char)value->ptr[start])) start++;
    *out = nova_wcopy(value->ptr + start, value->len - start);
}

void nova_string_trim_end_impl(nova_str *out, nova_str *value) {
    long long end = value->len;
    if (!value->ptr || value->len <= 0) {
        *out = nova_wcopy("", 0);
        return;
    }
    while (end > 0 && nova_wspace((unsigned char)value->ptr[end - 1])) end--;
    *out = nova_wcopy(value->ptr, end);
}

void nova_string_split_impl(nova_str *value, nova_str *separator, nova_list *result) {
    result->data = 0;
    result->len = 0;
    result->cap = 0;
    if (!value->ptr) {
        nova_str empty = nova_wcopy("", 0);
        nova_list_push(result, &empty, (long long)sizeof(nova_str));
        return;
    }
    if (separator->len <= 0) {
        nova_str whole = nova_wcopy(value->ptr, value->len);
        nova_list_push(result, &whole, (long long)sizeof(nova_str));
        return;
    }
    long long start = 0;
    long long i = 0;
    while (i + separator->len <= value->len) {
        int matched = 1;
        for (long long k = 0; k < separator->len; k++) {
            if (value->ptr[i + k] != separator->ptr[k]) {
                matched = 0;
                break;
            }
        }
        if (matched) {
            nova_str piece = nova_wcopy(value->ptr + start, i - start);
            nova_list_push(result, &piece, (long long)sizeof(nova_str));
            i += separator->len;
            start = i;
        } else {
            i++;
        }
    }
    nova_str tail = nova_wcopy(value->ptr + start, value->len - start);
    nova_list_push(result, &tail, (long long)sizeof(nova_str));
}

void nova_string_join_impl(nova_str *out, nova_list *list, nova_str *separator) {
    if (!list || list->len <= 0 || !list->data) {
        *out = nova_wcopy("", 0);
        return;
    }
    nova_str *items = (nova_str *)list->data;
    long long total = separator->len * (list->len - 1);
    for (long long i = 0; i < list->len; i++) total += items[i].len;
    char *buffer = (char *)nova_alloc(total + 1);
    long long at = 0;
    for (long long i = 0; i < list->len; i++) {
        if (i > 0 && separator->len > 0) {
            memcpy(buffer + at, separator->ptr, (usize)separator->len);
            at += separator->len;
        }
        if (items[i].len > 0 && items[i].ptr) {
            memcpy(buffer + at, items[i].ptr, (usize)items[i].len);
            at += items[i].len;
        }
    }
    buffer[at] = '\0';
    out->ptr = buffer;
    out->len = at;
}

/* ------------------------------------------------------------------ */
/* Input buffer                                                       */
/* ------------------------------------------------------------------ */

static char nova_input[1024 * 1024];
static long long nova_input_len = 0;

void *nova_input_ptr(void) {
    return nova_input;
}

void nova_input_set(long long len) {
    if (len < 0) len = 0;
    if (len > (long long)sizeof(nova_input)) len = (long long)sizeof(nova_input);
    nova_input_len = len;
}

void nova_output_write_impl(nova_str *value) {
    nova_emit(value->ptr, value->len);
}

void nova_input_str_impl(nova_str *out) {
    out->ptr = nova_input;
    out->len = nova_input_len;
}

long long nova_output_ptr(void) {
    return (long long)(usize)nova_out;
}

long long nova_output_len(void) {
    return nova_out_len;
}

void nova_reset_output(void) {
    nova_out_len = 0;
}

long long nova_state_get(long long slot) {
    if (slot < 0 || slot >= NOVA_STATE_SLOTS) return 0;
    return nova_state[slot];
}

void nova_state_set(long long slot, long long value) {
    if (slot < 0 || slot >= NOVA_STATE_SLOTS) return;
    nova_state[slot] = value;
}
