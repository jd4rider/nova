/*
 * Nova native runtime (MVP).
 *
 * Memory model: strings and lists are allocated for the lifetime of the
 * process. This is safe but does not reclaim memory; reference counting is a
 * later milestone. No allocation failure is recoverable (the process exits).
 */

#if !defined(_WIN32)
#define _POSIX_C_SOURCE 200809L
#define _DEFAULT_SOURCE
#endif

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <conio.h>
#include <io.h>
#include <windows.h>
#else
#include <poll.h>
#include <signal.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <termios.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#endif

typedef struct {
    const char *ptr;
    int64_t len;
} nova_str;

typedef struct {
    void *data;
    int64_t len;
    int64_t cap;
} nova_list;

void nova_list_push(nova_list *list, void *elem, int64_t elem_size);
int32_t nova_term_leave_raw(void);

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

static int nova_utf8_decode(
    const unsigned char *text, int64_t len, int64_t index, uint32_t *cp, int64_t *next
);

/* ------------------------------------------------------------------ */
/* Terminal input and size                                            */
/* ------------------------------------------------------------------ */

/* Key codes above the Unicode range. Mouse events carry their coordinates in
 * nova_mouse_x/nova_mouse_y; a paste announces itself with 3000 and its text is
 * buffered for nova_term_paste_next / nova_term_take_paste. */
#define NOVA_KEY_RESIZE 1020
#define NOVA_KEY_PASTE 3000

static int32_t nova_mouse_x = 0;
static int32_t nova_mouse_y = 0;

static char *nova_paste_data = NULL;
static int64_t nova_paste_len = 0;
static int64_t nova_paste_cap = 0;
static int64_t nova_paste_pos = 0;
static int64_t nova_paste_total = 0;

static int32_t nova_last_cols = -1;
static int32_t nova_last_rows = -1;

static int32_t nova_term_resized(void);
static int32_t nova_mouse_code(int button, int release);
static void nova_read_paste(void);

#if defined(_WIN32)
static DWORD nova_saved_input_mode;
static DWORD nova_saved_output_mode;
static int nova_raw_active = 0;
static int nova_atexit_registered = 0;

static void nova_restore_terminal(void) {
    nova_term_leave_raw();
}

int32_t nova_term_is_tty(void) {
    return _isatty(_fileno(stdin)) && _isatty(_fileno(stdout));
}

int32_t nova_term_enter_raw(void) {
    HANDLE input = GetStdHandle(STD_INPUT_HANDLE);
    HANDLE output = GetStdHandle(STD_OUTPUT_HANDLE);
    if (input == INVALID_HANDLE_VALUE || output == INVALID_HANDLE_VALUE) return 0;
    if (!GetConsoleMode(input, &nova_saved_input_mode)) return 0;
    if (!GetConsoleMode(output, &nova_saved_output_mode)) return 0;
    DWORD input_mode = nova_saved_input_mode & ~(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT);
    if (!SetConsoleMode(input, input_mode)) return 0;
    SetConsoleMode(output, nova_saved_output_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    nova_raw_active = 1;
    if (!nova_atexit_registered) {
        atexit(nova_restore_terminal);
        nova_atexit_registered = 1;
    }
    return 1;
}

int32_t nova_term_leave_raw(void) {
    if (!nova_raw_active) return 1;
    HANDLE input = GetStdHandle(STD_INPUT_HANDLE);
    HANDLE output = GetStdHandle(STD_OUTPUT_HANDLE);
    if (input != INVALID_HANDLE_VALUE) SetConsoleMode(input, nova_saved_input_mode);
    if (output != INVALID_HANDLE_VALUE) SetConsoleMode(output, nova_saved_output_mode);
    nova_raw_active = 0;
    return 1;
}

int32_t nova_term_width(void) {
    CONSOLE_SCREEN_BUFFER_INFO info;
    HANDLE output = GetStdHandle(STD_OUTPUT_HANDLE);
    if (GetConsoleScreenBufferInfo(output, &info)) {
        return (int32_t)(info.srWindow.Right - info.srWindow.Left + 1);
    }
    return 80;
}

int32_t nova_term_height(void) {
    CONSOLE_SCREEN_BUFFER_INFO info;
    HANDLE output = GetStdHandle(STD_OUTPUT_HANDLE);
    if (GetConsoleScreenBufferInfo(output, &info)) {
        return (int32_t)(info.srWindow.Bottom - info.srWindow.Top + 1);
    }
    return 24;
}

int64_t nova_term_read_key(int64_t timeout_ms) {
    if (nova_term_resized()) return NOVA_KEY_RESIZE;
    if (timeout_ms < 0) {
        while (!_kbhit()) {
            Sleep(10);
            if (nova_term_resized()) return NOVA_KEY_RESIZE;
        }
    } else {
        int64_t waited = 0;
        while (!_kbhit() && waited < timeout_ms) {
            Sleep(10);
            waited += 10;
            if (nova_term_resized()) return NOVA_KEY_RESIZE;
        }
        if (!_kbhit()) return -1;
    }
    int ch = _getch();
    if (ch == 0 || ch == 224) {
        int code = _getch();
        switch (code) {
            case 72: return 1001;
            case 80: return 1002;
            case 77: return 1003;
            case 75: return 1004;
            case 71: return 1005;
            case 79: return 1006;
            case 73: return 1007;
            case 81: return 1008;
            case 83: return 1009;
            case 82: return 1010;
            default: return ch;
        }
    }
    return ch;
}
#else
static struct termios nova_saved_termios;
static int nova_raw_active = 0;
static int nova_atexit_registered = 0;

static void nova_restore_terminal(void) {
    nova_term_leave_raw();
}

int32_t nova_term_is_tty(void) {
    return isatty(STDIN_FILENO) && isatty(STDOUT_FILENO);
}

int32_t nova_term_enter_raw(void) {
    if (!isatty(STDIN_FILENO)) return 0;
    if (nova_raw_active) return 1;
    if (tcgetattr(STDIN_FILENO, &nova_saved_termios) != 0) return 0;
    struct termios raw = nova_saved_termios;
    raw.c_lflag &= (tcflag_t)~(ICANON | ECHO | IEXTEN | ISIG);
    raw.c_iflag &= (tcflag_t)~(IXON | ICRNL | BRKINT | INPCK | ISTRIP);
    raw.c_cflag |= (tcflag_t)CS8;
    raw.c_cc[VMIN] = 1;
    raw.c_cc[VTIME] = 0;
    if (tcsetattr(STDIN_FILENO, TCSANOW, &raw) != 0) return 0;
    nova_raw_active = 1;
    if (!nova_atexit_registered) {
        atexit(nova_restore_terminal);
        nova_atexit_registered = 1;
    }
    return 1;
}

int32_t nova_term_leave_raw(void) {
    if (!nova_raw_active) return 1;
    if (tcsetattr(STDIN_FILENO, TCSANOW, &nova_saved_termios) != 0) return 0;
    nova_raw_active = 0;
    return 1;
}

int32_t nova_term_width(void) {
    struct winsize size;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &size) == 0 && size.ws_col > 0) {
        return (int32_t)size.ws_col;
    }
    const char *columns = getenv("COLUMNS");
    if (columns) {
        int value = atoi(columns);
        if (value > 0) return value;
    }
    return 80;
}

