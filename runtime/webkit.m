/*
 * nova-webkit — the thin macOS WebKit adapter for Nova Browser.
 *
 * This file is deliberately small and replaceable. It owns WKWebView objects,
 * navigation callbacks and snapshots, but no browser UI, history, layout or
 * document model. Nova owns those through libraries/nova-webview and the
 * browser engine adapter. WebCore must never call into this file.
 *
 * Nova has no C-to-Nova callbacks, so state is polled just like nova-gui.
 * Handles are opaque integers; raw Objective-C objects never cross the ABI.
 */

#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
#import <Security/Security.h>
#import <CommonCrypto/CommonCrypto.h>

#include <math.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Matches the Nova runtime ABI: { const char *ptr; int64_t len; } UTF-8. */
typedef struct {
    const char *ptr;
    int64_t len;
} nova_str;

#define NOVA_WEBVIEW_EMPTY 0
#define NOVA_WEBVIEW_LOADING 1
#define NOVA_WEBVIEW_LOADED 2
#define NOVA_WEBVIEW_FAILED 3
#define NOVA_WEBVIEW_INVALID (-1)

@class NovaWebViewEntry;

@interface NovaWebViewDelegate : NSObject <WKNavigationDelegate>
@property (nonatomic, weak) NovaWebViewEntry *entry;
@end

@interface NovaWebViewEntry : NSObject
@property (nonatomic, strong) WKWebView *webView;
@property (nonatomic, strong) NSView *hostView;
@property (nonatomic, strong) NovaWebViewDelegate *delegate;
@property (nonatomic, assign) int64_t state;
@property (nonatomic, copy) NSString *errorText;
@property (nonatomic, copy) NSString *titleFallback;
@property (nonatomic, assign) BOOL chromeFocusRequested;
@property (nonatomic, copy) NSString *pendingURL;
@property (nonatomic, copy) NSString *errorURL;
@property (nonatomic, strong) WKNavigation *currentNavigation;
@property (nonatomic, assign) int64_t generation;
@property (nonatomic, assign) int64_t findState; /* 0 cleared 1 found 2 not found 3 searching */
@property (nonatomic, assign) BOOL privateView;
/* The last password form the page submitted, held until Nova polls it into
 * the Keychain. Nothing is written for a private view. */
@property (nonatomic, copy) NSString *loginOrigin;
@property (nonatomic, copy) NSString *loginUser;
@property (nonatomic, copy) NSString *loginPass;
@end

@implementation NovaWebViewEntry
@end

/* One real WebKit download. The destination is a unique file in the download
 * directory, and progress is read from that file so the ABI never needs a
 * callback: Nova polls these accessors in its frame loop. */
static NSString *nova_download_directory(void);
static NSString *nova_download_target(NSString *suggested);
static void nova_webview_register_saved(NSString *path, int64_t bytes);

@interface NovaWebDownload : NSObject <WKDownloadDelegate>
@property (nonatomic, assign) int64_t ident;
@property (nonatomic, strong) WKDownload *download;
@property (nonatomic, copy) NSString *name;
@property (nonatomic, copy) NSString *path;
@property (nonatomic, assign) int64_t expected;
@property (nonatomic, assign) int64_t state; /* 0 waiting 1 active 2 done 3 failed 4 cancelled 5 paused */
@property (nonatomic, strong) WKWebView *webView; /* resuming needs the originating view */
@property (nonatomic, strong) NSData *resumeData;
@end

@implementation NovaWebDownload

- (void)download:(WKDownload *)download decideDestinationUsingResponse:(NSURLResponse *)response
 suggestedFilename:(NSString *)suggestedFilename
 completionHandler:(void (^)(NSURL *destination))completionHandler {
    (void)download;
    /* A resumed transfer must keep writing into the partial file it started. */
    if (self.path.length > 0) {
        self.state = 1;
        completionHandler([NSURL fileURLWithPath:self.path]);
        return;
    }
    NSString *target = nova_download_target(suggestedFilename);
    self.name = target.lastPathComponent;
    self.path = target;
    self.expected = response.expectedContentLength > 0 ? response.expectedContentLength : -1;
    self.state = 1;
    completionHandler([NSURL fileURLWithPath:target]);
}

- (void)downloadDidFinish:(WKDownload *)download {
    (void)download;
    NSDictionary *attrs = [[NSFileManager defaultManager] attributesOfItemAtPath:self.path error:NULL];
    if (attrs) {
        self.expected = (int64_t)[attrs fileSize];
    }
    self.state = 2;
    self.webView = nil;
}

- (void)download:(WKDownload *)download didFailWithError:(NSError *)error resumeData:(NSData *)resumeData {
    (void)download;
    (void)error;
    self.resumeData = resumeData;
    self.state = 3;
}

@end

static NSMutableDictionary<NSNumber *, NovaWebViewEntry *> *nova_webviews = nil;
static NSMutableArray<NovaWebDownload *> *nova_downloads = nil;
static int64_t nova_next_download = 1;

/* The Downloads folder, and a unique file name inside it. Shared by real
 * WKDownloads and by the blob bridge below. */
static NSString *nova_download_directory(void) {
    const char *dir_env = getenv("NOVA_BROWSER_DOWNLOAD_DIR");
    return (dir_env && *dir_env) ? [NSString stringWithUTF8String:dir_env]
                                 : [NSHomeDirectory() stringByAppendingPathComponent:@"Downloads"];
}

static NSString *nova_download_target(NSString *suggested) {
    NSString *name = suggested.lastPathComponent;
    if (name.length == 0) name = @"download";
    name = [name stringByReplacingOccurrencesOfString:@"/" withString:@"_"];
    NSString *dir = nova_download_directory();
    [[NSFileManager defaultManager] createDirectoryAtPath:dir
                              withIntermediateDirectories:YES
                                               attributes:nil
                                                    error:NULL];
    NSString *target = [dir stringByAppendingPathComponent:name];
    NSString *base = name.stringByDeletingPathExtension;
    NSString *ext = name.pathExtension;
    int suffix = 1;
    while ([[NSFileManager defaultManager] fileExistsAtPath:target]) {
        NSString *candidate = ext.length > 0
            ? [NSString stringWithFormat:@"%@-%d.%@", base, suffix, ext]
            : [NSString stringWithFormat:@"%@-%d", base, suffix];
        target = [dir stringByAppendingPathComponent:candidate];
        suffix = suffix + 1;
    }
    return target;
}

/* A finished file that did not come through WKDownload: the page built a Blob
 * and clicked a download link, and the bytes arrived over the message bridge. */
static void nova_webview_register_saved(NSString *path, int64_t bytes) {
    if (!nova_downloads) nova_downloads = [NSMutableArray array];
    NovaWebDownload *entry = [[NovaWebDownload alloc] init];
    entry.ident = nova_next_download++;
    entry.name = path.lastPathComponent;
    entry.path = path;
    entry.expected = bytes;
    entry.state = 2;
    [nova_downloads addObject:entry];
}

/* Chrome's download button on chatgpt.com and similar apps builds a Blob and
 * clicks an <a download>. WKWebView cannot turn a blob: URL into a WKDownload,
 * so the page hands the bytes over instead: fetch the blob, read it as base64,
 * and let the host write the file and list it like any other download. */
