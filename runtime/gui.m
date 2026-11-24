/*
 * nova-gui — a native window, immediate-mode rasterizer and input queue.
 *
 * macOS implementation (AppKit + CoreGraphics + CoreText). Nova has no
 * callbacks across the FFI boundary, so this follows the same shape as the
 * terminal bindings: the host owns a window and an event queue, and a Nova
 * `while` loop drains events, draws a frame and calls nova_gui_present().
 *
 * Coordinates are in points with the origin at the top-left, so drawing code
 * reads like CSS or a canvas. The backing bitmap is device-sized (2x on a
 * Retina display) and every draw is scaled by the same factor, which keeps
 * text crisp without the caller knowing about pixel ratios.
 *
 * Only macOS is implemented. The build compiles this file only when a program
 * references nova_gui_ symbols, so non-GUI Nova programs never link AppKit.
 */

#import <Cocoa/Cocoa.h>
#import <CoreText/CoreText.h>
#import <ImageIO/ImageIO.h>
#include <pthread.h>
#include <errno.h>
#include <fcntl.h>
#include <termios.h>
#include <time.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <signal.h>
#import <QuartzCore/QuartzCore.h>

#include <math.h>
#include <stdint.h>
#include <stdarg.h>
#include <string.h>
#include <dirent.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

/* Matches the runtime ABI: { const char *ptr; int64_t len; } UTF-8 bytes. */
typedef struct {
    const char *ptr;
    int64_t len;
} nova_str;

/* Input events. Printable keys report their Unicode scalar (32..0x10FFFF);
 * everything else uses these codes, which match nova-tui where they overlap. */
#define NOVA_GUI_NONE 0
#define NOVA_GUI_KEY 1
#define NOVA_GUI_MOUSE_DOWN 2
#define NOVA_GUI_MOUSE_UP 3
#define NOVA_GUI_MOUSE_MOVE 4
#define NOVA_GUI_DRAG 5
#define NOVA_GUI_SCROLL 6
#define NOVA_GUI_RESIZE 7
#define NOVA_GUI_CLOSE 8
#define NOVA_GUI_REPAINT 9
#define NOVA_GUI_KEY_UP 10

#define NOVA_KEY_ENTER 13
#define NOVA_KEY_TAB 9
#define NOVA_KEY_ESCAPE 27
#define NOVA_KEY_BACKSPACE 127
#define NOVA_KEY_DELETE 1010
#define NOVA_KEY_LEFT 1004
#define NOVA_KEY_RIGHT 1003
#define NOVA_KEY_UP 1001
#define NOVA_KEY_DOWN 1002
#define NOVA_KEY_HOME 1005
#define NOVA_KEY_END 1006
#define NOVA_KEY_PAGE_UP 1007
#define NOVA_KEY_PAGE_DOWN 1008
#define NOVA_KEY_F12 1009

/* Modifier bits: 1 command, 2 shift, 4 option, 8 control. */
#define NOVA_MOD_COMMAND 1
#define NOVA_MOD_SHIFT 2
#define NOVA_MOD_OPTION 4
#define NOVA_MOD_CONTROL 8

typedef struct {
    int64_t kind;
    int64_t code;
    int64_t x;
    int64_t y;
    int64_t mods;
    int64_t delta;
} NovaGuiEvent;

#define NOVA_GUI_QUEUE 1024

static NovaGuiEvent nova_queue[NOVA_GUI_QUEUE];
static int nova_q_head = 0;
static int nova_q_tail = 0;

static CGContextRef nova_ctx = NULL;
static CGImageRef nova_image = NULL;
static int64_t nova_w = 0;
static int64_t nova_h = 0;
static int64_t nova_scale = 1;
static int nova_clip_depth = 0;
static int nova_running = 0;
static NSWindow *nova_window = nil;
static NSView *nova_view = nil;
static NSMutableString *nova_title = nil;

/* Opt-in event trace: NOVA_GUI_TRACE=/path appends one line per input and
 * activation change. Headless checks read it to tell "the click never arrived"
 * apart from "the click arrived and the UI ignored it". */
static void nova_gui_trace(const char *format, ...) {
    static FILE *log = NULL;
    static int tried = 0;
    if (!tried) {
        tried = 1;
        const char *path = getenv("NOVA_GUI_TRACE");
        if (path && *path) log = fopen(path, "a");
    }
    if (!log) return;
    va_list args;
    va_start(args, format);
    vfprintf(log, format, args);
    va_end(args);
    fputc('\n', log);
    fflush(log);
}

/* Brings the app forward and makes the window key. A window launched from a
 * shell script does not become active on its own, and macOS tints inactive
 * windows, which reads as a greyed-out, frozen application. */
static void nova_gui_activate_window(void) {
    if (!nova_window) return;
    [nova_window makeKeyAndOrderFront:nil];
    /* makeKeyAndOrderFront only raises the window within its own app when the
     * app is not yet active, so another app's full-size window can stay on top
     * and hide the whole UI. This raises it above every normal window. */
    [nova_window orderFrontRegardless];
    [nova_window makeKeyWindow];
    if (nova_view) {
        BOOL made = [nova_window makeFirstResponder:nova_view];
        nova_gui_trace("firstResponder made=%d responder=%s", (int)made,
                       [[[nova_window firstResponder] className] UTF8String]);
    }
    if ([NSApp respondsToSelector:@selector(activate)]) {
        [NSApp activate];
    }
    /* Still honoured on current macOS, and the only call that pulls a window
     * forward when the launch was not user-initiated. */
    [NSApp activateIgnoringOtherApps:YES];
}

static void nova_gui_emit(int64_t kind, int64_t code, int64_t x, int64_t y, int64_t mods, int64_t delta) {
    int next = (nova_q_tail + 1) % NOVA_GUI_QUEUE;
    if (next == nova_q_head) return; /* queue full: drop the newest event */
    NovaGuiEvent *event = &nova_queue[nova_q_tail];
    event->kind = kind;
    event->code = code;
    event->x = x;
    event->y = y;
    event->mods = mods;
    event->delta = delta;
    nova_q_tail = next;
}

static int64_t nova_gui_modifiers(NSEventModifierFlags flags) {
    int64_t mods = 0;
    if (flags & NSEventModifierFlagCommand) mods |= NOVA_MOD_COMMAND;
    if (flags & NSEventModifierFlagShift) mods |= NOVA_MOD_SHIFT;
    if (flags & NSEventModifierFlagOption) mods |= NOVA_MOD_OPTION;
    if (flags & NSEventModifierFlagControl) mods |= NOVA_MOD_CONTROL;
    return mods;
}

@interface NovaGuiView : NSView
@end

@implementation NovaGuiView

- (BOOL)acceptsFirstResponder {
    return YES;
}

- (BOOL)isOpaque {
    return YES;
}

- (void)drawRect:(NSRect)dirtyRect {
    (void)dirtyRect;
    CGContextRef context = [[NSGraphicsContext currentContext] CGContext];
    if (!context) return;
    if (nova_image) {
        CGContextSaveGState(context);
        CGContextSetInterpolationQuality(context, kCGInterpolationLow);
        CGContextDrawImage(context, self.bounds, nova_image);
        CGContextRestoreGState(context);
    }
}

/* View coordinates are bottom-left; the drawing API is top-left. */
- (NSPoint)nova_point:(NSEvent *)event {
    NSPoint local = [self convertPoint:event.locationInWindow fromView:nil];
    return NSMakePoint(local.x, (CGFloat)nova_h - local.y);
}

/* Maps an AppKit key event to the Nova key vocabulary: named keys get the
 * codes from nova-tui, printable characters keep their Unicode scalar. */
static int64_t nova_gui_key_code(NSEvent *event, int64_t mods) {
    switch (event.keyCode) {
        case 36: return NOVA_KEY_ENTER;
        case 48: return NOVA_KEY_TAB;
        case 53: return NOVA_KEY_ESCAPE;
        case 51: return NOVA_KEY_BACKSPACE;
        case 117: return NOVA_KEY_DELETE;
        case 123: return NOVA_KEY_LEFT;
        case 124: return NOVA_KEY_RIGHT;
        case 126: return NOVA_KEY_UP;
        case 125: return NOVA_KEY_DOWN;
        case 115: return NOVA_KEY_HOME;
        case 119: return NOVA_KEY_END;
        case 116: return NOVA_KEY_PAGE_UP;
        case 121: return NOVA_KEY_PAGE_DOWN;
        case 111: return NOVA_KEY_F12;
        default: break;
    }
    NSString *text = (mods & NOVA_MOD_COMMAND) ? event.charactersIgnoringModifiers : event.characters;
    if (text.length == 0) return 0;
    unichar first = [text characterAtIndex:0];
    if (first >= 0xD800 && first <= 0xDBFF && text.length > 1) {
        unichar low = [text characterAtIndex:1];
        return 0x10000 + ((int64_t)(first - 0xD800) << 10) + (int64_t)(low - 0xDC00);
    }
    return first;
}

- (void)keyDown:(NSEvent *)event {
    int64_t mods = nova_gui_modifiers(event.modifierFlags);
    int64_t code = nova_gui_key_code(event, mods);
    nova_gui_trace("keyDown keyCode=%d mods=%lld key=%d repeat=%d active=%d",
                   (int)event.keyCode, mods, (int)nova_window.isKeyWindow,
                   (int)event.isARepeat, (int)NSApp.isActive);
    nova_gui_emit(NOVA_GUI_KEY, code, 0, 0, mods, 0);
}