int32_t nova_term_height(void) {
    struct winsize size;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &size) == 0 && size.ws_row > 0) {
        return (int32_t)size.ws_row;
    }
    const char *lines = getenv("LINES");
    if (lines) {
        int value = atoi(lines);
        if (value > 0) return value;
    }
    return 24;
}

/* 1 = byte read, 0 = timeout, -1 = poll error, -2 = EOF. */
static int nova_read_byte(int64_t timeout_ms, unsigned char *out) {
    struct pollfd pfd;
    pfd.fd = STDIN_FILENO;
    pfd.events = POLLIN;
    pfd.revents = 0;
    int ready = poll(&pfd, 1, timeout_ms < 0 ? -1 : (int)timeout_ms);
    if (ready == 0) return 0;
    if (ready < 0) return -1;
    if (pfd.revents & (POLLHUP | POLLERR | POLLNVAL)) return -2;
    if (!(pfd.revents & POLLIN)) return 0;
    ssize_t count = read(STDIN_FILENO, out, 1);
    if (count == 0) return -2;
    if (count < 0) return -1;
    return 1;
}

static int32_t nova_decode_escape(void) {
    unsigned char buffer[32];
    int length = 0;
    while (length < (int)sizeof(buffer)) {
        int status = nova_read_byte(20, &buffer[length]);
        if (status <= 0) break;
        length += 1;
        unsigned char c = buffer[length - 1];
        // `[` and `O` are introducers, not the final byte of a sequence.
        if (length == 1 && (c == '[' || c == 'O')) continue;
        if (c >= 0x40 && c <= 0x7e) break;
    }
    if (length == 0) return 27;
    /* Legacy X10 mouse: ESC [ M followed by button, column and row bytes. */
    if (buffer[0] == '[' && length == 2 && buffer[1] == 'M') {
        unsigned char mouse[3];
        for (int i = 0; i < 3; i++) {
            if (nova_read_byte(20, &mouse[i]) <= 0) return 27;
        }
        nova_mouse_x = (int32_t)mouse[1] - 32;
        nova_mouse_y = (int32_t)mouse[2] - 32;
        return nova_mouse_code((int)mouse[0] - 32, 0);
    }
    /* SGR mouse: ESC [ < button ; column ; row M (press/motion) or m (release). */
    if (buffer[0] == '[' && length > 2 && buffer[1] == '<') {
        char text[64];
        int copy = length < 63 ? length : 63;
        int button = 0;
        int x = 0;
        int y = 0;
        char final_byte = 0;
        memcpy(text, buffer, (size_t)copy);
        text[copy] = '\0';
        if (sscanf(text, "[<%d;%d;%d%c", &button, &x, &y, &final_byte) == 4 &&
            (final_byte == 'M' || final_byte == 'm')) {
            nova_mouse_x = (int32_t)x;
            nova_mouse_y = (int32_t)y;
            if (button & 64) {
                return nova_mouse_code(button, 0);
            }
            return nova_mouse_code(button, final_byte == 'm');
        }
        return 27;
    }
    if (buffer[0] == '[') {
        unsigned char last = buffer[length - 1];
        if (last == 'A') return 1001;
        if (last == 'B') return 1002;
        if (last == 'C') return 1003;
        if (last == 'D') return 1004;
        if (last == 'H') return 1005;
        if (last == 'F') return 1006;
        if (last == 'Z') return 1011;
        if (last == '~' && length == 5 && buffer[1] == '2' && buffer[2] == '0' && buffer[3] == '0') {
            nova_read_paste();
            return NOVA_KEY_PASTE;
        }
        if (last == '~' && length >= 2) {
            switch (buffer[1]) {
                case '1': return 1005;
                case '2': return 1010;
                case '3': return 1009;
                case '4': return 1006;
                case '5': return 1007;
                case '6': return 1008;
                case '7': return 1005;
                case '8': return 1006;
                default: return 27;
            }
        }
    }
    if (buffer[0] == 'O') {
        unsigned char last = buffer[length - 1];
        if (last == 'A') return 1001;
        if (last == 'B') return 1002;
        if (last == 'C') return 1003;
        if (last == 'D') return 1004;
        if (last == 'H') return 1005;
        if (last == 'F') return 1006;
    }
    return 27;
}

int64_t nova_term_read_key(int64_t timeout_ms) {
    if (nova_term_resized()) return NOVA_KEY_RESIZE;
    unsigned char first = 0;
    int64_t waited = 0;
    for (;;) {
        int64_t slice = 50;
        if (timeout_ms >= 0) {
            int64_t left = timeout_ms - waited;
            if (left <= 0) return -1;
            if (left < slice) slice = left;
        }
        int status = nova_read_byte(slice, &first);
        if (status == 1) break;
        if (status < 0) return -2;
        waited += slice;
        if (timeout_ms >= 0 && waited >= timeout_ms) return -1;
        if (nova_term_resized()) return NOVA_KEY_RESIZE;
    }
    if (first == 27) return nova_decode_escape();
    if (first < 0x80) return (int32_t)first;

    int extra = 0;
    uint32_t cp = 0;
    if ((first & 0xe0) == 0xc0) {
        extra = 1;
        cp = first & 0x1f;
    } else if ((first & 0xf0) == 0xe0) {
        extra = 2;
        cp = first & 0x0f;
    } else if ((first & 0xf8) == 0xf0) {
        extra = 3;
        cp = first & 0x07;
    } else {
        return 0xfffd;
    }
    for (int i = 0; i < extra; i++) {
        unsigned char next = 0;
        if (nova_read_byte(20, &next) <= 0) return 0xfffd;
        if ((next & 0xc0) != 0x80) return 0xfffd;
        cp = (cp << 6) | (next & 0x3f);
    }
    return (int32_t)cp;
}
#endif

/* 1 when the size differs from the previous call; the first call only records
 * it. Polling this is how a resize becomes an event without a signal handler. */
static int32_t nova_term_resized(void) {
    int32_t cols = nova_term_width();
    int32_t rows = nova_term_height();
    if (nova_last_cols < 0) {
        nova_last_cols = cols;
        nova_last_rows = rows;
        return 0;
    }
    if (cols == nova_last_cols && rows == nova_last_rows) return 0;
    nova_last_cols = cols;
    nova_last_rows = rows;
    return 1;
}

int64_t nova_term_mouse_x(void) { return nova_mouse_x; }

int64_t nova_term_mouse_y(void) { return nova_mouse_y; }

static void nova_paste_reset(void) {
    nova_paste_len = 0;
    nova_paste_pos = 0;
    nova_paste_total = 0;
}

static void nova_paste_push(unsigned char byte) {
    if (nova_paste_len >= nova_paste_cap) {
        int64_t cap = nova_paste_cap > 0 ? nova_paste_cap * 2 : 256;
        char *grown = (char *)realloc(nova_paste_data, (size_t)cap);
        if (!grown) nova_die_oom();
        nova_paste_data = grown;
        nova_paste_cap = cap;
    }
    nova_paste_data[nova_paste_len++] = (char)byte;
}

/* Bracketed paste ends with ESC [ 201 ~. CRLF is folded to LF so the buffer is
 * stable across terminals. */