static NSString *nova_blob_script(void) {
    return @"(function(){"
           "function closest(a){return a&&a.closest?a.closest('a[download]'):null;}"
           "document.addEventListener('click',function(ev){"
           "var a=closest(ev.target);if(!a)return;"
           "var href=a.href||'';if(href.indexOf('blob:')!==0)return;"
           "ev.preventDefault();ev.stopPropagation();"
           "fetch(href).then(function(r){return r.blob();}).then(function(b){"
           "var fr=new FileReader();"
           "fr.onload=function(){var s=String(fr.result);var i=s.indexOf(',');"
           "window.webkit.messageHandlers.novaBlob.postMessage({name:a.getAttribute('download')||'download',data:s.slice(i+1)});};"
           "fr.readAsDataURL(b);});"
           "},true);"
           "})();";
}

@interface NovaBlobSink : NSObject <WKScriptMessageHandler>
@end

@implementation NovaBlobSink
- (void)userContentController:(WKUserContentController *)controller
      didReceiveScriptMessage:(WKScriptMessage *)message {
    (void)controller;
    NSDictionary *body = [message.body isKindOfClass:[NSDictionary class]] ? message.body : nil;
    NSString *name = body[@"name"];
    NSString *encoded = body[@"data"];
    if (![name isKindOfClass:[NSString class]]) name = @"download";
    if (![encoded isKindOfClass:[NSString class]] || encoded.length == 0) return;
    NSData *data = [[NSData alloc] initWithBase64EncodedString:encoded
                                                       options:NSDataBase64DecodingIgnoreUnknownCharacters];
    if (!data) return;
    NSString *target = nova_download_target(name);
    if (![data writeToFile:target atomically:YES]) return;
    nova_webview_register_saved(target, (int64_t)data.length);
}
@end

static void nova_webview_register_download(WKWebView *webView, WKDownload *download) {
    if (!download) return;
    if (!nova_downloads) nova_downloads = [NSMutableArray array];
    NovaWebDownload *entry = [[NovaWebDownload alloc] init];
    entry.ident = nova_next_download++;
    entry.download = download;
    /* download.webView is nil when a main-frame navigation turned into a
     * download, so keep the view the delegate already has. */
    entry.webView = download.webView ?: webView;
    entry.state = 0;
    entry.expected = -1;
    download.delegate = entry;
    [nova_downloads addObject:entry];
}

static NovaWebDownload *nova_webview_download(int64_t ident) {
    if (!nova_downloads) return nil;
    for (NovaWebDownload *entry in nova_downloads) {
        if (entry.ident == ident) return entry;
    }
    return nil;
}
static int64_t nova_next_handle = 1;
/* One native window hosts every tab, so browser-level focus requests and
 * shortcuts belong to the window, not to whichever WKWebView the dictionary
 * happens to enumerate first. */
static BOOL nova_chrome_focus_request = NO;
static int64_t nova_pending_shortcut = 0;
static int64_t nova_pending_mods = 0;

static NovaWebViewEntry *nova_webview_entry(int64_t handle) {
    if (!nova_webviews || handle <= 0) return nil;
    return nova_webviews[@(handle)];
}

static NovaWebViewEntry *nova_webview_entry_for_window(NSWindow *window) {
    if (!window || !nova_webviews) return nil;
    for (NSNumber *key in nova_webviews) {
        NovaWebViewEntry *entry = nova_webviews[key];
        if (entry.webView.window == window) return entry;
    }
    return nil;
}

/* Cmd+L is a browser-level command, not page content. WKWebView consumes it
 * while the page owns focus, so the embedder needs a narrow interception that
 * hands keyboard focus back to the Nova chrome. It does not alter the page or
 * execute a shortcut on its own: Nova polls the request and decides. */
static id nova_webview_key_monitor = nil;
static void nova_webview_install_key_monitor(void) {
    if (nova_webview_key_monitor) return;
    nova_webview_key_monitor = [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskKeyDown
                                                                   handler:^NSEvent *(NSEvent *event) {
        if (!(event.modifierFlags & NSEventModifierFlagCommand)) return event;
        NSString *key = event.charactersIgnoringModifiers.lowercaseString;
        NovaWebViewEntry *entry = nova_webview_entry_for_window(event.window);
        if (!entry || !entry.webView.superview) return event;
        if ([key isEqualToString:@"l"]) {
            nova_chrome_focus_request = YES;
            [event.window makeFirstResponder:entry.webView.superview];
            return nil;
        }
        if ([key isEqualToString:@"t"]) {
            nova_pending_shortcut = 116;
            nova_pending_mods = 1 | ((event.modifierFlags & NSEventModifierFlagShift) ? 2 : 0);
            return nil;
        }
        if ([key isEqualToString:@"w"]) {
            nova_pending_shortcut = 119;
            nova_pending_mods = 1 | ((event.modifierFlags & NSEventModifierFlagShift) ? 2 : 0);
            return nil;
        }
        if ([key isEqualToString:@"n"] && (event.modifierFlags & NSEventModifierFlagShift)) {
            nova_pending_shortcut = 45;
            nova_pending_mods = 1 | 2;
            return nil;
        }
        if (key.length == 1) {
            unichar digit = [key characterAtIndex:0];
            if (digit >= '1' && digit <= '6') {
                nova_pending_shortcut = 49 + (int64_t)(digit - '1');
                nova_pending_mods = 1;
                return nil;
            }
        }
        /* Bookmark, tab layout and Google import are chrome commands too, so
         * they have to be queued while the page owns focus. Only the exact
         * chords are taken; Cmd+D/E/I without Shift stay with the page. */
        if ([key isEqualToString:@"b"] && (event.modifierFlags & NSEventModifierFlagOption)) {
            nova_pending_shortcut = 98;
            nova_pending_mods = 1 | 4;
            return nil;
        }
        if ([key isEqualToString:@"d"]) {
            nova_pending_shortcut = 100;
            nova_pending_mods = 1;
            return nil;
        }
        /* Find and page zoom are chrome commands too. The page must not keep
         * Cmd+F for its own find or Cmd+= for its own zoom. */
        if ([key isEqualToString:@"f"]) {
            nova_pending_shortcut = 3;
            nova_pending_mods = 1;
            return nil;
        }
        if ([key isEqualToString:@"g"]) {
            nova_pending_shortcut = 5;
            nova_pending_mods = 1 | ((event.modifierFlags & NSEventModifierFlagShift) ? 2 : 0);
            return nil;
        }
        if ([key isEqualToString:@"="] || [key isEqualToString:@"+"]) {
            nova_pending_shortcut = 24;
            nova_pending_mods = 1 | ([key isEqualToString:@"+"] ? 2 : 0);
            return nil;
        }
        if ([key isEqualToString:@"-"]) {
            nova_pending_shortcut = 27;
            nova_pending_mods = 1;
            return nil;
        }
        if ([key isEqualToString:@"0"]) {
            nova_pending_shortcut = 29;
            nova_pending_mods = 1;
            return nil;
        }
        if ([key isEqualToString:@"e"] && (event.modifierFlags & NSEventModifierFlagShift)) {
            nova_pending_shortcut = 101;
            nova_pending_mods = 1 | 2;
            return nil;
        }
        if ([key isEqualToString:@"i"] && (event.modifierFlags & NSEventModifierFlagShift)) {
            nova_pending_shortcut = 105;
            nova_pending_mods = 1 | 2;
            return nil;
        }
        if ([key isEqualToString:@"s"] && (event.modifierFlags & NSEventModifierFlagShift)) {
            nova_pending_shortcut = 115;
            nova_pending_mods = 1 | 2;
            return nil;
        }
        return event;
    }];
}