/* Key releases let a program distinguish "held" from "tapped": the arrows are
 * sampled every tick while held, instead of relying on the OS repeat delay. */
- (void)keyUp:(NSEvent *)event {
    int64_t mods = nova_gui_modifiers(event.modifierFlags);
    int64_t code = nova_gui_key_code(event, mods);
    nova_gui_trace("keyUp keyCode=%d code=%lld", (int)event.keyCode, code);
    nova_gui_emit(NOVA_GUI_KEY_UP, code, 0, 0, mods, 0);
}

- (void)flagsChanged:(NSEvent *)event {
    /* Keep the modifier state fresh so a later click reports shift/command. */
    nova_gui_emit(NOVA_GUI_NONE, 0, 0, 0, nova_gui_modifiers(event.modifierFlags), 0);
}

/* Without this the click that activates an inactive window is swallowed by
 * AppKit: the app comes forward but the click is never delivered, so the first
 * click on every control appears to do nothing. */
- (BOOL)acceptsFirstMouse:(NSEvent *)event {
    (void)event;
    return YES;
}

- (void)mouseDown:(NSEvent *)event {
    /* The first click on an inactive window has to pull the app forward. */
    nova_gui_activate_window();
    NSPoint point = [self nova_point:event];
    nova_gui_emit(NOVA_GUI_MOUSE_DOWN, 0, (int64_t)point.x, (int64_t)point.y, nova_gui_modifiers(event.modifierFlags), 1);
    nova_gui_trace("mouseDown point=%.0f,%.0f frame=%dx%d viewH=%.0f key=%d active=%d",
                   point.x, point.y, (int)nova_w, (int)nova_h, self.bounds.size.height,
                   (int)nova_window.isKeyWindow, (int)NSApp.isActive);
    /* The window hides the system title bar and draws its own 35px strip, so
     * dragging there has to be done by hand. The traffic lights own the top
     * left corner and get their clicks from the window server, not from here. */
    if (point.y < 35 && !(point.x < 76 && point.y < 28)) {
        [nova_window performWindowDragWithEvent:event];
        return;
    }
}

- (void)mouseUp:(NSEvent *)event {
    NSPoint point = [self nova_point:event];
    nova_gui_emit(NOVA_GUI_MOUSE_UP, 0, (int64_t)point.x, (int64_t)point.y, nova_gui_modifiers(event.modifierFlags), 1);
}

- (void)rightMouseDown:(NSEvent *)event {
    NSPoint point = [self nova_point:event];
    nova_gui_emit(NOVA_GUI_MOUSE_DOWN, 0, (int64_t)point.x, (int64_t)point.y, nova_gui_modifiers(event.modifierFlags), 2);
}

- (void)mouseMoved:(NSEvent *)event {
    NSPoint point = [self nova_point:event];
    nova_gui_emit(NOVA_GUI_MOUSE_MOVE, 0, (int64_t)point.x, (int64_t)point.y, nova_gui_modifiers(event.modifierFlags), 0);
}

- (void)mouseDragged:(NSEvent *)event {
    NSPoint point = [self nova_point:event];
    nova_gui_emit(NOVA_GUI_DRAG, 0, (int64_t)point.x, (int64_t)point.y, nova_gui_modifiers(event.modifierFlags), 1);
}

- (void)rightMouseDragged:(NSEvent *)event {
    [self mouseDragged:event];
}

- (void)scrollWheel:(NSEvent *)event {
    NSPoint point = [self nova_point:event];
    CGFloat delta = event.hasPreciseScrollingDeltas ? event.scrollingDeltaY : event.scrollingDeltaY * 10.0;
    if (delta == 0.0) return;
    nova_gui_emit(NOVA_GUI_SCROLL, 0, (int64_t)point.x, (int64_t)point.y, nova_gui_modifiers(event.modifierFlags), (int64_t)lround(delta));
}

- (void)updateTrackingAreas {
    for (NSTrackingArea *area in [self trackingAreas]) {
        [self removeTrackingArea:area];
    }
    NSTrackingAreaOptions options = NSTrackingMouseMoved | NSTrackingMouseEnteredAndExited |
                                    NSTrackingActiveInKeyWindow | NSTrackingInVisibleRect;
    NSTrackingArea *area = [[NSTrackingArea alloc] initWithRect:[self bounds]
                                                        options:options
                                                          owner:self
                                                       userInfo:nil];
    [self addTrackingArea:area];
}

@end

@interface NovaGuiDelegate : NSObject <NSWindowDelegate, NSApplicationDelegate>
@end

@implementation NovaGuiDelegate

- (BOOL)windowShouldClose:(NSWindow *)sender {
    (void)sender;
    nova_running = 0;
    nova_gui_emit(NOVA_GUI_CLOSE, 0, 0, 0, 0, 0);
    return NO;
}

/* Activation arrives asynchronously after `activate`, so the window has to be
 * made key when it lands. An active app whose window is not key receives mouse
 * events but no key events at all — which looks like a frozen editor. */
- (void)applicationDidBecomeActive:(NSNotification *)notification {
    (void)notification;
    nova_gui_trace("applicationDidBecomeActive window=%p", (__bridge void *)nova_window);
    if (nova_window) {
        [nova_window makeKeyAndOrderFront:nil];
        if (nova_view) [nova_window makeFirstResponder:nova_view];
    }
    nova_gui_emit(NOVA_GUI_REPAINT, 0, 0, 0, 0, 0);
}

- (void)windowDidBecomeKey:(NSNotification *)notification {
    (void)notification;
    nova_gui_trace("windowDidBecomeKey");
    nova_gui_emit(NOVA_GUI_REPAINT, 0, 0, 0, 0, 0);
}

/* Clicking the Dock icon, or opening an already running app, arrives as a
 * reopen. Without this the window stays behind whatever is in front, which
 * looks exactly like an app that ignores clicks. */
- (BOOL)applicationShouldHandleReopen:(NSApplication *)app hasVisibleWindows:(BOOL)visible {
    (void)app;
    nova_gui_trace("reopen visible=%d", (int)visible);
    if (nova_window) {
        if (!visible) [nova_window makeKeyAndOrderFront:nil];
        nova_gui_activate_window();
    }
    return YES;
}

- (void)windowDidResignKey:(NSNotification *)notification {
    (void)notification;
    nova_gui_trace("windowDidResignKey active=%d", (int)NSApp.isActive);
    nova_gui_emit(NOVA_GUI_REPAINT, 0, 0, 0, 0, 0);
}

- (void)windowDidResize:(NSNotification *)notification {
    (void)notification;
    NSSize size = nova_view ? nova_view.bounds.size : NSMakeSize(nova_w, nova_h);
    nova_gui_emit(NOVA_GUI_RESIZE, 0, (int64_t)size.width, (int64_t)size.height, 0, 0);
}

@end

static NovaGuiDelegate *nova_delegate = nil;

/* ------------------------------------------------------------------ */
/* Menu bar                                                           */
/* ------------------------------------------------------------------ */

/* A menu item enqueues the same key event the keyboard would produce, so a
 * command and its shortcut take exactly one path through the program. */
@interface NovaGuiMenuTarget : NSObject
@end

static NovaGuiMenuTarget *nova_menu_target = nil;

@implementation NovaGuiMenuTarget

- (void)menuCommand:(id)sender {
    int64_t tag = (int64_t)[(NSMenuItem *)sender tag];
    nova_gui_emit(NOVA_GUI_KEY, tag & 0xFFFF, 0, 0, (tag >> 16) & 0xF, 0);
}

- (void)openDocs:(id)sender {
    (void)sender;
    NSURL *url = [NSURL URLWithString:@"https://jd4rider.github.io/nova/"];
    if (url) [[NSWorkspace sharedWorkspace] openURL:url];
}

@end

static void nova_gui_menu_item(NSMenu *menu, NSString *title, NSString *key,
                               NSEventModifierFlags mask, int64_t mods, int64_t code) {
    NSMenuItem *item = [menu addItemWithTitle:title action:@selector(menuCommand:)
                                keyEquivalent:key ? key : @""];
    item.tag = (code & 0xFFFF) | ((mods & 0xF) << 16);
    item.keyEquivalentModifierMask = mask;
    item.target = nova_menu_target;
}

/* Without a menu bar the app looks like a stray window: macOS only gives a
 * process its own menu bar when the app is properly launched and activated. */