static void nova_read_paste(void) {
    nova_paste_reset();
    int after_cr = 0;
    for (;;) {
        unsigned char byte = 0;
        if (nova_read_byte(2000, &byte) <= 0) break;
        if (byte == '\r') {
            nova_paste_push('\n');
            after_cr = 1;
            continue;
        }
        if (byte == '\n' && after_cr) {
            after_cr = 0;
            continue;
        }
        after_cr = 0;
        nova_paste_push(byte);
        int64_t len = nova_paste_len;
        if (len >= 6 && memcmp(nova_paste_data + len - 6, "\x1b[201~", 6) == 0) {
            nova_paste_len -= 6;
            break;
        }
    }
    int64_t i = 0;
    int64_t count = 0;
    while (i < nova_paste_len) {
        uint32_t cp = 0;
        int64_t next = i;
        if (!nova_utf8_decode((const unsigned char *)nova_paste_data, nova_paste_len, i, &cp, &next)) break;
        i = next;
        count += 1;
    }
    nova_paste_total = count;
}

int64_t nova_term_paste_len(void) { return (int32_t)nova_paste_total; }

int64_t nova_term_paste_next(void) {
    if (nova_paste_pos >= nova_paste_len) return -1;
    uint32_t cp = 0;
    int64_t next = nova_paste_pos;
    if (!nova_utf8_decode(
            (const unsigned char *)nova_paste_data, nova_paste_len, nova_paste_pos, &cp, &next)) {
        nova_paste_pos = nova_paste_len;
        return -1;
    }
    nova_paste_pos = next;
    return (int32_t)cp;
}

/* Consumes the buffered paste. Codegen exposes this as the `term_paste()`
 * builtin because scalar FFI cannot return a String. */
nova_str nova_term_take_paste(void) {
    nova_str result = nova_copy_str(nova_paste_data ? nova_paste_data : "", nova_paste_len);
    nova_paste_pos = nova_paste_len;
    return result;
}

/* ------------------------------------------------------------------ */
/* HTTP and JSON                                                      */
/* ------------------------------------------------------------------ */

/* Fetches a URL with the host `curl` binary and returns the response body.
 * The child is exec'd directly (no shell), so a URL containing shell
 * metacharacters cannot be interpreted as a command. An empty string means the
 * request failed; callers decide how to report that. POSIX only. */
nova_str nova_http_get(nova_str url) {
#if defined(_WIN32)
    (void)url;
    return nova_copy_str("", 0);
#else
    char *target = (char *)nova_alloc(url.len + 1);
    if (url.len > 0) memcpy(target, url.ptr, (size_t)url.len);
    target[url.len] = '\0';

    int fds[2];
    if (pipe(fds) != 0) {
        free(target);
        return nova_copy_str("", 0);
    }
    pid_t pid = fork();
    if (pid < 0) {
        close(fds[0]);
        close(fds[1]);
        free(target);
        return nova_copy_str("", 0);
    }
    if (pid == 0) {
        dup2(fds[1], STDOUT_FILENO);
        close(fds[0]);
        close(fds[1]);
        execlp("curl", "curl", "-sS", "--max-time", "20", "--url", target, (char *)NULL);
        _exit(127);
    }
    close(fds[1]);

    char *buffer = NULL;
    int64_t len = 0;
    int64_t cap = 0;
    char chunk[4096];
    ssize_t got;
    while ((got = read(fds[0], chunk, sizeof(chunk))) > 0) {
        if (len + got > cap) {
            int64_t next_cap = (len + got) * 2;
            char *grown = (char *)realloc(buffer, (size_t)next_cap);
            if (!grown) {
                free(buffer);
                close(fds[0]);
                free(target);
                nova_die_oom();
            }
            buffer = grown;
            cap = next_cap;
        }
        memcpy(buffer + len, chunk, (size_t)got);
        len += got;
    }
    close(fds[0]);
    int status = 0;
    waitpid(pid, &status, 0);
    free(target);
    if (!buffer) return nova_copy_str("", 0);
    nova_str out = nova_copy_str(buffer, len);
    free(buffer);
    return out;
#endif
}

/* Extracts a JSON string field. This is deliberately not a parser: it finds
 * the last "key": "value" occurrence (so a top-level field wins over the same
 * key nested in an earlier array) and decodes the standard JSON escapes.
 * Numbers, arrays and objects are not decoded. Returns "" when the key is
 * absent. */
nova_str nova_json_string(nova_str json, nova_str key) {
    char *needle = (char *)nova_alloc(key.len + 3);
    needle[0] = '"';
    if (key.len > 0) memcpy(needle + 1, key.ptr, (size_t)key.len);
    needle[key.len + 1] = '"';
    needle[key.len + 2] = '\0';

    char *body = (char *)nova_alloc(json.len + 1);
    if (json.len > 0) memcpy(body, json.ptr, (size_t)json.len);
    body[json.len] = '\0';

    char *at = NULL;
    char *scan = body;
    char *found;
    while ((found = strstr(scan, needle)) != NULL) {
        at = found;
        scan = found + 1;
    }
    free(needle);
    if (!at) {
        free(body);
        return nova_copy_str("", 0);
    }
    at += key.len + 2;
    while (*at == ' ' || *at == '\t' || *at == '\n' || *at == '\r') at++;
    if (*at != ':') {
        free(body);
        return nova_copy_str("", 0);
    }
    at++;
    while (*at == ' ' || *at == '\t' || *at == '\n' || *at == '\r') at++;
    if (*at != '"') {
        free(body);
        return nova_copy_str("", 0);
    }
    at++;

    int64_t cap = (int64_t)strlen(at) + 1;
    char *out = (char *)nova_alloc(cap);
    int64_t len = 0;
    while (*at && *at != '"') {
        unsigned char c = (unsigned char)*at++;
        if (c != '\\') {
            out[len++] = (char)c;
            continue;
        }
        char escape = *at++;
        switch (escape) {
            case '"': out[len++] = '"'; break;
            case '\\': out[len++] = '\\'; break;
            case '/': out[len++] = '/'; break;
            case 'b': out[len++] = '\b'; break;
            case 'f': out[len++] = '\f'; break;
            case 'n': out[len++] = '\n'; break;
            case 'r': out[len++] = '\r'; break;
            case 't': out[len++] = '\t'; break;
            case 'u': {
                uint32_t cp = 0;
                for (int i = 0; i < 4; i++) {
                    char digit = *at++;
                    cp <<= 4;
                    if (digit >= '0' && digit <= '9') cp |= (uint32_t)(digit - '0');
                    else if (digit >= 'a' && digit <= 'f') cp |= (uint32_t)(digit - 'a' + 10);
                    else if (digit >= 'A' && digit <= 'F') cp |= (uint32_t)(digit - 'A' + 10);
                }
                /* Lone surrogates are replaced; pairs are not combined. */
                if (cp >= 0xD800 && cp <= 0xDFFF) cp = 0xFFFD;
                if (cp < 0x80) {
                    out[len++] = (char)cp;
                } else if (cp < 0x800) {
                    out[len++] = (char)(0xC0 | (cp >> 6));
                    out[len++] = (char)(0x80 | (cp & 0x3F));
                } else {
                    out[len++] = (char)(0xE0 | (cp >> 12));
                    out[len++] = (char)(0x80 | ((cp >> 6) & 0x3F));
                    out[len++] = (char)(0x80 | (cp & 0x3F));
                }
                break;
            }
            default: out[len++] = escape; break;
        }
    }
    free(body);
    nova_str result = nova_copy_str(out, len);
    free(out);
    return result;
}

/* ------------------------------------------------------------------ */
/* HTTP server                                                        */
/* ------------------------------------------------------------------ */