static nova_str nova_webview_empty(void) {
    char *buffer = (char *)malloc(1);
    if (buffer) buffer[0] = '\0';
    nova_str result = {buffer, 0};
    return result;
}

static nova_str nova_webview_copy(NSString *value) {
    if (!value) return nova_webview_empty();
    NSData *data = [value dataUsingEncoding:NSUTF8StringEncoding];
    if (!data) return nova_webview_empty();
    NSUInteger length = data.length;
    char *buffer = (char *)malloc(length + 1);
    if (!buffer) return nova_webview_empty();
    memcpy(buffer, data.bytes, length);
    buffer[length] = '\0';
    nova_str result = {buffer, (int64_t)length};
    return result;
}

static NSString *nova_webview_string(nova_str value) {
    if (!value.ptr || value.len <= 0) return @"";
    NSString *text = [[NSString alloc] initWithBytes:value.ptr
                                              length:(NSUInteger)value.len
                                            encoding:NSUTF8StringEncoding];
    return text ? text : @"";
}

static BOOL nova_webview_scheme_allowed(NSString *scheme) {
    if (!scheme || scheme.length == 0) return NO;
    NSString *lower = scheme.lowercaseString;
    return [lower isEqualToString:@"http"] ||
           [lower isEqualToString:@"https"] ||
           [lower isEqualToString:@"about"];
}

static void nova_webview_fail(NovaWebViewEntry *entry, NSString *message) {
    if (!entry) return;
    entry.state = NOVA_WEBVIEW_FAILED;
    entry.errorText = message.length > 0 ? message : @"navigation failed";
}

static NSString *nova_webview_eval_sync(NovaWebViewEntry *entry, NSString *script, double timeout) {
    if (!entry || !entry.webView || !script) return @"";
    __block NSString *result = nil;
    __block BOOL done = NO;
    [entry.webView evaluateJavaScript:script
                   completionHandler:^(id value, NSError *error) {
        if ([value isKindOfClass:[NSString class]]) {
            result = value;
        } else if (value) {
            result = [value description];
        }
        if (error) result = @"";
        done = YES;
    }];
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:timeout];
    while (!done && [deadline timeIntervalSinceNow] > 0) {
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
                                 beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.01]];
    }
    return result ? result : @"";
}

@implementation NovaWebViewDelegate

- (void)webView:(WKWebView *)webView didStartProvisionalNavigation:(WKNavigation *)navigation {
    (void)webView;
    if (!self.entry) return;
    if (navigation && self.entry.currentNavigation &&
        navigation != self.entry.currentNavigation) return;
    self.entry.state = NOVA_WEBVIEW_LOADING;
    self.entry.errorText = nil;
}

- (void)webView:(WKWebView *)webView didCommitNavigation:(WKNavigation *)navigation {
    (void)webView;
    if (!self.entry) return;
    if (navigation && self.entry.currentNavigation &&
        navigation != self.entry.currentNavigation) return;
    self.entry.state = NOVA_WEBVIEW_LOADING;
}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation {
    (void)webView;
    if (!self.entry) return;
    if (navigation && self.entry.currentNavigation &&
        navigation != self.entry.currentNavigation) return;
    self.entry.state = NOVA_WEBVIEW_LOADED;
    self.entry.errorText = nil;
}- (void)webView:(WKWebView *)webView
 didFailProvisionalNavigation:(WKNavigation *)navigation
      withError:(NSError *)error {
    (void)webView;
    if (!self.entry) return;
    if (navigation && self.entry.currentNavigation &&
        navigation != self.entry.currentNavigation) return;
    self.entry.errorURL = self.entry.pendingURL;
    nova_webview_fail(self.entry, error.localizedDescription);
}- (void)webView:(WKWebView *)webView
 didFailNavigation:(WKNavigation *)navigation
      withError:(NSError *)error {
    (void)webView;
    if (!self.entry) return;
    if (navigation && self.entry.currentNavigation &&
        navigation != self.entry.currentNavigation) return;
    self.entry.errorURL = self.entry.pendingURL;
    nova_webview_fail(self.entry, error.localizedDescription);
}

- (void)webView:(WKWebView *)webView
decidePolicyForNavigationAction:(WKNavigationAction *)navigationAction
decisionHandler:(void (^)(WKNavigationActionPolicy))decisionHandler {
    (void)webView;
    NSString *scheme = navigationAction.request.URL.scheme;
    /* A blob: link has no response to load; WebKit cancels it as a scheme it
     * does not render. Treat it as a download request and let the page-side
     * bridge deliver the bytes. */
    if ([scheme.lowercaseString isEqualToString:@"blob"]) {
        decisionHandler(WKNavigationActionPolicyDownload);
        return;
    }
    if (!nova_webview_scheme_allowed(scheme)) {
        NSString *message = [NSString stringWithFormat:@"blocked unsupported scheme: %@", scheme ?: @"(none)"];
        nova_webview_fail(self.entry, message);
        decisionHandler(WKNavigationActionPolicyCancel);
        return;
    }
    decisionHandler(WKNavigationActionPolicyAllow);
}

/* WebKit asks before it renders a response. A response it cannot display is a
 * download, not an error page: hand it to WKDownload and let Nova list it. */
- (void)webView:(WKWebView *)webView
decidePolicyForNavigationResponse:(WKNavigationResponse *)navigationResponse
decisionHandler:(void (^)(WKNavigationResponsePolicy))decisionHandler {
    (void)webView;
    if (!navigationResponse.canShowMIMEType) {
        decisionHandler(WKNavigationResponsePolicyDownload);
        return;
    }
    decisionHandler(WKNavigationResponsePolicyAllow);
}

- (void)webView:(WKWebView *)webView
 navigationAction:(WKNavigationAction *)navigationAction
 didBecomeDownload:(WKDownload *)download {
    (void)navigationAction;
    nova_webview_register_download(webView, download);
}

- (void)webView:(WKWebView *)webView
 navigationResponse:(WKNavigationResponse *)navigationResponse
 didBecomeDownload:(WKDownload *)download {
    (void)navigationResponse;
    nova_webview_register_download(webView, download);
}

@end

/* A page can name its icon with <link rel="icon">; the app prefers that over
 * guessing /favicon.ico. The message carries the absolute href. */
static NSMutableDictionary<NSNumber *, NSString *> *nova_icon_urls = nil;

@interface NovaIconSink : NSObject <WKScriptMessageHandler>
@end