static void nova_gui_build_menu(void) {
    nova_menu_target = [[NovaGuiMenuTarget alloc] init];
    NSMenu *menubar = [[NSMenu alloc] init];
    NSEventModifierFlags command = NSEventModifierFlagCommand;

    NSMenuItem *app_item = [[NSMenuItem alloc] init];
    [menubar addItem:app_item];
    NSMenu *app = [[NSMenu alloc] initWithTitle:@"Nova Studio"];
    [app addItemWithTitle:@"About Nova Studio" action:@selector(orderFrontStandardAboutPanel:)
           keyEquivalent:@""];
    [app addItem:[NSMenuItem separatorItem]];
    nova_gui_menu_item(app, @"Settings…", @",", command, NOVA_MOD_COMMAND, ',');
    [app addItem:[NSMenuItem separatorItem]];
    [app addItemWithTitle:@"Hide Nova Studio" action:@selector(hide:) keyEquivalent:@"h"];
    NSMenuItem *hide_others = [app addItemWithTitle:@"Hide Others"
                                            action:@selector(hideOtherApplications:)
                                     keyEquivalent:@"h"];
    hide_others.keyEquivalentModifierMask = command | NSEventModifierFlagOption;
    [app addItemWithTitle:@"Show All" action:@selector(unhideAllApplications:) keyEquivalent:@""];
    [app addItem:[NSMenuItem separatorItem]];
    [app addItemWithTitle:@"Quit Nova Studio" action:@selector(terminate:) keyEquivalent:@"q"];
    app_item.submenu = app;

    NSMenuItem *file_item = [[NSMenuItem alloc] init];
    [menubar addItem:file_item];
    NSMenu *file = [[NSMenu alloc] initWithTitle:@"File"];
    nova_gui_menu_item(file, @"Open File…", @"o", command, NOVA_MOD_COMMAND, 'o');
    nova_gui_menu_item(file, @"Open Folder…", @"O", command | NSEventModifierFlagShift,
                       NOVA_MOD_COMMAND | NOVA_MOD_SHIFT, 'O');
    [file addItem:[NSMenuItem separatorItem]];
    nova_gui_menu_item(file, @"Go to File…", @"p", command, NOVA_MOD_COMMAND, 'p');
    nova_gui_menu_item(file, @"Close Tab", @"w", command, NOVA_MOD_COMMAND, 'w');
    [file addItem:[NSMenuItem separatorItem]];
    nova_gui_menu_item(file, @"Save", @"s", command, NOVA_MOD_COMMAND, 's');
    [file addItem:[NSMenuItem separatorItem]];
    nova_gui_menu_item(file, @"Build", @"b", command, NOVA_MOD_COMMAND, 'b');
    nova_gui_menu_item(file, @"Check", @"B", command | NSEventModifierFlagShift,
                       NOVA_MOD_COMMAND | NOVA_MOD_SHIFT, 'b');
    nova_gui_menu_item(file, @"Build and Run in Terminal", @"r", command, NOVA_MOD_COMMAND, 'r');
    file_item.submenu = file;

    NSMenuItem *edit_item = [[NSMenuItem alloc] init];
    [menubar addItem:edit_item];
    NSMenu *edit = [[NSMenu alloc] initWithTitle:@"Edit"];
    nova_gui_menu_item(edit, @"Undo", @"z", command, NOVA_MOD_COMMAND, 'z');
    nova_gui_menu_item(edit, @"Redo", @"Z", command | NSEventModifierFlagShift,
                       NOVA_MOD_COMMAND | NOVA_MOD_SHIFT, 'z');
    [edit addItem:[NSMenuItem separatorItem]];
    nova_gui_menu_item(edit, @"Cut", @"x", command, NOVA_MOD_COMMAND, 'x');
    nova_gui_menu_item(edit, @"Copy", @"c", command, NOVA_MOD_COMMAND, 'c');
    nova_gui_menu_item(edit, @"Paste", @"v", command, NOVA_MOD_COMMAND, 'v');
    nova_gui_menu_item(edit, @"Select All", @"a", command, NOVA_MOD_COMMAND, 'a');
    [edit addItem:[NSMenuItem separatorItem]];
    nova_gui_menu_item(edit, @"Find", @"f", command, NOVA_MOD_COMMAND, 'f');
    edit_item.submenu = edit;

    NSMenuItem *view_item = [[NSMenuItem alloc] init];
    [menubar addItem:view_item];
    NSMenu *view = [[NSMenu alloc] initWithTitle:@"View"];
    nova_gui_menu_item(view, @"Command Palette…", @"P", command | NSEventModifierFlagShift,
                       NOVA_MOD_COMMAND | NOVA_MOD_SHIFT, 'p');
    nova_gui_menu_item(view, @"Toggle Sidebar", @"E", command | NSEventModifierFlagShift,
                       NOVA_MOD_COMMAND | NOVA_MOD_SHIFT, 'e');
    nova_gui_menu_item(view, @"Toggle Panel", @"`", command, NOVA_MOD_COMMAND, '`');
    view_item.submenu = view;

    NSMenuItem *window_item = [[NSMenuItem alloc] init];
    [menubar addItem:window_item];
    NSMenu *window = [[NSMenu alloc] initWithTitle:@"Window"];
    [window addItemWithTitle:@"Minimize" action:@selector(performMiniaturize:) keyEquivalent:@"m"];
    [window addItemWithTitle:@"Zoom" action:@selector(performZoom:) keyEquivalent:@""];
    [window addItem:[NSMenuItem separatorItem]];
    NSMenuItem *full = [window addItemWithTitle:@"Enter Full Screen"
                                         action:@selector(toggleFullScreen:)
                                  keyEquivalent:@"f"];
    full.keyEquivalentModifierMask = command | NSEventModifierFlagControl;
    window_item.submenu = window;
    [NSApp setWindowsMenu:window];

    NSMenuItem *help_item = [[NSMenuItem alloc] init];
    [menubar addItem:help_item];
    NSMenu *help = [[NSMenu alloc] initWithTitle:@"Help"];
    NSMenuItem *docs = [help addItemWithTitle:@"Nova Documentation"
                                       action:@selector(openDocs:)
                                keyEquivalent:@""];
    docs.target = nova_menu_target;
    help_item.submenu = help;
    [NSApp setHelpMenu:help];

    [NSApp setMainMenu:menubar];
}

/* ------------------------------------------------------------------ */
/* Framebuffer                                                        */
/* ------------------------------------------------------------------ */

static void nova_gui_build_context(void) {
    if (nova_ctx) {
        CGContextRelease(nova_ctx);
        nova_ctx = NULL;
    }
    if (nova_image) {
        CGImageRelease(nova_image);
        nova_image = NULL;
    }
    if (nova_w <= 0 || nova_h <= 0) return;
    size_t pixels_w = (size_t)(nova_w * nova_scale);
    size_t pixels_h = (size_t)(nova_h * nova_scale);
    /* sRGB, not DeviceRGB. DeviceRGB is Generic RGB with a 1.8 gamma, so
     * compositing it into an sRGB window lifts every midtone: #1e1e1e arrives
     * as roughly #2c2c2c and the whole UI reads as washed out. */
    CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    nova_ctx = CGBitmapContextCreate(NULL, pixels_w, pixels_h, 8, 0, space,
                                     kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big);
    CGColorSpaceRelease(space);
    if (!nova_ctx) return;
    /* Top-left origin with y growing downward, expressed in points. */
    CGContextTranslateCTM(nova_ctx, 0, (CGFloat)pixels_h);
    CGContextScaleCTM(nova_ctx, (CGFloat)nova_scale, -(CGFloat)nova_scale);
    CGContextSetShouldAntialias(nova_ctx, true);
    CGContextSetLineWidth(nova_ctx, 1.0);
    CGContextSetTextMatrix(nova_ctx, CGAffineTransformMakeScale(1.0, -1.0));
}

/* Rebuild the bitmap when the window is resized, and re-clamp the clip stack. */
static void nova_gui_sync_size(void) {
    if (!nova_view) return;
    int64_t width = (int64_t)lround(nova_view.bounds.size.width);
    int64_t height = (int64_t)lround(nova_view.bounds.size.height);
    if (width <= 0 || height <= 0) return;
    if (width == nova_w && height == nova_h && nova_ctx) return;
    nova_w = width;
    nova_h = height;
    nova_clip_depth = 0;
    nova_gui_build_context();
}

static CGColorRef nova_gui_color(int64_t rgb) {
    CGFloat components[4] = {
        (CGFloat)((rgb >> 16) & 0xFF) / 255.0,
        (CGFloat)((rgb >> 8) & 0xFF) / 255.0,
        (CGFloat)(rgb & 0xFF) / 255.0,
        1.0,
    };
    CGColorSpaceRef space = CGColorSpaceCreateDeviceRGB();
    CGColorRef color = CGColorCreate(space, components);
    CGColorSpaceRelease(space);
    return color;
}

/* The monospaced family is a setting, not a constant: terminals want a Nerd
 * Font for its icons, editors may want Menlo. The cache lives at file scope so
 * changing the family can drop it. */
static char nova_mono_family[160] = {0};
static int nova_mono_ready = 0;
static CTFontRef nova_font_cache[2][80];

static void nova_mono_pick(void) {
    if (nova_mono_ready) return;
    nova_mono_ready = 1;
    const char *env = getenv("NOVA_MONO_FONT");
    if (env && env[0]) {
        snprintf(nova_mono_family, sizeof(nova_mono_family), "%s", env);
        return;
    }
    @autoreleasepool {
        NSString *best = nil;
        NSString *any_nerd = nil;
        for (NSString *name in [[NSFontManager sharedFontManager] availableFontFamilies]) {
            if ([name rangeOfString:@"Nerd Font"].location == NSNotFound) continue;
            if (!any_nerd) any_nerd = name;
            if (!best && [name rangeOfString:@"Mono"].location != NSNotFound) best = name;
        }
        NSString *chosen = best ? best : any_nerd;
        if (!chosen) chosen = @"Menlo";
        snprintf(nova_mono_family, sizeof(nova_mono_family), "%s", [chosen UTF8String]);
    }
}

static const char *nova_mono_name(void) {
    nova_mono_pick();
    return nova_mono_family;
}