/* A single-threaded, blocking HTTP/1.1 server bound to 127.0.0.1. One request
 * is held at a time: nova_http_accept parses it, the program reads it through
 * the http_* builtins, and nova_http_respond writes the response and closes the
 * connection. Request bodies are capped at 4 MiB. POSIX only; the Windows
 * stubs report failure so a program can print a useful message. */

#if defined(_WIN32)
int64_t nova_http_listen(int64_t port) {
    (void)port;
    return -1;
}
int64_t nova_http_accept(void) { return -1; }
int64_t nova_http_close(void) { return -1; }
nova_str nova_http_method(void) { return nova_copy_str("", 0); }
nova_str nova_http_path(void) { return nova_copy_str("", 0); }
nova_str nova_http_query(void) { return nova_copy_str("", 0); }
nova_str nova_http_body(void) { return nova_copy_str("", 0); }
nova_str nova_http_header(nova_str name) {
    (void)name;
    return nova_copy_str("", 0);
}
nova_str nova_http_form(nova_str name) {
    (void)name;
    return nova_copy_str("", 0);
}
int64_t nova_http_respond(int64_t status, nova_str content_type, nova_str body) {
    (void)status;
    (void)content_type;
    (void)body;
    return -1;
}
#else

#include <ctype.h>

static int nova_srv_fd = -1;
static int nova_srv_client = -1;
static nova_str nova_srv_method = {NULL, 0};
static nova_str nova_srv_path = {NULL, 0};
static nova_str nova_srv_query = {NULL, 0};
static nova_str nova_srv_body = {NULL, 0};
static nova_str nova_srv_headers = {NULL, 0};

int64_t nova_http_close(void);

static int nova_srv_ci_eq(const char *a, int64_t alen, const char *b, int64_t blen) {
    if (alen != blen) return 0;
    for (int64_t i = 0; i < alen; i++) {
        if (tolower((unsigned char)a[i]) != tolower((unsigned char)b[i])) return 0;
    }
    return 1;
}

static int64_t nova_srv_send_all(int fd, const char *data, int64_t len) {
    int64_t sent = 0;
    while (sent < len) {
        ssize_t n = send(fd, data + sent, (size_t)(len - sent), 0);
        if (n <= 0) return -1;
        sent += n;
    }
    return sent;
}

static const char *nova_srv_reason(int64_t status) {
    switch (status) {
        case 200: return "OK";
        case 201: return "Created";
        case 202: return "Accepted";
        case 204: return "No Content";
        case 301: return "Moved Permanently";
        case 302: return "Found";
        case 303: return "See Other";
        case 304: return "Not Modified";
        case 307: return "Temporary Redirect";
        case 308: return "Permanent Redirect";
        case 400: return "Bad Request";
        case 401: return "Unauthorized";
        case 403: return "Forbidden";
        case 404: return "Not Found";
        case 405: return "Method Not Allowed";
        case 409: return "Conflict";
        case 418: return "I'm a teapot";
        case 422: return "Unprocessable Content";
        case 429: return "Too Many Requests";
        case 500: return "Internal Server Error";
        case 503: return "Service Unavailable";
        default: return "OK";
    }
}

int64_t nova_http_listen(int64_t port) {
    nova_http_close();
    signal(SIGPIPE, SIG_IGN);
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    int reuse = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    addr.sin_addr.s_addr = htonl(0x7f000001U);
    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) != 0 || listen(fd, 16) != 0) {
        close(fd);
        return -1;
    }
    nova_srv_fd = fd;
    return 0;
}

static int64_t nova_srv_content_length(const char *headers, int64_t len) {
    int64_t i = 0;
    while (i < len) {
        int64_t start = i;
        while (i < len && headers[i] != '\r' && headers[i] != '\n') i++;
        int64_t stop = i;
        while (i < len && (headers[i] == '\r' || headers[i] == '\n')) i++;
        const char *colon = memchr(headers + start, ':', (size_t)(stop - start));
        if (!colon) continue;
        int64_t name_len = colon - (headers + start);
        if (!nova_srv_ci_eq(headers + start, name_len, "Content-Length", 14)) continue;
        const char *value = colon + 1;
        const char *value_end = headers + stop;
        while (value < value_end && (*value == ' ' || *value == '\t')) value++;
        int64_t number = 0;
        while (value < value_end && *value >= '0' && *value <= '9') {
            number = number * 10 + (*value - '0');
            value++;
        }
        return number;
    }
    return 0;
}

int64_t nova_http_accept(void) {
    if (nova_srv_fd < 0) return -1;
    int client = accept(nova_srv_fd, NULL, NULL);
    if (client < 0) return -1;

    int64_t cap = 8192;
    int64_t len = 0;
    char *buf = (char *)nova_alloc(cap);
    buf[0] = '\0';
    char *head_end = NULL;
    while (len < 1048576) {
        if (len + 65537 > cap) {
            cap = (len + 65537) * 2;
            char *grown = (char *)realloc(buf, (size_t)cap);
            if (!grown) {
                close(client);
                free(buf);
                nova_die_oom();
            }
            buf = grown;
        }
        ssize_t got = recv(client, buf + len, (size_t)(cap - len - 1), 0);
        if (got <= 0) break;
        len += got;
        buf[len] = '\0';
        head_end = strstr(buf, "\r\n\r\n");
        if (head_end) break;
    }
    if (!head_end) {
        close(client);
        free(buf);
        return -1;
    }

    char *line_end = strstr(buf, "\r\n");
    char *first_space = line_end ? memchr(buf, ' ', (size_t)(line_end - buf)) : NULL;
    char *second_space = first_space
        ? memchr(first_space + 1, ' ', (size_t)(line_end - first_space - 1))
        : NULL;
    if (!line_end || !first_space || !second_space) {
        close(client);
        free(buf);
        return -1;
    }

    const char *target = first_space + 1;
    int64_t target_len = second_space - target;
    const char *question = memchr(target, '?', (size_t)target_len);
    int64_t path_len = question ? question - target : target_len;
    const char *query = question ? question + 1 : target + target_len;
    int64_t query_len = question ? target_len - path_len - 1 : 0;

    int64_t headers_len = head_end - (line_end + 2);
    int64_t need = nova_srv_content_length(line_end + 2, headers_len);
    if (need < 0) need = 0;
    if (need > 4194304) need = 4194304;

    const char *body_start = head_end + 4;
    int64_t have = len - (body_start - buf);
    int64_t copied = have < need ? have : need;
    char *body_buf = (char *)nova_alloc(need + 1);
    if (copied > 0) memcpy(body_buf, body_start, (size_t)copied);
    while (copied < need) {
        ssize_t got = recv(client, body_buf + copied, (size_t)(need - copied), 0);
        if (got <= 0) break;
        copied += got;
    }
    body_buf[copied] = '\0';

    nova_srv_method = nova_copy_str(buf, first_space - buf);
    nova_srv_path = nova_copy_str(target, path_len);
    nova_srv_query = nova_copy_str(query, query_len);
    nova_srv_headers = nova_copy_str(line_end + 2, headers_len);
    nova_srv_body = nova_copy_str(body_buf, copied);

    free(body_buf);
    free(buf);
    nova_srv_client = client;
    return 1;
}

nova_str nova_http_method(void) {
    return nova_copy_str(nova_srv_method.ptr ? nova_srv_method.ptr : "", nova_srv_method.len);
}