@implementation NovaIconSink
- (void)userContentController:(WKUserContentController *)controller
      didReceiveScriptMessage:(WKScriptMessage *)message {
    (void)controller;
    NSDictionary *body = [message.body isKindOfClass:[NSDictionary class]] ? message.body : nil;
    NSString *href = body[@"href"];
    if (![href isKindOfClass:[NSString class]] || href.length == 0) return;
    for (NSNumber *key in nova_webviews) {
        NovaWebViewEntry *entry = nova_webviews[key];
        if (entry.webView != message.webView) continue;
        if (!nova_icon_urls) nova_icon_urls = [NSMutableDictionary dictionary];
        nova_icon_urls[key] = href;
        return;
    }
}

@end

static NSString *nova_icon_script(void) {
    return @"(function(){"
           "function post(){"
           "var l=document.querySelector(\"link[rel~='icon'],link[rel='apple-touch-icon']\");"
           "if(l&&l.href)window.webkit.messageHandlers.novaIcon.postMessage({href:l.href});"
           "}"
           "if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',post);else post();"
           "})();";
}

/* A page reports a submitted password form. The value goes to the host, which
 * stores it in the macOS Keychain; a private tab is refused before it is kept. */
static NSString *nova_login_script(void) {
    return @"(function(){"
           "if(window.__novaLoginCapture)return;window.__novaLoginCapture=true;"
           "function userOf(f){"
           "var u=f.querySelector(\"input[autocomplete~='username'],input[type='email'],input[type='text'],input[type='tel']\");"
           "return u&&u.value?u.value:'';}"
           "document.addEventListener('submit',function(ev){"
           "var f=ev.target;if(!f||f.tagName!=='FORM')return;"
           "var p=f.querySelector(\"input[type='password']\");"
           "if(!p||!p.value)return;"
           "var l=location;if(l.protocol!=='https:'&&l.protocol!=='http:')return;"
           "window.webkit.messageHandlers.novaLogin.postMessage({origin:l.origin,user:userOf(f),pass:p.value});"
           "},true);})();";
}

@interface NovaLoginSink : NSObject <WKScriptMessageHandler>
@end

@implementation NovaLoginSink
- (void)userContentController:(WKUserContentController *)controller
      didReceiveScriptMessage:(WKScriptMessage *)message {
    (void)controller;
    NSDictionary *body = [message.body isKindOfClass:[NSDictionary class]] ? message.body : nil;
    NSString *origin = body[@"origin"];
    NSString *user = body[@"user"];
    NSString *pass = body[@"pass"];
    if (![origin isKindOfClass:[NSString class]] || origin.length == 0) return;
    if (![pass isKindOfClass:[NSString class]] || pass.length == 0) return;
    if (![user isKindOfClass:[NSString class]]) user = @"";
    for (NSNumber *key in nova_webviews) {
        NovaWebViewEntry *entry = nova_webviews[key];
        if (entry.webView != message.webView) continue;
        if (entry.privateView) return;
        entry.loginOrigin = origin;
        entry.loginUser = user;
        entry.loginPass = pass;
        return;
    }
}
@end

@interface NovaBlockList : NSObject
@end

@implementation NovaBlockList
@end

static WKContentRuleList *nova_content_rules = nil;

/* Compiles the JSON rules once and applies them to every live tab. WebKit
 * caches a compiled list by identifier, so relaunches are cheap. */
int64_t nova_webview_apply_blocklist(nova_str rules) {
    if (![NSThread isMainThread]) return 0;
    NSString *text = nova_webview_string(rules);
    if (text.length == 0) return 0;
    __block BOOL done = NO;
    __block WKContentRuleList *compiled = nil;
    [[WKContentRuleListStore defaultStore]
        compileContentRuleListForIdentifier:@"nova-blocklist"
                     encodedContentRuleList:text
                          completionHandler:^(WKContentRuleList *list, NSError *error) {
        (void)error;
        compiled = list;
        done = YES;
    }];
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:5.0];
    while (!done && [deadline timeIntervalSinceNow] > 0) {
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
                                 beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.01]];
    }
    if (!compiled) return 0;
    nova_content_rules = compiled;
    if (nova_webviews) {
        for (NSNumber *key in nova_webviews) {
            [nova_webviews[key].webView.configuration.userContentController
                addContentRuleList:compiled];
        }
    }
    return 1;
}

/* Creates a WKWebView. parent_handle is a NovaGuiView pointer from
 * nova_gui_native_view(); 0 creates an offscreen host for tests. Coordinates
 * are Nova points from the top-left; AppKit uses a bottom-left origin. */
static int64_t nova_webview_create_store(int64_t parent_handle, int64_t x, int64_t y,
                                         int64_t width, int64_t height, BOOL private) {
    if (width <= 0 || height <= 0) return 0;
    if (![NSThread isMainThread]) return 0;
    @autoreleasepool {
        [NSApplication sharedApplication];
        if (!nova_webviews) nova_webviews = [NSMutableDictionary dictionary];
        nova_webview_install_key_monitor();

        NSView *container = nil;
        NSView *host = nil;
        if (parent_handle != 0) {
            container = (__bridge NSView *)(void *)(intptr_t)parent_handle;
        } else {
            host = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, width, height)];
            container = host;
        }
        if (!container) return 0;

        WKWebViewConfiguration *configuration = [[WKWebViewConfiguration alloc] init];
        /* A private tab gets WebKit's non-persistent store: cookies, cache and
         * local storage live only as long as the view, and nothing is written
         * to the normal browsing store. */
        if (private) {
            configuration.websiteDataStore = [WKWebsiteDataStore nonPersistentDataStore];
        }
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = NO;
        WKUserContentController *userContent = configuration.userContentController;
        [userContent addScriptMessageHandler:[[NovaBlobSink alloc] init] name:@"novaBlob"];
        [userContent addUserScript:[[WKUserScript alloc] initWithSource:nova_blob_script()
                                                           injectionTime:WKUserScriptInjectionTimeAtDocumentStart
                                                        forMainFrameOnly:YES]];
        [userContent addScriptMessageHandler:[[NovaIconSink alloc] init] name:@"novaIcon"];
        [userContent addUserScript:[[WKUserScript alloc] initWithSource:nova_icon_script()
                                                           injectionTime:WKUserScriptInjectionTimeAtDocumentStart
                                                        forMainFrameOnly:YES]];
        [userContent addScriptMessageHandler:[[NovaLoginSink alloc] init] name:@"novaLogin"];
        [userContent addUserScript:[[WKUserScript alloc] initWithSource:nova_login_script()
                                                           injectionTime:WKUserScriptInjectionTimeAtDocumentStart
                                                        forMainFrameOnly:YES]];
        if (nova_content_rules) [userContent addContentRuleList:nova_content_rules];
        WKWebView *web = [[WKWebView alloc] initWithFrame:NSMakeRect(0, 0, width, height)
                                          configuration:configuration];
        if (!web) return 0;
        web.allowsBackForwardNavigationGestures = YES;
        web.autoresizingMask = NSViewNotSizable;
        /* Google refuses OAuth in an embedded WebView unless the agent looks
         * like Safari; without this its "browser may not be secure" page is a
         * dead end for sign-in. The engine underneath is the same WebKit, so a
         * Safari token is accurate about the renderer, and NOVA_BROWSER_UA can
         * override it with the full browser's own string. */
        NSString *userAgent = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_BROWSER_UA"];
        if (userAgent.length == 0) {
            userAgent = @"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                        @"AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.4 Safari/605.1.15";
        }
        web.customUserAgent = userAgent;

        NovaWebViewEntry *entry = [[NovaWebViewEntry alloc] init];
        entry.webView = web;
        entry.hostView = host;
        entry.privateView = private;
        entry.state = NOVA_WEBVIEW_EMPTY;
        NovaWebViewDelegate *delegate = [[NovaWebViewDelegate alloc] init];
        delegate.entry = entry;
        entry.delegate = delegate;
        web.navigationDelegate = delegate;

        [container addSubview:web];
        int64_t handle = nova_next_handle++;
        nova_webviews[@(handle)] = entry;
        if (parent_handle != 0) {
            /* Reuse set_bounds so top-left conversion lives in one place. */
            extern void nova_webview_set_bounds(int64_t, int64_t, int64_t, int64_t, int64_t);
            nova_webview_set_bounds(handle, x, y, width, height);
        }
        return handle;
    }
}