static CTFontRef nova_gui_font(int64_t size, int64_t mono) {
    CTFontRef (*cache)[80] = nova_font_cache;
    int index = (int)(size < 1 ? 1 : (size > 79 ? 79 : size));
    int which = mono ? 1 : 0;
    if (!cache[which][index]) {
        CTFontRef font = NULL;
        if (mono) {
            CFStringRef family = CFStringCreateWithCString(NULL, nova_mono_name(), kCFStringEncodingUTF8);
            if (family) {
                font = CTFontCreateWithName(family, (CGFloat)index, NULL);
                CFRelease(family);
            }
        }
        if (!font) {
            font = CTFontCreateUIFontForLanguage(kCTFontUIFontSystem, (CGFloat)index, NULL);
        }
        if (!font) {
            font = CTFontCreateWithName(CFSTR("Helvetica"), (CGFloat)index, NULL);
        }
        cache[which][index] = font;
    }
    return cache[which][index];
}

static CTLineRef nova_gui_make_line(nova_str text, int64_t size, int64_t mono) {
    if (!text.ptr || text.len <= 0) return NULL;
    CFStringRef string = CFStringCreateWithBytes(NULL, (const UInt8 *)text.ptr, (CFIndex)text.len,
                                                 kCFStringEncodingUTF8, false);
    if (!string) return NULL;
    CTFontRef font = nova_gui_font(size, mono);
    /* Without this flag CTLineDraw ignores the fill colour set on the context
     * and paints the line black, which makes every label invisible on a dark
     * background. The line is cached per (text, size, font), so the colour has
     * to come from the context at draw time rather than from an attribute. */
    const void *keys[2] = { kCTFontAttributeName, kCTForegroundColorFromContextAttributeName };
    const void *values[2] = { font, kCFBooleanTrue };
    CFDictionaryRef attributes = CFDictionaryCreate(NULL, keys, values, 2,
                                                    &kCFTypeDictionaryKeyCallBacks,
                                                    &kCFTypeDictionaryValueCallBacks);
    CFAttributedStringRef attributed = CFAttributedStringCreate(NULL, string, attributes);
    CTLineRef line = CTLineCreateWithAttributedString(attributed);
    CFRelease(attributed);
    CFRelease(attributes);
    CFRelease(string);
    return line;
}

#include <stdint.h>
#include <string.h>

/* An immediate-mode loop redraws the same strings many times a second, and
 * building a CTLine (attribute dictionary, font lookup, shaping) costs far more
 * than drawing it. These are the most recently used lines, keyed by content. */
#define NOVA_LINE_CACHE 512

typedef struct {
    uint64_t hash;
    char *bytes;
    int64_t len;
    int64_t size;
    int64_t mono;
    CTLineRef line;
} NovaLineEntry;

static NovaLineEntry nova_lines[NOVA_LINE_CACHE];

static CTLineRef nova_gui_cached_line(nova_str text, int64_t size, int64_t mono) {
    if (!text.ptr || text.len <= 0 || text.len > 2048) return NULL;
    uint64_t hash = 1469598103934665603ULL;
    int64_t i = 0;
    while (i < text.len) {
        hash = (hash ^ (uint64_t)(unsigned char)text.ptr[i]) * 1099511628211ULL;
        i = i + 1;
    }
    hash = hash * 1099511628211ULL ^ (uint64_t)size;
    hash = hash * 1099511628211ULL ^ (uint64_t)mono;
    NovaLineEntry *entry = &nova_lines[hash % NOVA_LINE_CACHE];
    if (entry->line && entry->hash == hash && entry->size == size && entry->mono == mono &&
        entry->len == text.len && entry->bytes &&
        memcmp(entry->bytes, text.ptr, (size_t)text.len) == 0) {
        return (CTLineRef)CFRetain(entry->line);
    }
    CTLineRef line = nova_gui_make_line(text, size, mono);
    if (!line) return NULL;
    if (entry->line) CFRelease(entry->line);
    free(entry->bytes);
    entry->bytes = (char *)malloc((size_t)text.len);
    if (entry->bytes) memcpy(entry->bytes, text.ptr, (size_t)text.len);
    entry->len = entry->bytes ? text.len : 0;
    entry->size = size;
    entry->mono = mono;
    entry->hash = hash;
    entry->line = line;                /* the cache owns this reference */
    return (CTLineRef)CFRetain(line);  /* the caller owns its own */
}

/* ------------------------------------------------------------------ */
/* Public API                                                         */
/* ------------------------------------------------------------------ */

int64_t nova_gui_open(nova_str title, int64_t width, int64_t height) {
    @autoreleasepool {
        nova_title = [[NSMutableString alloc] initWithBytes:title.ptr ? title.ptr : ""
                                                     length:(NSUInteger)(title.len > 0 ? title.len : 0)
                                                   encoding:NSUTF8StringEncoding];
        if (!nova_title) nova_title = [NSMutableString stringWithString:@"Nova"];

        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        [NSApp finishLaunching];

        nova_gui_build_menu();

        NSRect frame = NSMakeRect(0, 0, (CGFloat)width, (CGFloat)height);
        NSWindowStyleMask style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                                  NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable |
                                  NSWindowStyleMaskFullSizeContentView;
        nova_window = [[NSWindow alloc] initWithContentRect:frame
                                                  styleMask:style
                                                    backing:NSBackingStoreBuffered
                                                      defer:NO];
        if (!nova_window) return 0;

        nova_window.title = nova_title;
        nova_window.titlebarAppearsTransparent = YES;
        nova_window.titleVisibility = NSWindowTitleHidden;
        nova_window.backgroundColor = [NSColor colorWithCalibratedRed:0.094 green:0.094 blue:0.094 alpha:1.0];
        /* Frame for the framebuffer image: sRGB, so 0x1e1e1e stays 0x1e1e1e. */
        nova_window.colorSpace = [NSColorSpace sRGBColorSpace];
        /* Opaque, fully opaque content: a window that composites with alpha
         * picks up whatever is behind it and reads as washed out. */
        nova_window.opaque = YES;
        nova_window.alphaValue = 1.0;
        nova_window.minSize = NSMakeSize(720, 480);

        nova_view = [[NovaGuiView alloc] initWithFrame:frame];
        nova_window.contentView = nova_view;
        nova_delegate = [[NovaGuiDelegate alloc] init];
        nova_window.delegate = nova_delegate;
        [NSApp setDelegate:nova_delegate];

        [nova_window center];
        /* Tracking areas report motion only when the window asks for it; without
         * this the app cannot highlight anything under the pointer. */
        [nova_window setAcceptsMouseMovedEvents:YES];
        [nova_window makeKeyAndOrderFront:nil];
        [nova_window makeFirstResponder:nova_view];
        nova_gui_activate_window();
        nova_gui_trace("open %dx%d key=%d active=%d", (int)nova_w, (int)nova_h,
                       (int)nova_window.isKeyWindow, (int)NSApp.isActive);
        /* One turn so the window is ordered and its first frame can commit. */
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
                                 beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];

        NSScreen *screen = [nova_window screen];
        nova_scale = screen ? (int64_t)lround(screen.backingScaleFactor) : 1;
        if (nova_scale < 1) nova_scale = 1;
        nova_w = (int64_t)lround(nova_view.bounds.size.width);
        nova_h = (int64_t)lround(nova_view.bounds.size.height);
        nova_gui_build_context();
        if (!nova_ctx) return 0;
        nova_running = 1;
        return 1;
    }
}

int64_t nova_gui_is_open(void) {
    return nova_running;
}

int64_t nova_gui_width(void) {
    return nova_w;
}

int64_t nova_gui_height(void) {
    return nova_h;
}

void nova_gui_begin(void) {
    nova_gui_sync_size();
    nova_clip_depth = 0;
    if (nova_ctx) {
        CGContextRestoreGState(nova_ctx);
        CGContextSaveGState(nova_ctx);
        CGContextSetTextMatrix(nova_ctx, CGAffineTransformMakeScale(1.0, -1.0));
        CGContextSetLineWidth(nova_ctx, 1.0);
    }
}

void nova_gui_clear(int64_t rgb) {
    if (!nova_ctx) return;
    CGColorRef color = nova_gui_color(rgb);
    CGContextSetFillColorWithColor(nova_ctx, color);
    CGContextFillRect(nova_ctx, CGRectMake(0, 0, (CGFloat)nova_w, (CGFloat)nova_h));
    CGColorRelease(color);
}

void nova_gui_rect(int64_t x, int64_t y, int64_t w, int64_t h, int64_t rgb) {
    if (!nova_ctx || w <= 0 || h <= 0) return;
    CGColorRef color = nova_gui_color(rgb);
    CGContextSetFillColorWithColor(nova_ctx, color);
    CGContextFillRect(nova_ctx, CGRectMake((CGFloat)x, (CGFloat)y, (CGFloat)w, (CGFloat)h));
    CGColorRelease(color);
}

void nova_gui_rect_alpha(int64_t x, int64_t y, int64_t w, int64_t h, int64_t rgb, int64_t alpha) {
    if (!nova_ctx || w <= 0 || h <= 0) return;
    CGFloat a = (CGFloat)(alpha < 0 ? 0 : (alpha > 255 ? 255 : alpha)) / 255.0;
    CGFloat components[4] = {
        (CGFloat)((rgb >> 16) & 0xFF) / 255.0,
        (CGFloat)((rgb >> 8) & 0xFF) / 255.0,
        (CGFloat)(rgb & 0xFF) / 255.0,
        a,
    };
    CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    CGColorRef color = CGColorCreate(space, components);
    CGColorSpaceRelease(space);
    CGContextSetFillColorWithColor(nova_ctx, color);
    CGContextFillRect(nova_ctx, CGRectMake((CGFloat)x, (CGFloat)y, (CGFloat)w, (CGFloat)h));
    CGColorRelease(color);
}