nova_str nova_http_path(void) {
    return nova_copy_str(nova_srv_path.ptr ? nova_srv_path.ptr : "", nova_srv_path.len);
}

nova_str nova_http_query(void) {
    return nova_copy_str(nova_srv_query.ptr ? nova_srv_query.ptr : "", nova_srv_query.len);
}

nova_str nova_http_body(void) {
    return nova_copy_str(nova_srv_body.ptr ? nova_srv_body.ptr : "", nova_srv_body.len);
}

nova_str nova_http_header(nova_str name) {
    const char *headers = nova_srv_headers.ptr;
    int64_t len = nova_srv_headers.len;
    if (!headers || !name.ptr) return nova_copy_str("", 0);
    int64_t i = 0;
    while (i < len) {
        int64_t start = i;
        while (i < len && headers[i] != '\r' && headers[i] != '\n') i++;
        int64_t stop = i;
        while (i < len && (headers[i] == '\r' || headers[i] == '\n')) i++;
        const char *colon = memchr(headers + start, ':', (size_t)(stop - start));
        if (!colon) continue;
        int64_t name_len = colon - (headers + start);
        if (!nova_srv_ci_eq(headers + start, name_len, name.ptr, name.len)) continue;
        const char *value = colon + 1;
        const char *value_end = headers + stop;
        while (value < value_end && (*value == ' ' || *value == '\t')) value++;
        while (value_end > value && (value_end[-1] == ' ' || value_end[-1] == '\t')) value_end--;
        return nova_copy_str(value, value_end - value);
    }
    return nova_copy_str("", 0);
}