int64_t nova_webview_create(int64_t parent_handle, int64_t x, int64_t y,
                            int64_t width, int64_t height) {
    return nova_webview_create_store(parent_handle, x, y, width, height, NO);
}

int64_t nova_webview_create_private(int64_t parent_handle, int64_t x, int64_t y,
                                    int64_t width, int64_t height) {
    return nova_webview_create_store(parent_handle, x, y, width, height, YES);
}

void nova_webview_set_bounds(int64_t handle, int64_t x, int64_t y,
                             int64_t width, int64_t height) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || width <= 0 || height <= 0) return;
    @autoreleasepool {
        NSView *parent = entry.webView.superview;
        if (!parent) return;
        if (entry.hostView) {
            entry.hostView.frame = NSMakeRect(0, 0, width, height);
            entry.webView.frame = NSMakeRect(0, 0, width, height);
            return;
        }
        CGFloat bottom = parent.bounds.size.height - (CGFloat)y - (CGFloat)height;
        entry.webView.frame = NSMakeRect((CGFloat)x, bottom, (CGFloat)width, (CGFloat)height);
    }
}

/* Hands keyboard focus back to the Nova chrome so an address field can accept
 * text. This is a view-focus operation only; it does not touch page content. */
void nova_webview_focus_chrome(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView.window || !entry.webView.superview) return;
    [entry.webView.window makeFirstResponder:entry.webView.superview];
}

void nova_webview_focus_page(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView.window) return;
    [entry.webView.window makeFirstResponder:entry.webView];
}

int64_t nova_webview_page_focused(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView.window) return 0;
    NSResponder *first = entry.webView.window.firstResponder;
    if (first == (NSResponder *)entry.webView) return 1;
    if ([first isKindOfClass:[NSView class]] &&
        [(NSView *)first isDescendantOf:entry.webView]) {
        return 1;
    }
    return 0;
}

int64_t nova_webview_take_chrome_focus_request(int64_t handle) {
    if (!nova_webview_entry(handle) || !nova_chrome_focus_request) return 0;
    nova_chrome_focus_request = NO;
    return 1;
}

/* Encoded as code * 16 + modifier bits. Zero means no queued shortcut. */
int64_t nova_webview_take_shortcut(int64_t handle) {
    if (!nova_webview_entry(handle) || nova_pending_shortcut == 0) return 0;
    int64_t encoded = nova_pending_shortcut * 16 + (nova_pending_mods & 0xF);
    nova_pending_shortcut = 0;
    nova_pending_mods = 0;
    return encoded;
}

/* Find-in-page. WKFindResult reports whether something matched, not how many,
 * so the chrome says Found / Not found instead of inventing a match count. */
void nova_webview_find(int64_t handle, nova_str query, int64_t forward) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView) return;
    NSString *text = nova_webview_string(query);
    if (text.length == 0) {
        entry.findState = 0;
        [entry.webView evaluateJavaScript:@"window.getSelection().removeAllRanges()"
                        completionHandler:nil];
        return;
    }
    entry.findState = 3;
    WKFindConfiguration *config = [[WKFindConfiguration alloc] init];
    config.backwards = (forward == 0);
    config.wraps = YES;
    config.caseSensitive = NO;
    __weak NovaWebViewEntry *weakEntry = entry;
    [entry.webView findString:text
           withConfiguration:config
           completionHandler:^(WKFindResult *result) {
        NovaWebViewEntry *strongEntry = weakEntry;
        if (strongEntry) strongEntry.findState = result.matchFound ? 1 : 2;
    }];
}

int64_t nova_webview_find_state(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return entry ? entry.findState : 0;
}

/* Page zoom is WebKit's own pageZoom, so text scales the way the page expects
 * instead of the framebuffer being stretched. */
void nova_webview_set_zoom(int64_t handle, int64_t percent) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView) return;
    if (percent < 25) percent = 25;
    if (percent > 300) percent = 300;
    if (@available(macOS 11.0, *)) {
        entry.webView.pageZoom = (CGFloat)percent / 100.0;
    }
}

int64_t nova_webview_zoom(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView) return 100;
    if (@available(macOS 11.0, *)) {
        return (int64_t)llround(entry.webView.pageZoom * 100.0);
    }
    return 100;
}

/* 1 when the tab writes cookies and cache to disk, 0 for a private tab. This
 * reports WebKit's own store, not Nova's intent. */
int64_t nova_webview_store_persistent(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView) return 0;
    return entry.webView.configuration.websiteDataStore.isPersistent ? 1 : 0;
}

/* Pages follow the chrome's appearance through prefers-color-scheme. Setting
 * it on each WKWebView keeps that true even before a view is in a window (the
 * headless harness creates offscreen views). */
void nova_webview_set_appearance_all(int64_t dark) {
    if (!nova_webviews) return;
    NSAppearanceName name = dark ? NSAppearanceNameDarkAqua : NSAppearanceNameAqua;
    NSAppearance *appearance = [NSAppearance appearanceNamed:name];
    for (NSNumber *key in nova_webviews) {
        NovaWebViewEntry *entry = nova_webviews[key];
        entry.webView.appearance = appearance;
    }
}

void nova_webview_destroy(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return;
    @autoreleasepool {
        [entry.webView stopLoading];
        entry.webView.navigationDelegate = nil;
        [entry.webView removeFromSuperview];
        entry.delegate.entry = nil;
        [nova_webviews removeObjectForKey:@(handle)];
    }
}

void nova_webview_load_html(int64_t handle, nova_str html, nova_str base_url) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return;
    @autoreleasepool {
        NSString *text = nova_webview_string(html);
        NSString *base = nova_webview_string(base_url);
        if (base.length == 0) base = @"https://fixture.nova.invalid/";
        NSURL *url = [NSURL URLWithString:base];
        if (!url || !nova_webview_scheme_allowed(url.scheme)) {
            nova_webview_fail(entry, @"invalid fixture base URL");
            return;
        }
        entry.state = NOVA_WEBVIEW_LOADING;
        entry.errorText = nil;
        entry.errorURL = nil;
        entry.titleFallback = nil;
        entry.pendingURL = base;
        entry.generation = entry.generation + 1;
        entry.currentNavigation = [entry.webView loadHTMLString:text baseURL:url];
    }
}