void nova_gui_round_rect(int64_t x, int64_t y, int64_t w, int64_t h, int64_t radius, int64_t rgb) {
    if (!nova_ctx || w <= 0 || h <= 0) return;
    CGFloat r = (CGFloat)(radius > 0 ? radius : 0);
    CGFloat limit = (CGFloat)(w < h ? w : h) / 2.0;
    if (r > limit) r = limit;
    CGRect rect = CGRectMake((CGFloat)x, (CGFloat)y, (CGFloat)w, (CGFloat)h);
    CGColorRef color = nova_gui_color(rgb);
    if (r <= 0) {
        CGContextSetFillColorWithColor(nova_ctx, color);
        CGContextFillRect(nova_ctx, rect);
    } else {
        CGPathRef path = CGPathCreateWithRoundedRect(rect, r, r, NULL);
        CGContextAddPath(nova_ctx, path);
        CGContextSetFillColorWithColor(nova_ctx, color);
        CGContextFillPath(nova_ctx);
        CGPathRelease(path);
    }
    CGColorRelease(color);
}

void nova_gui_stroke_rect(int64_t x, int64_t y, int64_t w, int64_t h, int64_t rgb) {
    if (!nova_ctx || w <= 0 || h <= 0) return;
    CGColorRef color = nova_gui_color(rgb);
    CGContextSetStrokeColorWithColor(nova_ctx, color);
    CGContextSetLineWidth(nova_ctx, 1.0);
    /* A half-pixel inset keeps a one-point border on the pixel grid. */
    CGContextStrokeRect(nova_ctx, CGRectMake((CGFloat)x + 0.5, (CGFloat)y + 0.5,
                                             (CGFloat)w - 1.0, (CGFloat)h - 1.0));
    CGColorRelease(color);
}

void nova_gui_line(int64_t x0, int64_t y0, int64_t x1, int64_t y1, int64_t rgb) {
    if (!nova_ctx) return;
    CGColorRef color = nova_gui_color(rgb);
    CGContextSetStrokeColorWithColor(nova_ctx, color);
    CGContextSetLineWidth(nova_ctx, 1.0);
    CGContextBeginPath(nova_ctx);
    CGContextMoveToPoint(nova_ctx, (CGFloat)x0 + 0.5, (CGFloat)y0 + 0.5);
    CGContextAddLineToPoint(nova_ctx, (CGFloat)x1 + 0.5, (CGFloat)y1 + 0.5);
    CGContextStrokePath(nova_ctx);
    CGColorRelease(color);
}

void nova_gui_text(int64_t x, int64_t y, nova_str text, int64_t size, int64_t rgb, int64_t mono) {
    if (!nova_ctx) return;
    CTLineRef line = nova_gui_cached_line(text, size, mono);
    if (!line) return;
    CTFontRef font = nova_gui_font(size, mono);
    CGColorRef color = nova_gui_color(rgb);
    CGContextSetFillColorWithColor(nova_ctx, color);
    /* Subpixel/light smoothing blurs light glyphs on a dark background.
     * Grayscale antialiasing keeps small text crisp instead of milky. */
    CGContextSetTextPosition(nova_ctx, (CGFloat)x, (CGFloat)y + CTFontGetAscent(font));
    CTLineDraw(line, nova_ctx);
    CGColorRelease(color);
    CFRelease(line);
}

int64_t nova_gui_text_width(nova_str text, int64_t size, int64_t mono) {
    CTLineRef line = nova_gui_cached_line(text, size, mono);
    if (!line) return 0;
    /* CTLineGetTypographicBounds returns the width; its pointer arguments are
     * ascent, descent and leading. Measuring into the first one yields the
     * ascent, which is why this reads the return value instead. */
    double width = CTLineGetTypographicBounds(line, NULL, NULL, NULL);
    CFRelease(line);
    return (int64_t)ceil(width);
}

/* Width of one space, which is the cell width a monospaced layout needs. */
int64_t nova_gui_advance(int64_t size, int64_t mono) {
    static int64_t cache[2][80];
    int index = (int)(size < 1 ? 1 : (size > 79 ? 79 : size));
    int which = mono ? 1 : 0;
    if (!cache[which][index]) {
        nova_str space = {" ", 1};
        int64_t width = nova_gui_text_width(space, size, mono);
        cache[which][index] = width > 0 ? width : (int64_t)((double)index * (mono ? 0.6 : 0.55));
    }
    return cache[which][index];
}

void nova_gui_clip(int64_t x, int64_t y, int64_t w, int64_t h) {
    if (!nova_ctx) return;
    CGContextSaveGState(nova_ctx);
    nova_clip_depth += 1;
    CGContextClipToRect(nova_ctx, CGRectMake((CGFloat)x, (CGFloat)y, (CGFloat)w, (CGFloat)h));
}

void nova_gui_clip_reset(void) {
    if (!nova_ctx) return;
    while (nova_clip_depth > 0) {
        CGContextRestoreGState(nova_ctx);
        nova_clip_depth -= 1;
    }
    CGContextSetTextMatrix(nova_ctx, CGAffineTransformMakeScale(1.0, -1.0));
    CGContextSetLineWidth(nova_ctx, 1.0);
}

/* Drains the platform queue and gives AppKit one turn to finish deferred work.
 * A program that only pumps when it draws will go unresponsive the moment it
 * idles, and a window whose layer transaction never commits stays blank, so
 * this is called on every iteration of a Nova loop, not only on redraws. */
void nova_gui_pump(void) {
    /* A freshly launched app can lose the activation race against the app that
     * was in front, leaving a dim window that ignores clicks. Nudge it forward
     * for the first fraction of a second, then leave focus alone. */
    static int startup_ticks = 40;
    if (startup_ticks > 0) {
        startup_ticks--;
        if (![NSApp isActive] && nova_window) {
            [NSApp activateIgnoringOtherApps:YES];
            [nova_window orderFrontRegardless];
        }
    }
    @autoreleasepool {
        for (;;) {
            NSEvent *event = [NSApp nextEventMatchingMask:NSEventMaskAny
                                                untilDate:[NSDate distantPast]
                                                   inMode:NSDefaultRunLoopMode
                                                  dequeue:YES];
            if (!event) break;
            [NSApp sendEvent:event];
        }
        /* `[NSDate date]` as the deadline means: run what is ready, never block. */
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode beforeDate:[NSDate date]];
    }
}

void nova_gui_present(void) {
    CFAbsoluteTime started = CFAbsoluteTimeGetCurrent();
    nova_gui_pump();
    @autoreleasepool {
        if (!nova_ctx || !nova_view) return;
        if (nova_image) {
            CGImageRelease(nova_image);
            nova_image = NULL;
        }
        nova_image = CGBitmapContextCreateImage(nova_ctx);
        [nova_view setNeedsDisplay:YES];
        [nova_view display];
        [CATransaction flush];
    }
    nova_gui_trace("present %.2fms", (CFAbsoluteTimeGetCurrent() - started) * 1000.0);
}

int64_t nova_gui_next_event(void) {
    if (nova_q_head == nova_q_tail) return NOVA_GUI_NONE;
    NovaGuiEvent *event = &nova_queue[nova_q_head];
    if (event->kind == NOVA_GUI_NONE) {
        /* Modifier-only event: consume it, report nothing. */
        nova_q_head = (nova_q_head + 1) % NOVA_GUI_QUEUE;
        return NOVA_GUI_NONE;
    }
    static NovaGuiEvent current;
    current = *event;
    nova_q_head = (nova_q_head + 1) % NOVA_GUI_QUEUE;
    return current.kind;
}

int64_t nova_gui_event_code(void) {
    return nova_queue[(nova_q_head + NOVA_GUI_QUEUE - 1) % NOVA_GUI_QUEUE].code;
}

int64_t nova_gui_event_x(void) {
    return nova_queue[(nova_q_head + NOVA_GUI_QUEUE - 1) % NOVA_GUI_QUEUE].x;
}

int64_t nova_gui_event_y(void) {
    return nova_queue[(nova_q_head + NOVA_GUI_QUEUE - 1) % NOVA_GUI_QUEUE].y;
}

int64_t nova_gui_event_mods(void) {
    return nova_queue[(nova_q_head + NOVA_GUI_QUEUE - 1) % NOVA_GUI_QUEUE].mods;
}

int64_t nova_gui_event_delta(void) {
    return nova_queue[(nova_q_head + NOVA_GUI_QUEUE - 1) % NOVA_GUI_QUEUE].delta;
}

void nova_gui_set_title(nova_str title) {
    if (!nova_window) return;
    NSString *text = [[NSString alloc] initWithBytes:title.ptr ? title.ptr : ""
                                              length:(NSUInteger)(title.len > 0 ? title.len : 0)
                                            encoding:NSUTF8StringEncoding];
    if (text) nova_window.title = text;
}

void nova_gui_activate(void) {
    nova_gui_activate_window();
}

void nova_gui_close(void) {
    nova_running = 0;
    if (nova_window) [nova_window orderOut:nil];
}

/* ------------------------------------------------------------------ */
/* Workspace access and process running                               */
/* ------------------------------------------------------------------ */