static int nova_srv_hex(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

static nova_str nova_srv_decode(const char *src, int64_t len) {
    char *out = (char *)nova_alloc(len + 1);
    int64_t written = 0;
    for (int64_t i = 0; i < len; i++) {
        char c = src[i];
        if (c == '+') {
            out[written++] = ' ';
            continue;
        }
        if (c == '%' && i + 2 < len) {
            int high = nova_srv_hex(src[i + 1]);
            int low = nova_srv_hex(src[i + 2]);
            if (high >= 0 && low >= 0) {
                out[written++] = (char)((high << 4) | low);
                i += 2;
                continue;
            }
        }
        out[written++] = c;
    }
    nova_str result = nova_copy_str(out, written);
    free(out);
    return result;
}

nova_str nova_http_form(nova_str name) {
    const char *src = nova_srv_body.len > 0 ? nova_srv_body.ptr : nova_srv_query.ptr;
    int64_t len = nova_srv_body.len > 0 ? nova_srv_body.len : nova_srv_query.len;
    if (!src || !name.ptr) return nova_copy_str("", 0);
    int64_t i = 0;
    while (i < len) {
        int64_t start = i;
        while (i < len && src[i] != '&') i++;
        int64_t stop = i;
        if (i < len) i++;
        const char *equals = memchr(src + start, '=', (size_t)(stop - start));
        int64_t key_len = equals ? equals - (src + start) : stop - start;
        if (!nova_srv_ci_eq(src + start, key_len, name.ptr, name.len)) continue;
        if (!equals) return nova_copy_str("", 0);
        int64_t value_len = stop - (equals - src) - 1;
        return nova_srv_decode(equals + 1, value_len);
    }
    return nova_copy_str("", 0);
}

int64_t nova_http_respond(int64_t status, nova_str content_type, nova_str body) {
    if (nova_srv_client < 0) return -1;
    char header[512];
    int written = snprintf(
        header,
        sizeof(header),
        "HTTP/1.1 %lld %s\r\nContent-Type: %.*s\r\nContent-Length: %lld\r\nConnection: close\r\n\r\n",
        (long long)status,
        nova_srv_reason(status),
        (int)content_type.len,
        content_type.ptr ? content_type.ptr : "",
        (long long)body.len
    );
    if (written < 0 || written >= (int)sizeof(header)) {
        close(nova_srv_client);
        nova_srv_client = -1;
        return -1;
    }
    int ok = nova_srv_send_all(nova_srv_client, header, written) >= 0;
    if (ok && body.len > 0) ok = nova_srv_send_all(nova_srv_client, body.ptr, body.len) >= 0;
    close(nova_srv_client);
    nova_srv_client = -1;
    return ok ? 0 : -1;
}

int64_t nova_http_close(void) {
    if (nova_srv_client >= 0) {
        close(nova_srv_client);
        nova_srv_client = -1;
    }
    if (nova_srv_fd < 0) return -1;
    close(nova_srv_fd);
    nova_srv_fd = -1;
    return 0;
}
#endif

/* 2000..2002 press, 2003 release, 2004/2005 wheel up/down, 2006/2007 wheel
 * left/right, 2010..2012 drag. Modifier bits are ignored. */
static int32_t nova_mouse_code(int button, int release) {
    if (button & 64) {
        switch (button & 3) {
            case 0: return 2004;
            case 1: return 2005;
            case 2: return 2006;
            default: return 2007;
        }
    }
    if (button & 32) return 2010 + (button & 3);
    if (release || (button & 3) == 3) return 2003;
    return 2000 + (button & 3);
}

/* ------------------------------------------------------------------ */
/* Unicode display width, clipping and wrapping                        */
/* ------------------------------------------------------------------ */

static int64_t nova_ansi_length(const unsigned char *text, int64_t len, int64_t index) {
    if (index >= len || text[index] != 0x1b) return 0;
    if (index + 1 >= len) return 1;
    if (text[index + 1] == '[') {
        int64_t i = index + 2;
        while (i < len) {
            if (text[i] >= 0x40 && text[i] <= 0x7e) return i - index + 1;
            i += 1;
        }
        return len - index;
    }
    if (text[index + 1] == ']') {
        int64_t i = index + 2;
        while (i < len) {
            if (text[i] == 0x07) return i - index + 1;
            if (text[i] == 0x1b && i + 1 < len && text[i + 1] == '\\') return i - index + 2;
            i += 1;
        }
        return len - index;
    }
    return 2;
}

static int nova_utf8_decode(
    const unsigned char *text, int64_t len, int64_t index, uint32_t *cp, int64_t *next
) {
    if (index >= len) return 0;
    unsigned char first = text[index];
    if (first < 0x80) {
        *cp = first;
        *next = index + 1;
        return 1;
    }
    int extra = 0;
    uint32_t value = 0;
    if ((first & 0xe0) == 0xc0) {
        extra = 1;
        value = first & 0x1f;
    } else if ((first & 0xf0) == 0xe0) {
        extra = 2;
        value = first & 0x0f;
    } else if ((first & 0xf8) == 0xf0) {
        extra = 3;
        value = first & 0x07;
    } else {
        *cp = 0xfffd;
        *next = index + 1;
        return 1;
    }
    if (index + extra >= len) {
        *cp = 0xfffd;
        *next = len;
        return 1;
    }
    for (int i = 1; i <= extra; i++) {
        unsigned char byte = text[index + i];
        if ((byte & 0xc0) != 0x80) {
            *cp = 0xfffd;
            *next = index + 1;
            return 1;
        }
        value = (value << 6) | (byte & 0x3f);
    }
    if (value > 0x10ffff || (value >= 0xd800 && value <= 0xdfff)) {
        *cp = 0xfffd;
        *next = index + extra + 1;
        return 1;
    }
    *cp = value;
    *next = index + extra + 1;
    return 1;
}

static int nova_codepoint_width(uint32_t cp) {
    if (cp == 0) return 0;
    if (cp < 0x20 || (cp >= 0x7f && cp < 0xa0)) return 0;
    if ((cp >= 0x0300 && cp <= 0x036f) || (cp >= 0x1ab0 && cp <= 0x1aff) ||
        (cp >= 0x1dc0 && cp <= 0x1dff) || (cp >= 0x20d0 && cp <= 0x20ff) ||
        (cp >= 0xfe20 && cp <= 0xfe2f)) return 0;
    if ((cp >= 0x200b && cp <= 0x200f) || (cp >= 0x202a && cp <= 0x202e) ||
        (cp >= 0x2060 && cp <= 0x2064) || cp == 0xfeff ||
        (cp >= 0xfe00 && cp <= 0xfe0f) || (cp >= 0xe0100 && cp <= 0xe01ef)) return 0;
    if ((cp >= 0x1100 && cp <= 0x115f) || cp == 0x2329 || cp == 0x232a ||
        (cp >= 0x2e80 && cp <= 0x303e) || (cp >= 0x3041 && cp <= 0x33ff) ||
        (cp >= 0x3400 && cp <= 0x4dbf) || (cp >= 0x4e00 && cp <= 0x9fff) ||
        (cp >= 0xa000 && cp <= 0xa4cf) || (cp >= 0xa960 && cp <= 0xa97f) ||
        (cp >= 0xac00 && cp <= 0xd7a3) || (cp >= 0xf900 && cp <= 0xfaff) ||
        (cp >= 0xfe10 && cp <= 0xfe19) || (cp >= 0xfe30 && cp <= 0xfe6f) ||
        (cp >= 0xff00 && cp <= 0xff60) || (cp >= 0xffe0 && cp <= 0xffe6) ||
        (cp >= 0x1f300 && cp <= 0x1f64f) || (cp >= 0x1f900 && cp <= 0x1f9ff) ||
        (cp >= 0x1fa70 && cp <= 0x1faff) || (cp >= 0x20000 && cp <= 0x3fffd)) return 2;
    return 1;
}

/* Marks that extend the preceding scalar rather than starting a new cluster. */
static int nova_is_cluster_extend(uint32_t cp) {
    if ((cp >= 0x0300 && cp <= 0x036f) || (cp >= 0x1ab0 && cp <= 0x1aff) ||
        (cp >= 0x1dc0 && cp <= 0x1dff) || (cp >= 0x20d0 && cp <= 0x20ff) ||
        (cp >= 0xfe20 && cp <= 0xfe2f)) return 1;
    if ((cp >= 0xfe00 && cp <= 0xfe0f) || (cp >= 0xe0100 && cp <= 0xe01ef)) return 1;
    if (cp >= 0x1f3fb && cp <= 0x1f3ff) return 1; /* emoji skin tones */
    if (cp == 0x20e3) return 1;                   /* combining keycap */
    return 0;
}

static int nova_is_regional(uint32_t cp) {
    return cp >= 0x1f1e6 && cp <= 0x1f1ff;
}

/* Measures one grapheme cluster: a base scalar plus combining marks, variation
 * selectors, emoji modifiers, a regional-indicator pair and ZWJ sequences.
 * `*next` always advances by at least one scalar. */
static int nova_cluster_width(
    const unsigned char *text, int64_t len, int64_t index, int64_t *next
) {
    uint32_t cp = 0;
    int64_t pos = index;
    if (!nova_utf8_decode(text, len, index, &cp, &pos)) {
        *next = index < len ? index + 1 : len;
        return 0;
    }
    int width = nova_codepoint_width(cp);
    int regional = nova_is_regional(cp);
    int joined = 0;
    int64_t end = pos;
    if (regional) width = 2;
    while (end < len) {
        uint32_t part = 0;
        int64_t after = end;
        if (!nova_utf8_decode(text, len, end, &part, &after)) break;
        if (part == 0x200d) { /* zero-width joiner: the next scalar stays joined */
            joined = 1;
            end = after;
            continue;
        }
        if (nova_is_cluster_extend(part)) {
            if (part == 0xfe0f && width < 2) width = 2; /* emoji presentation */
            end = after;
            continue;
        }
        if (regional == 1 && nova_is_regional(part)) {
            regional = 2;
            width = 2;
            end = after;
            continue;
        }
        if (joined) {
            int part_width = nova_codepoint_width(part);
            if (part_width > width) width = part_width;
            if (width == 0) width = 1;
            joined = 0;
            end = after;
            continue;
        }
        break;
    }
    *next = end > index ? end : index + 1;
    return width;
}

int64_t nova_string_display_width(nova_str value) {
    int64_t width = 0;
    int64_t i = 0;
    while (i < value.len) {
        int64_t ansi = nova_ansi_length((const unsigned char *)value.ptr, value.len, i);
        if (ansi > 0) {
            i += ansi;
            continue;
        }
        int64_t next = i;
        width += nova_cluster_width((const unsigned char *)value.ptr, value.len, i, &next);
        i = next;
    }
    return width;
}

nova_str nova_string_clip(nova_str value, int64_t width) {
    if (width <= 0 || !value.ptr || value.len == 0) return nova_copy_str("", 0);
    char *buffer = (char *)nova_alloc(value.len + 8);
    int64_t out = 0;
    int64_t used = 0;
    int64_t i = 0;
    int saw_ansi = 0;
    while (i < value.len) {
        int64_t ansi = nova_ansi_length((const unsigned char *)value.ptr, value.len, i);
        if (ansi > 0) {
            memcpy(buffer + out, value.ptr + i, (size_t)ansi);
            out += ansi;
            i += ansi;
            saw_ansi = 1;
            continue;
        }
        int64_t next = i;
        int columns = nova_cluster_width((const unsigned char *)value.ptr, value.len, i, &next);
        if (used + columns > width) break;
        memcpy(buffer + out, value.ptr + i, (size_t)(next - i));
        out += next - i;
        used += columns;
        i = next;
    }
    if (saw_ansi) {
        memcpy(buffer + out, "\x1b[0m", 4);
        out += 4;
    }
    buffer[out] = '\0';
    nova_str result;
    result.ptr = buffer;
    result.len = out;
    return result;
}

static void nova_wrap_push(nova_list *list, nova_str line) {
    nova_list_push(list, &line, (int64_t)sizeof(nova_str));
}

void nova_string_wrap(nova_str value, int64_t width, nova_list *result) {
    result->data = NULL;
    result->len = 0;
    result->cap = 0;
    if (width < 1) width = 1;
    if (!value.ptr || value.len == 0) {
        nova_wrap_push(result, nova_copy_str("", 0));
        return;
    }

    int64_t line_start = 0;
    int64_t i = 0;
    int64_t line_width = 0;
    int64_t last_space = -1;
    while (i < value.len) {
        int64_t ansi = nova_ansi_length((const unsigned char *)value.ptr, value.len, i);
        if (ansi > 0) {
            i += ansi;
            continue;
        }
        uint32_t cp = 0;
        int64_t scalar_end = i;
        if (!nova_utf8_decode((const unsigned char *)value.ptr, value.len, i, &cp, &scalar_end)) break;
        int64_t next = i;
        int columns = nova_cluster_width((const unsigned char *)value.ptr, value.len, i, &next);
        if (line_width + columns > width && i > line_start) {
            int64_t break_at = i;
            if (cp != ' ' && last_space > line_start) break_at = last_space;
            int64_t end = break_at;
            while (end > line_start && value.ptr[end - 1] == ' ') end -= 1;
            nova_wrap_push(result, nova_copy_str(value.ptr + line_start, end - line_start));
            line_start = break_at;
            while (line_start < value.len && value.ptr[line_start] == ' ') line_start += 1;
            line_width = 0;
            last_space = -1;
            i = line_start;
            continue;
        }
        line_width += columns;
        if (cp == ' ') last_space = next;
        i = next;
    }
    int64_t end = value.len;
    while (end > line_start && value.ptr[end - 1] == ' ') end -= 1;
    if (end > line_start || result->len == 0) {
        nova_wrap_push(result, nova_copy_str(value.ptr + line_start, end - line_start));
    }
}

/* Splits text into extended grapheme clusters. ANSI escape sequences stay
 * attached to the cluster they precede (or the last cluster when trailing), so
 * styled runs survive the split. */
void nova_string_graphemes(nova_str value, nova_list *result) {
    result->data = NULL;
    result->len = 0;
    result->cap = 0;
    if (!value.ptr || value.len == 0) return;
    int64_t i = 0;
    int64_t start = 0;
    int64_t last_start = -1;
    while (i < value.len) {
        int64_t ansi = nova_ansi_length((const unsigned char *)value.ptr, value.len, i);
        if (ansi > 0) {
            i += ansi;
            continue;
        }
        int64_t next = i;
        nova_cluster_width((const unsigned char *)value.ptr, value.len, i, &next);
        if (next <= i) next = i + 1;
        nova_wrap_push(result, nova_copy_str(value.ptr + start, next - start));
        last_start = start;
        start = next;
        i = next;
    }
    if (start < value.len) {
        if (result->len > 0) {
            nova_str *items = (nova_str *)result->data;
            items[result->len - 1].len = value.len - last_start;
        } else {
            nova_wrap_push(result, nova_copy_str(value.ptr + start, value.len - start));
        }
    }
}

/* Splits text into raw Unicode scalars. Unlike nova_string_graphemes this keeps
 * ANSI escapes as their individual bytes, which a terminal emulator needs: it
 * is parsing the escape, not measuring styled text. */
void nova_string_chars(nova_str value, nova_list *result) {
    result->data = NULL;
    result->len = 0;
    result->cap = 0;
    if (!value.ptr || value.len == 0) return;
    int64_t i = 0;
    while (i < value.len) {
        uint32_t cp = 0;
        int64_t next = i;
        if (!nova_utf8_decode((const unsigned char *)value.ptr, value.len, i, &cp, &next)) {
            next = i + 1;
        }
        if (next <= i) next = i + 1;
        nova_wrap_push(result, nova_copy_str(value.ptr + i, next - i));
        i = next;
    }
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

int32_t nova_string_contains(nova_str haystack, nova_str needle) {
    if (needle.len == 0) return 1;
    if (!haystack.ptr || !needle.ptr) return 0;
    if (haystack.len < needle.len) return 0;
    for (int64_t i = 0; i + needle.len <= haystack.len; i++) {
        if (memcmp(haystack.ptr + i, needle.ptr, (size_t)needle.len) == 0) return 1;
    }
    return 0;
}

int32_t nova_string_starts_with(nova_str value, nova_str prefix) {
    if (prefix.len == 0) return 1;
    if (!value.ptr || !prefix.ptr || value.len < prefix.len) return 0;
    return memcmp(value.ptr, prefix.ptr, (size_t)prefix.len) == 0;
}

int32_t nova_string_ends_with(nova_str value, nova_str suffix) {
    if (suffix.len == 0) return 1;
    if (!value.ptr || !suffix.ptr || value.len < suffix.len) return 0;
    return memcmp(value.ptr + (value.len - suffix.len), suffix.ptr, (size_t)suffix.len) == 0;
}

/* Trims ASCII whitespace. UTF-8 bytes are copied verbatim, so multibyte text
 * is never split by the trim itself. */
static int nova_ascii_space(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == 0x0b || c == 0x0c;
}

nova_str nova_string_trim(nova_str value) {
    int64_t start = 0;
    int64_t end = value.len;
    if (!value.ptr || value.len == 0) return nova_copy_str("", 0);
    while (start < end && nova_ascii_space((unsigned char)value.ptr[start])) start++;
    while (end > start && nova_ascii_space((unsigned char)value.ptr[end - 1])) end--;
    return nova_copy_str(value.ptr + start, end - start);
}

nova_str nova_string_replace(nova_str text, nova_str from, nova_str to) {
    if (from.len <= 0 || text.len <= 0) return nova_copy_str(text.ptr ? text.ptr : "", text.len);
    char *needle = (char *)nova_alloc(from.len + 1);
    memcpy(needle, from.ptr, (size_t)from.len);
    needle[from.len] = '\0';
    char *body = (char *)nova_alloc(text.len + 1);
    memcpy(body, text.ptr, (size_t)text.len);
    body[text.len] = '\0';

    int64_t cap = text.len + 1;
    int64_t len = 0;
    char *out = (char *)nova_alloc(cap);
    char *cursor = body;
    for (;;) {
        char *hit = strstr(cursor, needle);
        int64_t chunk = hit ? (int64_t)(hit - cursor) : (int64_t)strlen(cursor);
        while (len + chunk + to.len + 1 > cap) cap *= 2;
        out = (char *)realloc(out, (size_t)cap);
        if (!out) nova_die_oom();
        if (chunk > 0) memcpy(out + len, cursor, (size_t)chunk);
        len += chunk;
        if (!hit) break;
        if (to.len > 0) memcpy(out + len, to.ptr, (size_t)to.len);
        len += to.len;
        cursor = hit + from.len;
    }
    free(needle);
    free(body);
    nova_str result = nova_copy_str(out, len);
    free(out);
    return result;
}

nova_str nova_string_lower(nova_str value) {
    nova_str result = nova_copy_str(value.ptr ? value.ptr : "", value.len);
    char *out = (char *)result.ptr;
    for (int64_t i = 0; i < result.len; i++) {
        unsigned char c = (unsigned char)out[i];
        if (c >= 'A' && c <= 'Z') out[i] = (char)(c + 32);
    }
    return result;
}

nova_str nova_codepoint_to_string(int64_t code) {
    char buf[4];
    int n = 0;
    uint32_t cp = (uint32_t)code;
    if (code < 0 || code > 0x10FFFF || (cp >= 0xD800 && cp <= 0xDFFF)) cp = 0xFFFD;
    if (cp < 0x80) {
        buf[n++] = (char)cp;
    } else if (cp < 0x800) {
        buf[n++] = (char)(0xC0 | (cp >> 6));
        buf[n++] = (char)(0x80 | (cp & 0x3F));
    } else if (cp < 0x10000) {
        buf[n++] = (char)(0xE0 | (cp >> 12));
        buf[n++] = (char)(0x80 | ((cp >> 6) & 0x3F));
        buf[n++] = (char)(0x80 | (cp & 0x3F));
    } else {
        buf[n++] = (char)(0xF0 | (cp >> 18));
        buf[n++] = (char)(0x80 | ((cp >> 12) & 0x3F));
        buf[n++] = (char)(0x80 | ((cp >> 6) & 0x3F));
        buf[n++] = (char)(0x80 | (cp & 0x3F));
    }
    return nova_copy_str(buf, n);
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

/* ------------------------------------------------------------------ */
/* Hashed maps and sets                                                */
/* ------------------------------------------------------------------ */

typedef struct {
    void *slots;
    unsigned char *state;
    int64_t len;
    int64_t cap;
    int64_t entry_size;
    int32_t key_kind;
} nova_map;

static uint64_t nova_hash_bytes(const unsigned char *data, int64_t len) {
    uint64_t hash = 1469598103934665603ULL;
    for (int64_t i = 0; i < len; i++) {
        hash ^= data[i];
        hash *= 1099511628211ULL;
    }
    return hash;
}

/* key_kind: 0 = Int, 1 = Bool, 2 = Char, 3 = String. */
static uint64_t nova_hash_key(const void *key, int32_t kind) {
    if (kind == 3) {
        nova_str s;
        memcpy(&s, key, sizeof(s));
        return nova_hash_bytes((const unsigned char *)s.ptr, s.len);
    }
    if (kind == 0) return nova_hash_bytes((const unsigned char *)key, 8);
    if (kind == 1) return nova_hash_bytes((const unsigned char *)key, 1);
    return nova_hash_bytes((const unsigned char *)key, 4);
}

static int nova_key_eq(const void *left, const void *right, int32_t kind) {
    if (kind == 3) {
        nova_str a, b;
        memcpy(&a, left, sizeof(a));
        memcpy(&b, right, sizeof(b));
        return a.len == b.len &&
               (a.len == 0 || memcmp(a.ptr, b.ptr, (size_t)a.len) == 0);
    }
    int64_t size = kind == 0 ? 8 : (kind == 1 ? 1 : 4);
    return memcmp(left, right, (size_t)size) == 0;
}

static void nova_map_resize(nova_map *map, int64_t new_cap) {
    void *old_slots = map->slots;
    unsigned char *old_state = map->state;
    int64_t old_cap = map->cap;
    map->slots = nova_alloc(new_cap * map->entry_size);
    map->state = (unsigned char *)nova_alloc(new_cap);
    memset(map->state, 0, (size_t)new_cap);
    map->cap = new_cap;
    for (int64_t i = 0; i < old_cap; i++) {
        if (!old_state[i]) continue;
        void *entry = (char *)old_slots + (size_t)i * (size_t)map->entry_size;
        uint64_t hash = nova_hash_key(entry, map->key_kind);
        int64_t mask = new_cap - 1;
        int64_t index = (int64_t)(hash & (uint64_t)mask);
        while (map->state[index]) index = (index + 1) & mask;
        map->state[index] = 1;
        memcpy((char *)map->slots + (size_t)index * (size_t)map->entry_size,
               entry, (size_t)map->entry_size);
    }
    free(old_slots);
    free(old_state);
}

static int64_t nova_map_find(const nova_map *map, const void *key) {
    if (map->cap == 0) return -1;
    uint64_t hash = nova_hash_key(key, map->key_kind);
    int64_t mask = map->cap - 1;
    int64_t index = (int64_t)(hash & (uint64_t)mask);
    for (;;) {
        if (!map->state[index]) return -1;
        void *entry = (char *)map->slots + (size_t)index * (size_t)map->entry_size;
        if (nova_key_eq(entry, key, map->key_kind)) return index;
        index = (index + 1) & mask;
    }
}

static int nova_map_put(nova_map *map, const void *entry) {
    int64_t existing = nova_map_find(map, entry);
    if (existing >= 0) {
        memcpy((char *)map->slots + (size_t)existing * (size_t)map->entry_size,
               entry, (size_t)map->entry_size);
        return 0;
    }
    if (map->cap == 0 || (map->len + 1) * 10 >= map->cap * 7) {
        nova_map_resize(map, map->cap == 0 ? 8 : map->cap * 2);
    }
    uint64_t hash = nova_hash_key(entry, map->key_kind);
    int64_t mask = map->cap - 1;
    int64_t index = (int64_t)(hash & (uint64_t)mask);
    while (map->state[index]) index = (index + 1) & mask;
    map->state[index] = 1;
    memcpy((char *)map->slots + (size_t)index * (size_t)map->entry_size,
           entry, (size_t)map->entry_size);
    map->len += 1;
    return 1;
}

void nova_map_insert(nova_map *map, void *entry) {
    (void)nova_map_put(map, entry);
}

int32_t nova_map_add(nova_map *map, void *entry) {
    return nova_map_put(map, entry);
}

void *nova_map_get(nova_map *map, void *key) {
    int64_t index = nova_map_find(map, key);
    if (index < 0) return NULL;
    return (char *)map->slots + (size_t)index * (size_t)map->entry_size;
}

int64_t nova_map_len(nova_map *map) {
    return map->len;
}

nova_str nova_string_trim_start(nova_str value) {
    if (!value.ptr || value.len == 0) return nova_copy_str("", 0);
    int64_t start = 0;
    while (start < value.len && nova_ascii_space((unsigned char)value.ptr[start])) start++;
    return nova_copy_str(value.ptr + start, value.len - start);
}

nova_str nova_string_trim_end(nova_str value) {
    if (!value.ptr || value.len == 0) return nova_copy_str("", 0);
    int64_t end = value.len;
    while (end > 0 && nova_ascii_space((unsigned char)value.ptr[end - 1])) end--;
    return nova_copy_str(value.ptr, end);
}

/* Splits on a byte separator, always producing at least one element, the same
 * way split works in every language a Nova program is likely to emulate. */
void nova_string_split(nova_str value, nova_str separator, nova_list *result) {
    result->data = NULL;
    result->len = 0;
    result->cap = 0;
    if (!value.ptr) {
        nova_wrap_push(result, nova_copy_str("", 0));
        return;
    }
    if (separator.len <= 0) {
        nova_wrap_push(result, nova_copy_str(value.ptr, value.len));
        return;
    }
    int64_t start = 0;
    int64_t i = 0;
    while (i + separator.len <= value.len) {
        if (memcmp(value.ptr + i, separator.ptr, (size_t)separator.len) == 0) {
            nova_wrap_push(result, nova_copy_str(value.ptr + start, i - start));
            i += separator.len;
            start = i;
        } else {
            i++;
        }
    }
    nova_wrap_push(result, nova_copy_str(value.ptr + start, value.len - start));
}

/* Joins a List<String> with a separator, sizing the buffer once. */
nova_str nova_string_join(nova_list *list_ptr, nova_str separator) {
    if (!list_ptr) return nova_copy_str("", 0);
    nova_list list = *list_ptr;
    int64_t count = list.len;
    if (count <= 0 || !list.data) return nova_copy_str("", 0);
    nova_str *items = (nova_str *)list.data;
    int64_t total = separator.len * (count - 1);
    for (int64_t i = 0; i < count; i++) total += items[i].len;
    char *buffer = (char *)nova_alloc(total + 1);
    if (!buffer) nova_die_oom();
    int64_t at = 0;
    for (int64_t i = 0; i < count; i++) {
        if (i > 0 && separator.len > 0) {
            memcpy(buffer + at, separator.ptr, (size_t)separator.len);
            at += separator.len;
        }
        if (items[i].len > 0 && items[i].ptr) {
            memcpy(buffer + at, items[i].ptr, (size_t)items[i].len);
            at += items[i].len;
        }
    }
    buffer[at] = '\0';
    nova_str out = nova_copy_str(buffer, at);
    free(buffer);
    return out;
}