void nova_webview_navigate(int64_t handle, nova_str address) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return;
    @autoreleasepool {
        NSString *text = nova_webview_string(address);
        NSURL *url = [NSURL URLWithString:text];
        if (!url || !nova_webview_scheme_allowed(url.scheme)) {
            nova_webview_fail(entry, @"only http, https and about URLs are allowed");
            return;
        }
        entry.state = NOVA_WEBVIEW_LOADING;
        entry.errorText = nil;
        entry.errorURL = nil;
        entry.titleFallback = nil;
        entry.pendingURL = text;
        entry.generation = entry.generation + 1;
        entry.currentNavigation = [entry.webView loadRequest:[NSURLRequest requestWithURL:url]];
    }
}

void nova_webview_back(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (entry && entry.webView.canGoBack) {
        entry.state = NOVA_WEBVIEW_LOADING;
        entry.errorURL = nil;
        entry.pendingURL = entry.webView.backForwardList.backItem.URL.absoluteString ?: entry.pendingURL;
        entry.generation = entry.generation + 1;
        entry.currentNavigation = [entry.webView goBack];
    }
}

void nova_webview_forward(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (entry && entry.webView.canGoForward) {
        entry.state = NOVA_WEBVIEW_LOADING;
        entry.errorURL = nil;
        entry.pendingURL = entry.webView.backForwardList.forwardItem.URL.absoluteString ?: entry.pendingURL;
        entry.generation = entry.generation + 1;
        entry.currentNavigation = [entry.webView goForward];
    }
}

void nova_webview_reload(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return;
    entry.state = NOVA_WEBVIEW_LOADING;
    entry.errorURL = nil;
    if (entry.webView.URL.absoluteString.length > 0) entry.pendingURL = entry.webView.URL.absoluteString;
    entry.generation = entry.generation + 1;
    entry.currentNavigation = [entry.webView reload];
}

void nova_webview_stop(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return;
    [entry.webView stopLoading];
    entry.state = NOVA_WEBVIEW_LOADED;
}

int64_t nova_webview_can_go_back(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return entry && entry.webView.canGoBack ? 1 : 0;
}

int64_t nova_webview_can_go_forward(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return entry && entry.webView.canGoForward ? 1 : 0;
}

int64_t nova_webview_state(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return entry ? entry.state : NOVA_WEBVIEW_INVALID;
}

int64_t nova_webview_generation(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return entry ? entry.generation : NOVA_WEBVIEW_INVALID;
}

void nova_webview_set_visible(int64_t handle, int64_t visible) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return;
    entry.webView.hidden = visible ? NO : YES;
}

int64_t nova_webview_wait(int64_t handle, int64_t timeout_ms) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return NOVA_WEBVIEW_INVALID;
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:(NSTimeInterval)timeout_ms / 1000.0];
    while (entry.state == NOVA_WEBVIEW_LOADING && [deadline timeIntervalSinceNow] > 0) {
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
                                 beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.01]];
    }
    return entry.state;
}

nova_str nova_webview_title(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return nova_webview_empty();
    @autoreleasepool {
        NSString *title = entry.webView.title;
        if (title.length == 0) {
            if (entry.titleFallback.length == 0) {
                entry.titleFallback = nova_webview_eval_sync(
                    entry, @"document.title ? document.title : ''", 0.5);
            }
            title = entry.titleFallback;
        }
        return nova_webview_copy(title);
    }
}

nova_str nova_webview_url(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return nova_webview_copy(entry ? entry.webView.URL.absoluteString : nil);
}

nova_str nova_webview_error(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return nova_webview_copy(entry ? entry.errorText : nil);
}

/* The address the failed load was aimed at. WKWebView keeps its previous URL
 * after a provisional failure, so the error panel needs the pending one. */
nova_str nova_webview_error_url(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return nova_webview_copy(entry ? entry.errorURL : nil);
}

/* WebKit's own load estimate, 0..1000. Polled rather than observed so the ABI
 * stays callback-free. */
int64_t nova_webview_progress(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return 0;
    double progress = entry.webView.estimatedProgress;
    if (progress < 0.0) progress = 0.0;
    if (progress > 1.0) progress = 1.0;
    return (int64_t)lround(progress * 1000.0);
}

static NSString *nova_ns_string(nova_str value) {
    if (!value.ptr || value.len <= 0) return @"";
    NSString *text = [[NSString alloc] initWithBytes:value.ptr
                                              length:(NSUInteger)value.len
                                            encoding:NSUTF8StringEncoding];
    return text ?: @"";
}

/* Runs a script in a page and returns its value as text. Used by the blob
 * check and by any headless probe that needs the page's own JS. */
nova_str nova_webview_run_js(int64_t handle, nova_str script) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return nova_webview_empty();
    return nova_webview_copy(nova_webview_eval_sync(entry, nova_ns_string(script), 4.0));
}

nova_str nova_webview_body_text(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return nova_webview_empty();
    return nova_webview_copy(nova_webview_eval_sync(
        entry, @"document.body ? document.body.innerText : ''", 2.0));
}

nova_str nova_webview_engine(void) {
    return nova_webview_copy(@"WebKit / WKWebView");
}

/* Downloads are window-level: a download belongs to the browser, not to the tab
 * that started it, and survives switching tabs. */
int64_t nova_webview_download_count(void) {
    return nova_downloads ? (int64_t)nova_downloads.count : 0;
}

int64_t nova_webview_download_id_at(int64_t index) {
    if (!nova_downloads || index < 0 || index >= (int64_t)nova_downloads.count) return 0;
    return nova_downloads[(NSUInteger)index].ident;
}

nova_str nova_webview_download_name(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    return nova_webview_copy(entry ? entry.name : nil);
}

nova_str nova_webview_download_path(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    return nova_webview_copy(entry ? entry.path : nil);
}

int64_t nova_webview_download_received(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    if (!entry) return 0;
    if (entry.state == 2) return entry.expected > 0 ? entry.expected : 0;
    NSDictionary *attrs = [[NSFileManager defaultManager] attributesOfItemAtPath:entry.path error:NULL];
    return attrs ? (int64_t)[attrs fileSize] : 0;
}

int64_t nova_webview_download_expected(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    return entry ? entry.expected : 0;
}

int64_t nova_webview_download_state(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    return entry ? entry.state : 0 - 1;
}

int64_t nova_webview_download_cancel(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    if (!entry || entry.state == 2 || entry.state == 4) return 0;
    entry.state = 4;
    entry.resumeData = nil;
    [entry.download cancel:^(NSData *resumeData) {
        (void)resumeData;
    }];
    return 1;
}

/* Pause keeps the partial file and WebKit's resume data, so resuming continues
 * from the byte the server acknowledged instead of restarting the transfer. */
int64_t nova_webview_download_pause(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    if (!entry || entry.state != 1) return 0;
    entry.state = 5;
    entry.resumeData = nil;
    [entry.download cancel:^(NSData *resumeData) {
        entry.resumeData = resumeData;
    }];
    return 1;
}