/* Nova's stdlib is still a placeholder, so a GUI workbench needs the host to
 * read, list and write files and to run a compiler. These functions are part
 * of the macOS nova-gui host, which is linked only into GUI programs. They are
 * deliberately not a sandbox: the caller decides which paths are allowed. */

static char *nova_gui_cstr(nova_str text) {
    char *buffer = (char *)malloc((size_t)(text.len > 0 ? text.len : 0) + 1);
    if (!buffer) return NULL;
    if (text.len > 0 && text.ptr) memcpy(buffer, text.ptr, (size_t)text.len);
    buffer[text.len > 0 ? text.len : 0] = '\0';
    return buffer;
}

static nova_str nova_gui_empty(void) {
    /* Nova takes ownership of host strings, so this must be heap memory: a
     * pointer to the "" literal would be handed to free() later. */
    char *buffer = (char *)malloc(1);
    if (buffer) buffer[0] = '\0';
    nova_str empty = {buffer, 0};
    return empty;
}

nova_str nova_gui_mono_font(void) {
    const char *name = nova_mono_name();
    size_t len = strlen(name);
    char *copy = (char *)malloc(len + 1);
    if (!copy) return nova_gui_empty();
    memcpy(copy, name, len + 1);
    nova_str result = {copy, (int64_t)len};
    return result;
}

int64_t nova_gui_set_mono_font(nova_str family) {
    char *name = nova_gui_cstr(family);
    if (!name) return 0;
    int matched = 0;
    @autoreleasepool {
        NSString *wanted = [NSString stringWithUTF8String:name];
        if (wanted) {
            for (NSString *candidate in [[NSFontManager sharedFontManager] availableFontFamilies]) {
                if ([candidate isEqualToString:wanted]) {
                    matched = 1;
                    break;
                }
            }
        }
        if (matched) {
            snprintf(nova_mono_family, sizeof(nova_mono_family), "%s", name);
        }
    }
    free(name);
    if (matched) {
        nova_mono_ready = 1;
        for (int i = 0; i < 80; i++) {
            if (nova_font_cache[1][i]) {
                CFRelease(nova_font_cache[1][i]);
                nova_font_cache[1][i] = NULL;
            }
        }
    }
    return matched;
}

/* Families worth offering in a terminal: patched (Nerd), monospaced or code. */
nova_str nova_gui_mono_fonts(void) {
    @autoreleasepool {
        NSMutableArray<NSString *> *names = [NSMutableArray array];
        for (NSString *name in [[NSFontManager sharedFontManager] availableFontFamilies]) {
            int nerd = [name rangeOfString:@"Nerd Font"].location != NSNotFound;
            int mono = [name rangeOfString:@"Mono"].location != NSNotFound;
            int code = [name rangeOfString:@"Code"].location != NSNotFound;
            if (nerd || mono || code) [names addObject:name];
        }
        [names sortUsingSelector:@selector(localizedCaseInsensitiveCompare:)];
        NSMutableString *out = [NSMutableString string];
        for (NSString *name in names) {
            if ([name rangeOfString:@"Nerd Font"].location != NSNotFound) {
                [out appendFormat:@"%@\n", name];
            }
        }
        for (NSString *name in names) {
            if ([name rangeOfString:@"Nerd Font"].location == NSNotFound) {
                [out appendFormat:@"%@\n", name];
            }
        }
        const char *utf8 = [out UTF8String];
        if (!utf8) return nova_gui_empty();
        size_t len = strlen(utf8);
        char *copy = (char *)malloc(len + 1);
        if (!copy) return nova_gui_empty();
        memcpy(copy, utf8, len + 1);
        nova_str result = {copy, (int64_t)len};
        return result;
    }
}

/* Reads a whole file. An unreadable path yields an empty string, which the
 * caller can distinguish with nova_fs_exists. */
nova_str nova_fs_read(nova_str path) {
    char *name = nova_gui_cstr(path);
    if (!name) return nova_gui_empty();
    FILE *file = fopen(name, "rb");
    free(name);
    if (!file) return nova_gui_empty();
    size_t cap = 8192;
    size_t len = 0;
    char *buffer = (char *)malloc(cap);
    if (!buffer) {
        fclose(file);
        return nova_gui_empty();
    }
    for (;;) {
        if (len + 4096 > cap) {
            cap *= 2;
            char *grown = (char *)realloc(buffer, cap);
            if (!grown) break;
            buffer = grown;
        }
        size_t got = fread(buffer + len, 1, 4096, file);
        len += got;
        if (got == 0) break;
    }
    fclose(file);
    buffer[len] = '\0';
    nova_str result = {buffer, (int64_t)len};
    return result;
}

int64_t nova_fs_write(nova_str path, nova_str data) {
    char *name = nova_gui_cstr(path);
    if (!name) return 0;
    FILE *file = fopen(name, "wb");
    free(name);
    if (!file) return 0;
    int ok = 1;
    if (data.len > 0 && data.ptr) ok = fwrite(data.ptr, 1, (size_t)data.len, file) == (size_t)data.len;
    if (fclose(file) != 0) ok = 0;
    return ok ? 1 : 0;
}

int64_t nova_fs_exists(nova_str path) {
    char *name = nova_gui_cstr(path);
    if (!name) return 0;
    struct stat info;
    int exists = stat(name, &info) == 0;
    free(name);
    return exists ? 1 : 0;
}

int64_t nova_fs_is_dir(nova_str path) {
    char *name = nova_gui_cstr(path);
    if (!name) return 0;
    struct stat info;
    int is_dir = stat(name, &info) == 0 && S_ISDIR(info.st_mode);
    free(name);
    return is_dir ? 1 : 0;
}

/* Directories sort before files, then by name, so a Nova file tree does not
 * need a string comparison of its own to present a stable order. */
static int nova_gui_entry_compare(const void *left, const void *right) {
    const struct dirent *a = *(const struct dirent *const *)left;
    const struct dirent *b = *(const struct dirent *const *)right;
    int a_dir = a->d_type == DT_DIR;
    int b_dir = b->d_type == DT_DIR;
    if (a_dir != b_dir) return a_dir ? -1 : 1;
    return strcmp(a->d_name, b->d_name);
}

/* Lists a directory as one entry per line: "d<TAB>name" or "f<TAB>name" for
 * children only, directories first, with no dot entries. Missing or unreadable
 * directories list as empty, which the caller renders as an empty tree. */
nova_str nova_fs_list(nova_str path) {
    char *name = nova_gui_cstr(path);
    if (!name) return nova_gui_empty();
    struct dirent **entries = NULL;
    int count = scandir(name, &entries, NULL, NULL);
    free(name);
    if (count < 0) {
        if (entries) free(entries);
        return nova_gui_empty();
    }
    qsort(entries, (size_t)count, sizeof(struct dirent *), nova_gui_entry_compare);

    size_t cap = 4096;
    size_t len = 0;
    char *buffer = (char *)malloc(cap);
    if (!buffer) {
        for (int i = 0; i < count; i++) free(entries[i]);
        free(entries);
        return nova_gui_empty();
    }
    for (int i = 0; i < count; i++) {
        const char *child_name = entries[i]->d_name;
        if (child_name[0] == '.') continue; /* hidden files stay out of the tree */
        size_t need = strlen(child_name) + 3;
        if (len + need + 1 > cap) {
            while (len + need + 1 > cap) cap *= 2;
            char *grown = (char *)realloc(buffer, cap);
            if (!grown) break;
            buffer = grown;
        }
        buffer[len++] = entries[i]->d_type == DT_DIR ? 'd' : 'f';
        buffer[len++] = '\t';
        memcpy(buffer + len, child_name, strlen(child_name));
        len += strlen(child_name);
        buffer[len++] = '\n';
    }
    for (int i = 0; i < count; i++) free(entries[i]);
    free(entries);
    buffer[len] = '\0';
    nova_str result = {buffer, (int64_t)len};
    return result;
}

static int nova_gui_exit_status = -1;

/* Runs a shell command in a directory and returns its combined output. The
 * compiler is invoked with a command string the app builds from fixed parts. */
nova_str nova_process_run(nova_str command, nova_str cwd) {
    char *cmd = nova_gui_cstr(command);
    char *dir = nova_gui_cstr(cwd);
    if (!cmd || !dir) {
        free(cmd);
        free(dir);
        return nova_gui_empty();
    }
    size_t size = strlen(cmd) + strlen(dir) + 32;
    char *full = (char *)malloc(size);
    if (!full) {
        free(cmd);
        free(dir);
        return nova_gui_empty();
    }
    snprintf(full, size, "cd '%s' && %s 2>&1", dir, cmd);
    free(cmd);
    free(dir);

    FILE *pipe = popen(full, "r");
    free(full);
    if (!pipe) {
        nova_gui_exit_status = -1;
        return nova_gui_empty();
    }
    size_t cap = 8192;
    size_t len = 0;
    char *buffer = (char *)malloc(cap);
    if (!buffer) {
        pclose(pipe);
        return nova_gui_empty();
    }
    for (;;) {
        if (len + 4096 > cap) {
            cap *= 2;
            char *grown = (char *)realloc(buffer, cap);
            if (!grown) break;
            buffer = grown;
        }
        size_t got = fread(buffer + len, 1, 4096, pipe);
        len += got;
        if (got == 0) break;
    }
    int status = pclose(pipe);
    nova_gui_exit_status = WIFEXITED(status) ? WEXITSTATUS(status) : -1;
    buffer[len] = '\0';
    nova_str result = {buffer, (int64_t)len};
    return result;
}