int64_t nova_webview_download_resumable(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    return (entry && entry.resumeData.length > 0 && entry.webView) ? 1 : 0;
}

int64_t nova_webview_download_resume(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    if (!entry || entry.resumeData.length == 0) return 0;
    if (entry.state == 1 || entry.state == 2 || entry.state == 4) return 0;
    WKWebView *view = entry.webView;
    if (!view) return 0;
    NSData *resumeData = entry.resumeData;
    entry.resumeData = nil;
    entry.state = 1;
    [view resumeDownloadFromResumeData:resumeData completionHandler:^(WKDownload *download) {
        if (!download) {
            entry.state = 3;
            return;
        }
        entry.download = download;
        download.delegate = entry;
        entry.state = 1;
    }];
    return 1;
}

int64_t nova_webview_download_reveal(int64_t ident) {
    NovaWebDownload *entry = nova_webview_download(ident);
    if (!entry || entry.path.length == 0) return 0;
    [[NSWorkspace sharedWorkspace] activateFileViewerSelectingURLs:@[[NSURL fileURLWithPath:entry.path]]];
    return 1;
}

/* Finished and cancelled entries leave the shelf; running ones stay. */
void nova_webview_download_clear_finished(void) {
    if (!nova_downloads) return;
    NSMutableArray *keep = [NSMutableArray array];
    for (NovaWebDownload *entry in nova_downloads) {
        if (entry.state == 0 || entry.state == 1 || entry.state == 5) {
            [keep addObject:entry];
        } else if (entry.state == 3 && entry.resumeData.length > 0) {
            [keep addObject:entry];
        }
    }
    nova_downloads = keep;
}

/* Writes a PNG snapshot of the current WebKit view. Returns 1 on success.
 * This is used by evidence tests and diagnostics; the Nova framebuffer PNG
 * cannot contain native subviews. */
int64_t nova_webview_snapshot(int64_t handle, nova_str path) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || !entry.webView || !path.ptr || path.len <= 0) return 0;
    @autoreleasepool {
        __block NSImage *image = nil;
        __block BOOL done = NO;
        WKSnapshotConfiguration *configuration = [[WKSnapshotConfiguration alloc] init];
        [entry.webView takeSnapshotWithConfiguration:configuration
                                   completionHandler:^(NSImage *snapshot, NSError *error) {
            if (!error) image = snapshot;
            done = YES;
        }];
        NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:5.0];
        while (!done && [deadline timeIntervalSinceNow] > 0) {
            [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
                                     beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.01]];
        }
        if (!image) return 0;
        NSData *tiff = image.TIFFRepresentation;
        if (!tiff) return 0;
        NSBitmapImageRep *bitmap = [NSBitmapImageRep imageRepWithData:tiff];
        if (!bitmap) return 0;
        NSData *png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
        if (!png) return 0;
        NSString *target = nova_webview_string(path);
        return [png writeToFile:target atomically:YES] ? 1 : 0;
    }
}

/* The icon href the page announced for this tab, or "" when it announced none. */
nova_str nova_webview_icon_url(int64_t handle) {
    NSString *href = nova_icon_urls ? nova_icon_urls[@(handle)] : nil;
    return nova_webview_copy(href);
}

/* ---------- saved logins ---------- */

int64_t nova_webview_login_pending(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return (entry && entry.loginPass.length > 0) ? 1 : 0;
}

nova_str nova_webview_login_origin(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return nova_webview_copy(entry ? entry.loginOrigin : nil);
}

nova_str nova_webview_login_user(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return nova_webview_copy(entry ? entry.loginUser : nil);
}

nova_str nova_webview_login_pass(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    return nova_webview_copy(entry ? entry.loginPass : nil);
}

void nova_webview_login_clear(int64_t handle) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry) return;
    entry.loginOrigin = nil;
    entry.loginUser = nil;
    entry.loginPass = nil;
}

static NSString *nova_js_quote(NSString *value) {
    NSMutableString *out = [NSMutableString stringWithString:@"'"];
    for (NSUInteger i = 0; i < value.length; i++) {
        unichar ch = [value characterAtIndex:i];
        if (ch == '\\') {
            [out appendString:@"\\\\"];
        } else if (ch == '\'') {
            [out appendString:@"\\'"];
        } else if (ch == '\n') {
            [out appendString:@"\\n"];
        } else if (ch == '\r') {
            [out appendString:@"\\r"];
        } else {
            [out appendFormat:@"%C", ch];
        }
    }
    [out appendString:@"'"];
    return out;
}

/* Fills the page's own password form from the Keychain. Only the visible
 * fields change: Nova never submits on the user's behalf, and an already
 * filled form is left alone so a page's own manager wins. */
int64_t nova_webview_fill_login(int64_t handle, nova_str user, nova_str pass) {
    NovaWebViewEntry *entry = nova_webview_entry(handle);
    if (!entry || entry.privateView) return 0;
    NSString *script = [NSString stringWithFormat:
        @"(function(u,p){"
         "var pf=document.querySelector(\"input[type='password']\");"
         "if(!pf)return 'nofield';if(pf.value)return 'already';"
         "var f=pf.form||document;var ui=null;var list=f.querySelectorAll('input');"
         "for(var i=0;i<list.length;i++){if(list[i]===pf)break;"
         "if(/^(text|email|tel)$/.test(list[i].type))ui=list[i];}"
         "function set(el,v){if(!el)return;var d=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value');"
         "if(d&&d.set)d.set.call(el,v);else el.value=v;"
         "el.dispatchEvent(new Event('input',{bubbles:true}));"
         "el.dispatchEvent(new Event('change',{bubbles:true}));}"
         "set(ui,u);set(pf,p);return 'filled';})(%@,%@)",
        nova_js_quote(nova_webview_string(user)),
        nova_js_quote(nova_webview_string(pass))];
    NSString *result = nova_webview_eval_sync(entry, script, 2.0);
    return [result isEqualToString:@"filled"] ? 1 : 0;
}

/* One generic-password item per host in the login keychain, under Nova's own
 * service. The value is Nova's escaped `user<TAB>password`. */
static NSString * const NOVA_LOGIN_SERVICE = @"dev.nova.browser.login";

int64_t nova_keychain_set(nova_str account, nova_str secret) {
    NSString *acct = nova_webview_string(account);
    NSData *data = [nova_webview_string(secret) dataUsingEncoding:NSUTF8StringEncoding];
    if (acct.length == 0 || !data) return 0;
    NSDictionary *query = @{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
                            (__bridge id)kSecAttrService:NOVA_LOGIN_SERVICE,
                            (__bridge id)kSecAttrAccount:acct};
    NSDictionary *update = @{(__bridge id)kSecValueData:data};
    OSStatus status = SecItemUpdate((__bridge CFDictionaryRef)query,
                                    (__bridge CFDictionaryRef)update);
    if (status == errSecItemNotFound) {
        NSMutableDictionary *add = [query mutableCopy];
        add[(__bridge id)kSecValueData] = data;
        add[(__bridge id)kSecAttrAccessible] = (__bridge id)kSecAttrAccessibleWhenUnlocked;
        status = SecItemAdd((__bridge CFDictionaryRef)add, NULL);
    }
    return status == errSecSuccess ? 1 : 0;
}