int64_t nova_process_status(void) {
    return nova_gui_exit_status;
}

nova_str nova_fs_cwd(void) {
    char buffer[4096];
    if (!getcwd(buffer, sizeof(buffer))) return nova_gui_empty();
    size_t len = strlen(buffer);
    char *copy = (char *)malloc(len + 1);
    if (!copy) return nova_gui_empty();
    memcpy(copy, buffer, len + 1);
    nova_str result = {copy, (int64_t)len};
    return result;
}

/* Native open panel: mode 0 picks a file, mode 1 picks a folder. The panel
 * runs its own modal loop, which is safe here because the app's loop is the
 * main thread and nothing holds state across the call. */
/* ------------------------------------------------------------------ */
/* Terminal pool (pty)                                                */
/* ------------------------------------------------------------------ */

/* A terminal is a child under a real pty: it can prompt, colourise, read keys
 * and see a window size. Up to eight run at once, each with its own reader
 * thread that appends to a locked buffer. Slot 0 keeps the older single
 * terminal API working. */

#define NOVA_PTY_SLOTS 8

typedef struct {
    pthread_mutex_t lock;
    char *buffer;
    size_t length;
    size_t capacity;
    size_t seen;
    int active;
    int master;
    int exit_code;
    pid_t pid;
} nova_pty_slot;

static nova_pty_slot nova_pty_pool[NOVA_PTY_SLOTS];
static pthread_once_t nova_pty_once = PTHREAD_ONCE_INIT;

static void nova_pty_setup(void) {
    for (int i = 0; i < NOVA_PTY_SLOTS; i++) {
        pthread_mutex_init(&nova_pty_pool[i].lock, NULL);
        nova_pty_pool[i].buffer = NULL;
        nova_pty_pool[i].length = 0;
        nova_pty_pool[i].capacity = 0;
        nova_pty_pool[i].seen = 0;
        nova_pty_pool[i].active = 0;
        nova_pty_pool[i].master = -1;
        nova_pty_pool[i].exit_code = -1;
        nova_pty_pool[i].pid = 0;
    }
}

static nova_pty_slot *nova_pty_get(int64_t slot) {
    pthread_once(&nova_pty_once, nova_pty_setup);
    if (slot < 0 || slot >= NOVA_PTY_SLOTS) return NULL;
    return &nova_pty_pool[slot];
}

static void nova_pty_append(nova_pty_slot *term, const char *data, size_t len) {
    pthread_mutex_lock(&term->lock);
    if (term->length + len + 1 > term->capacity) {
        size_t cap = term->capacity ? term->capacity : 8192;
        while (cap < term->length + len + 1) cap *= 2;
        char *grown = (char *)realloc(term->buffer, cap);
        if (grown) {
            term->buffer = grown;
            term->capacity = cap;
        }
    }
    if (term->buffer && term->capacity >= term->length + len + 1) {
        memcpy(term->buffer + term->length, data, len);
        term->length += len;
        term->buffer[term->length] = '\0';
    }
    pthread_mutex_unlock(&term->lock);
}

struct nova_pty_job {
    nova_pty_slot *term;
    int master;
    pid_t pid;
};

static void *nova_pty_worker(void *arg) {
    struct nova_pty_job *job = (struct nova_pty_job *)arg;
    char chunk[8192];
    for (;;) {
        ssize_t got = read(job->master, chunk, sizeof(chunk));
        if (got > 0) {
            nova_pty_append(job->term, chunk, (size_t)got);
        } else if (got == 0) {
            break;
        } else if (errno == EINTR) {
            continue;
        } else if (errno == EAGAIN || errno == EWOULDBLOCK) {
            struct timespec pause = {0, 1000000};
            nanosleep(&pause, NULL);
        } else {
            break;
        }
    }
    close(job->master);
    int status = 0;
    waitpid(job->pid, &status, 0);
    int code = WIFEXITED(status) ? WEXITSTATUS(status) : -1;
    pthread_mutex_lock(&job->term->lock);
    job->term->active = 0;
    job->term->master = -1;
    job->term->exit_code = code;
    pthread_mutex_unlock(&job->term->lock);
    nova_gui_exit_status = code;
    free(job);
    return NULL;
}

/* Opens `command` under a fresh pty in `cwd`, sized cols x rows. Returns 1 on
 * success; a slot that is still running is not replaced. */
int64_t nova_pty_open(int64_t slot, nova_str command, nova_str cwd, int64_t cols, int64_t rows) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return 0;
    pthread_mutex_lock(&term->lock);
    int busy = term->active;
    pthread_mutex_unlock(&term->lock);
    if (busy) return 0;
    char *cmd = nova_gui_cstr(command);
    char *dir = nova_gui_cstr(cwd);
    if (!cmd || !dir) {
        free(cmd);
        free(dir);
        return 0;
    }
    int master = posix_openpt(O_RDWR | O_NOCTTY);
    if (master < 0) {
        free(cmd);
        free(dir);
        return 0;
    }
    if (grantpt(master) != 0 || unlockpt(master) != 0) {
        close(master);
        free(cmd);
        free(dir);
        return 0;
    }
    const char *slave_name = ptsname(master);
    if (!slave_name) {
        close(master);
        free(cmd);
        free(dir);
        return 0;
    }
    struct winsize size;
    size.ws_row = (unsigned short)(rows > 0 ? rows : 30);
    size.ws_col = (unsigned short)(cols > 0 ? cols : 120);
    size.ws_xpixel = 0;
    size.ws_ypixel = 0;
    ioctl(master, TIOCSWINSZ, &size);
    fcntl(master, F_SETFL, fcntl(master, F_GETFL, 0) | O_NONBLOCK);

    pid_t pid = fork();
    if (pid < 0) {
        close(master);
        free(cmd);
        free(dir);
        return 0;
    }
    if (pid == 0) {
        setsid();
        int slave = open(slave_name, O_RDWR);
        if (slave < 0) _exit(127);
        ioctl(slave, TIOCSCTTY, 0);
        dup2(slave, 0);
        dup2(slave, 1);
        dup2(slave, 2);
        if (slave > 2) close(slave);
        if (chdir(dir) != 0) _exit(127);
        execl("/bin/sh", "sh", "-c", cmd, (char *)NULL);
        _exit(127);
    }
    free(cmd);
    free(dir);
    pthread_mutex_lock(&term->lock);
    term->length = 0;
    term->seen = 0;
    term->exit_code = -1;
    if (term->buffer) term->buffer[0] = '\0';
    term->active = 1;
    term->master = master;
    term->pid = pid;
    pthread_mutex_unlock(&term->lock);
    struct nova_pty_job *job = (struct nova_pty_job *)malloc(sizeof(struct nova_pty_job));
    if (!job) {
        close(master);
        term->active = 0;
        return 0;
    }
    job->term = term;
    job->master = master;
    job->pid = pid;
    pthread_t thread;
    if (pthread_create(&thread, NULL, nova_pty_worker, job) != 0) {
        close(master);
        free(job);
        pthread_mutex_lock(&term->lock);
        term->active = 0;
        pthread_mutex_unlock(&term->lock);
        return 0;
    }
    pthread_detach(thread);
    return 1;
}

/* Returns only the bytes produced since the last poll. */
nova_str nova_pty_poll(int64_t slot) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return nova_gui_empty();
    pthread_mutex_lock(&term->lock);
    size_t from = term->seen;
    size_t to = term->length;
    char *copy = NULL;
    size_t len = 0;
    if (to > from && term->buffer) {
        len = to - from;
        copy = (char *)malloc(len + 1);
        if (copy) {
            memcpy(copy, term->buffer + from, len);
            copy[len] = '\0';
            term->seen = to;
        } else {
            len = 0;
        }
    }
    pthread_mutex_unlock(&term->lock);
    if (!copy) return nova_gui_empty();
    nova_str result = {copy, (int64_t)len};
    return result;
}

int64_t nova_pty_running(int64_t slot) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return 0;
    pthread_mutex_lock(&term->lock);
    int active = term->active;
    pthread_mutex_unlock(&term->lock);
    return active;
}

/* One byte: 13 Enter, 127 Backspace, ASCII, or a UTF-8 lead byte. */
int64_t nova_pty_write(int64_t slot, int64_t code) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return 0;
    pthread_mutex_lock(&term->lock);
    int master = term->master;
    pthread_mutex_unlock(&term->lock);
    if (master < 0) return 0;
    unsigned char byte = (unsigned char)(code & 0xFF);
    ssize_t wrote = write(master, &byte, 1);
    return wrote == 1 ? 1 : 0;
}

/* Writes a whole UTF-8 string, e.g. an escape sequence, a paste or a tab. */
int64_t nova_pty_write_text(int64_t slot, nova_str text) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term || text.len <= 0) return 0;
    pthread_mutex_lock(&term->lock);
    int master = term->master;
    pthread_mutex_unlock(&term->lock);
    if (master < 0) return 0;
    ssize_t wrote = write(master, text.ptr, (size_t)text.len);
    return wrote == text.len ? 1 : 0;
}

void nova_pty_resize(int64_t slot, int64_t cols, int64_t rows) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return;
    pthread_mutex_lock(&term->lock);
    int master = term->master;
    pid_t pid = term->pid;
    pthread_mutex_unlock(&term->lock);
    if (master < 0) return;
    struct winsize size;
    size.ws_row = (unsigned short)(rows > 0 ? rows : 24);
    size.ws_col = (unsigned short)(cols > 0 ? cols : 80);
    size.ws_xpixel = 0;
    size.ws_ypixel = 0;
    ioctl(master, TIOCSWINSZ, &size);
    if (pid > 0) kill(pid, SIGWINCH);
}

void nova_pty_clear(int64_t slot) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return;
    pthread_mutex_lock(&term->lock);
    term->length = 0;
    term->seen = 0;
    if (term->buffer) term->buffer[0] = '\0';
    pthread_mutex_unlock(&term->lock);
}

/* Exit code of the last process in the slot, or -1 while it runs. */
int64_t nova_pty_status(int64_t slot) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return -1;
    pthread_mutex_lock(&term->lock);
    int code = term->exit_code;
    pthread_mutex_unlock(&term->lock);
    return code;
}

/* Asks the child to go away: SIGTERM, then the slot is reusable. */
void nova_pty_close(int64_t slot) {
    nova_pty_slot *term = nova_pty_get(slot);
    if (!term) return;
    pthread_mutex_lock(&term->lock);
    pid_t pid = term->pid;
    int master = term->master;
    pthread_mutex_unlock(&term->lock);
    if (pid > 0) kill(pid, SIGHUP);
    if (master >= 0) close(master);
    pthread_mutex_lock(&term->lock);
    term->active = 0;
    term->master = -1;
    pthread_mutex_unlock(&term->lock);
}

/* The original single-terminal API, kept as slot 0 for existing callers. */
int64_t nova_term_start(nova_str command, nova_str cwd) {
    return nova_pty_open(0, command, cwd, 120, 30);
}

nova_str nova_term_poll(void) {
    return nova_pty_poll(0);
}

int64_t nova_term_running(void) {
    return nova_pty_running(0);
}

int64_t nova_term_send(int64_t code) {
    return nova_pty_write(0, code);
}

void nova_term_clear(void) {
    nova_pty_clear(0);
}

/* Clipboard access for the editor: ⌘C, ⌘X and ⌘V go through NSPasteboard. */
nova_str nova_gui_clipboard_get(void) {
    @autoreleasepool {
        NSString *text = [[NSPasteboard generalPasteboard] stringForType:NSPasteboardTypeString];
        if (!text) return nova_gui_empty();
        const char *utf8 = [text UTF8String];
        size_t len = strlen(utf8);
        char *copy = (char *)malloc(len + 1);
        if (!copy) return nova_gui_empty();
        memcpy(copy, utf8, len + 1);
        nova_str result = {copy, (int64_t)len};
        return result;
    }
}

void nova_gui_clipboard_set(nova_str text) {
    @autoreleasepool {
        char *utf8 = nova_gui_cstr(text);
        if (!utf8) return;
        NSString *value = [[NSString alloc] initWithUTF8String:utf8];
        free(utf8);
        NSPasteboard *board = [NSPasteboard generalPasteboard];
        [board clearContents];
        if (value) [board setString:value forType:NSPasteboardTypeString];
    }
}

nova_str nova_gui_panel(int64_t mode) {
    @autoreleasepool {
        if (!NSApp) return nova_gui_empty();
        NSOpenPanel *panel = [NSOpenPanel openPanel];
        [panel setCanChooseFiles:mode == 0];
        [panel setCanChooseDirectories:mode == 1];
        [panel setAllowsMultipleSelection:NO];
        [panel setCanCreateDirectories:YES];
        [panel setTitle:(mode == 1 ? @"Open Folder" : @"Open File")];
        /* A borderless Nova window can leave the app inactive, and an inactive
         * app's modal panel never comes forward: activate first. */
        [NSApp activateIgnoringOtherApps:YES];
        if ([panel runModal] != NSModalResponseOK) return nova_gui_empty();
        NSURL *url = [[panel URLs] firstObject];
        if (!url || ![url path]) return nova_gui_empty();
        const char *utf8 = [[url path] UTF8String];
        size_t len = strlen(utf8);
        char *copy = (char *)malloc(len + 1);
        if (!copy) return nova_gui_empty();
        memcpy(copy, utf8, len + 1);
        nova_str result = {copy, (int64_t)len};
        return result;
    }
}

nova_str nova_env(nova_str name) {
    char *key = nova_gui_cstr(name);
    if (!key) return nova_gui_empty();
    const char *value = getenv(key);
    free(key);
    if (!value) return nova_gui_empty();
    size_t len = strlen(value);
    char *copy = (char *)malloc(len + 1);
    if (!copy) return nova_gui_empty();
    memcpy(copy, value, len + 1);
    nova_str result = {copy, (int64_t)len};
    return result;
}

/* Writes the current framebuffer to a PNG. This exists so a headless run can
 * verify what the window drew without a screen recorder. */
int64_t nova_gui_save_png(nova_str path) {
    if (!nova_ctx) return 0;
    char *name = nova_gui_cstr(path);
    if (!name) return 0;
    CGImageRef image = CGBitmapContextCreateImage(nova_ctx);
    CFStringRef target = CFStringCreateWithCString(NULL, name, kCFStringEncodingUTF8);
    free(name);
    if (!image || !target) {
        if (image) CGImageRelease(image);
        if (target) CFRelease(target);
        return 0;
    }
    CFURLRef url = CFURLCreateWithFileSystemPath(NULL, target, kCFURLPOSIXPathStyle, false);
    int64_t ok = 0;
    if (url) {
        CGImageDestinationRef destination = CGImageDestinationCreateWithURL(url, CFSTR("public.png"), 1, NULL);
        if (destination) {
            CGImageDestinationAddImage(destination, image, NULL);
            ok = CGImageDestinationFinalize(destination) ? 1 : 0;
            CFRelease(destination);
        }
        CFRelease(url);
    }
    CFRelease(target);
    CGImageRelease(image);
    return ok;
}

/* ---------- sound ----------
 *
 * Short UI tones are synthesized here rather than shipped as files: a sine with
 * a 6 ms attack and an exponential decay, wrapped in a small WAV container and
 * handed to NSSound. That keeps a game bundle self-contained and lets the Nova
 * side pick a pitch per event. Several tones may overlap, which gives a level
 * clear its little chord.
 */

static NSMutableArray *nova_sounds = nil;

static void nova_wav_u32(unsigned char *p, uint32_t v) {
    p[0] = (unsigned char)(v & 0xff);
    p[1] = (unsigned char)((v >> 8) & 0xff);
    p[2] = (unsigned char)((v >> 16) & 0xff);
    p[3] = (unsigned char)((v >> 24) & 0xff);
}

static void nova_wav_u16(unsigned char *p, uint16_t v) {
    p[0] = (unsigned char)(v & 0xff);
    p[1] = (unsigned char)((v >> 8) & 0xff);
}

int nova_sound_tone(int64_t frequency, int64_t milliseconds, int64_t volume) {
    if (frequency < 40) frequency = 40;
    if (frequency > 8000) frequency = 8000;
    if (milliseconds < 10) milliseconds = 10;
    if (milliseconds > 1200) milliseconds = 1200;
    if (volume <= 0) return 0;
    if (volume > 100) volume = 100;

    const int64_t rate = 44100;
    const int64_t frames = rate * milliseconds / 1000;
    const int64_t data_bytes = frames * 2;
    const int64_t total = 44 + data_bytes;
    unsigned char *buffer = calloc(1, (size_t)total);
    if (!buffer) return 0;

    memcpy(buffer + 0, "RIFF", 4);
    nova_wav_u32(buffer + 4, (uint32_t)(36 + data_bytes));
    memcpy(buffer + 8, "WAVEfmt ", 8);
    nova_wav_u32(buffer + 16, 16);
    nova_wav_u16(buffer + 20, 1);
    nova_wav_u16(buffer + 22, 1);
    nova_wav_u32(buffer + 24, (uint32_t)rate);
    nova_wav_u32(buffer + 28, (uint32_t)(rate * 2));
    nova_wav_u16(buffer + 32, 2);
    nova_wav_u16(buffer + 34, 16);
    memcpy(buffer + 36, "data", 4);
    nova_wav_u32(buffer + 40, (uint32_t)data_bytes);

    const double amplitude = 0.32 * (double)volume / 100.0;
    const double attack = 0.006 * (double)rate;
    const double decay = (double)frames * 0.32;
    for (int64_t i = 0; i < frames; i++) {
        double t = (double)i / (double)rate;
        double envelope = 1.0;
        if ((double)i < attack) envelope = (double)i / attack;
        envelope *= exp(-(double)i / decay);
        double phase = 2.0 * M_PI * (double)frequency * t;
        /* A touch of second harmonic reads as "game" rather than "beep". */
        double sample = sin(phase) * 0.82 + sin(phase * 2.0) * 0.18;
        int16_t pcm = (int16_t)(sample * envelope * amplitude * 32767.0);
        nova_wav_u16(buffer + 44 + i * 2, (uint16_t)pcm);
    }

    NSData *data = [NSData dataWithBytes:buffer length:(NSUInteger)total];
    free(buffer);
    NSSound *sound = [[NSSound alloc] initWithData:data];
    if (!sound) return 0;
    sound.volume = (float)volume / 100.0f;
    if (!nova_sounds) nova_sounds = [NSMutableArray array];
    [nova_sounds addObject:sound];
    while (nova_sounds.count > 24) [nova_sounds removeObjectAtIndex:0];
    return [sound play] ? 1 : 0;
}