nova_str nova_keychain_get(nova_str account) {
    NSString *acct = nova_webview_string(account);
    if (acct.length == 0) return nova_webview_empty();
    NSDictionary *query = @{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
                            (__bridge id)kSecAttrService:NOVA_LOGIN_SERVICE,
                            (__bridge id)kSecAttrAccount:acct,
                            (__bridge id)kSecReturnData:@YES,
                            (__bridge id)kSecMatchLimit:(__bridge id)kSecMatchLimitOne};
    CFTypeRef found = NULL;
    OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &found);
    if (status != errSecSuccess || !found) return nova_webview_empty();
    NSData *data = (__bridge_transfer NSData *)found;
    NSString *text = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    return nova_webview_copy(text);
}

/* Every host with a saved login, sorted, newline-joined. Nova reads this once
 * when the passwords panel opens so drawing never touches Security.framework. */
nova_str nova_keychain_list(void) {
    NSDictionary *query = @{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
                            (__bridge id)kSecAttrService:NOVA_LOGIN_SERVICE,
                            (__bridge id)kSecReturnAttributes:@YES,
                            (__bridge id)kSecMatchLimit:(__bridge id)kSecMatchLimitAll};
    CFTypeRef found = NULL;
    OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &found);
    if (status != errSecSuccess || !found) return nova_webview_empty();
    NSArray *items = (__bridge_transfer NSArray *)found;
    NSMutableArray *accounts = [NSMutableArray array];
    for (NSDictionary *item in items) {
        id account = item[(__bridge id)kSecAttrAccount];
        if ([account isKindOfClass:[NSString class]] && [account length] > 0) {
            [accounts addObject:account];
        }
    }
    [accounts sortUsingSelector:@selector(localizedStandardCompare:)];
    return nova_webview_copy([accounts componentsJoinedByString:@"\n"]);
}

int64_t nova_keychain_delete(nova_str account) {
    NSString *acct = nova_webview_string(account);
    if (acct.length == 0) return 0;
    NSDictionary *query = @{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
                            (__bridge id)kSecAttrService:NOVA_LOGIN_SERVICE,
                            (__bridge id)kSecAttrAccount:acct};
    OSStatus status = SecItemDelete((__bridge CFDictionaryRef)query);
    return (status == errSecSuccess || status == errSecItemNotFound) ? 1 : 0;
}

/* ---------- Chrome password import ---------- */

/* Chrome's macOS password store is `v10` plus AES-128-CBC. The key is
 * PBKDF2-HMAC-SHA1 of the "Chrome Safe Storage" keychain password over
 * "saltysalt" with 1003 iterations, and the IV is sixteen spaces. Anything
 * unexpected (another prefix, a bad pad, a missing key) returns "" rather than
 * a guessed plaintext. */
static NSData *nova_hex_data(NSString *hex) {
    if (hex.length == 0 || hex.length % 2 != 0) return nil;
    NSUInteger length = hex.length / 2;
    NSMutableData *data = [NSMutableData dataWithLength:length];
    unsigned char *bytes = data.mutableBytes;
    for (NSUInteger i = 0; i < length; i++) {
        unsigned int value = 0;
        if (sscanf([[hex substringWithRange:NSMakeRange(i * 2, 2)] UTF8String], "%02x", &value) != 1) {
            return nil;
        }
        bytes[i] = (unsigned char)value;
    }
    return data;
}

static NSData *nova_chrome_derive(NSString *password) {
    const char *pass = password.UTF8String;
    if (!pass) return nil;
    unsigned char key[kCCKeySizeAES128];
    int status = CCKeyDerivationPBKDF(kCCPBKDF2, pass, strlen(pass),
                                      (const uint8_t *)"saltysalt", 9,
                                      kCCPRFHmacAlgSHA1, 1003,
                                      key, sizeof(key));
    if (status != kCCSuccess) return nil;
    return [NSData dataWithBytes:key length:sizeof(key)];
}

nova_str nova_chrome_decrypt(nova_str hex, nova_str password) {
    NSData *blob = nova_hex_data(nova_webview_string(hex));
    NSString *pass = nova_webview_string(password);
    if (blob.length <= 3 || pass.length == 0) return nova_webview_empty();
    NSData *prefix = [@"v10" dataUsingEncoding:NSUTF8StringEncoding];
    if (![[blob subdataWithRange:NSMakeRange(0, 3)] isEqualToData:prefix]) {
        return nova_webview_empty();
    }
    NSData *key = nova_chrome_derive(pass);
    if (!key) return nova_webview_empty();
    NSData *cipher = [blob subdataWithRange:NSMakeRange(3, blob.length - 3)];
    if (cipher.length == 0 || cipher.length % kCCBlockSizeAES128 != 0) {
        return nova_webview_empty();
    }
    unsigned char iv[kCCBlockSizeAES128];
    memset(iv, ' ', sizeof(iv));
    size_t out_len = 0;
    NSMutableData *out = [NSMutableData dataWithLength:cipher.length + kCCBlockSizeAES128];
    CCCryptorStatus status = CCCrypt(kCCDecrypt, kCCAlgorithmAES128, 0,
                                     key.bytes, key.length, iv,
                                     cipher.bytes, cipher.length,
                                     out.mutableBytes, out.length, &out_len);
    if (status != kCCSuccess || out_len == 0) return nova_webview_empty();
    unsigned char *bytes = out.mutableBytes;
    NSUInteger pad = bytes[out_len - 1];
    if (pad == 0 || pad > kCCBlockSizeAES128 || pad > out_len) return nova_webview_empty();
    for (NSUInteger i = 0; i < pad; i++) {
        if (bytes[out_len - 1 - i] != pad) return nova_webview_empty();
    }
    out.length = out_len - pad;
    NSString *text = [[NSString alloc] initWithData:out encoding:NSUTF8StringEncoding];
    return nova_webview_copy(text);
}

/* Chrome's key, from its own keychain item. macOS may ask before handing it
 * over; a refusal is an empty key, and the import then reports a lock instead
 * of guessing. The environment overrides exist for tests and for a key the
 * user keeps outside the keychain. */
nova_str nova_chrome_key(void) {
    const char *forced = getenv("NOVA_BROWSER_CHROME_KEY");
    if (forced) return nova_webview_copy([NSString stringWithUTF8String:forced]);
    if (getenv("NOVA_BROWSER_NO_CHROME_KEY")) return nova_webview_empty();
    NSDictionary *query = @{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
                            (__bridge id)kSecAttrService:@"Chrome Safe Storage",
                            (__bridge id)kSecAttrAccount:@"Chrome",
                            (__bridge id)kSecReturnData:@YES,
                            (__bridge id)kSecMatchLimit:(__bridge id)kSecMatchLimitOne};
    CFTypeRef found = NULL;
    if (SecItemCopyMatching((__bridge CFDictionaryRef)query, &found) != errSecSuccess || !found) {
        return nova_webview_empty();
    }
    NSData *data = (__bridge_transfer NSData *)found;
    NSString *text = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    return nova_webview_copy(text);
}
