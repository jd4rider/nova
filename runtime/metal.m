/*
 * nova-metal — a thin native host for the Nova engine renderer.
 *
 * Metal objects stay here; Nova owns the loop, ordering and lifetime decisions.
 * The FFI has no structs or pointers, so the boundary is Int handles, Ints,
 * strings and a polled running flag. There are no callbacks.
 *
 * This first slice owns exactly one session. `nova_metal_init` returns handle 1
 * or 0; every other entry validates the handle. A per-window handle table is
 * only needed once the editor can open more than one viewport, and is recorded
 * as future work rather than pretended here.
 *
 * The build compiles this file only when a program references nova_metal_
 * symbols (see examples/nova-tool/src/main.nova), so non-render programs never
 * link Metal.
 */

#import <Cocoa/Cocoa.h>
#import <Metal/Metal.h>
#import <MetalKit/MetalKit.h>
#import <ImageIO/ImageIO.h>
#import <QuartzCore/QuartzCore.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef struct {
    const char *ptr;
    int64_t len;
} nova_str;

#define NOVA_METAL_SESSION 1

static char nova_metal_error[1024];
static id<MTLDevice> nova_device = nil;
static id<MTLCommandQueue> nova_queue = nil;
static id<MTLRenderPipelineState> nova_pipeline = nil;
static id<MTLDepthStencilState> nova_depth_state = nil;
static id<MTLBuffer> nova_vertices = nil;
static id<MTLBuffer> nova_cube_vertices = nil;
static id<MTLBuffer> nova_lit_vertices = nil;
static id<MTLTexture> nova_texture = nil;
static int nova_geometry = 0; /* 0 quad, 1 colour cube, 2 textured lit cube */
/* Direction toward the directional light (xyz) and whether lighting is on
 * (w). The host converts the Q16.16 direction; the shader normalizes it. */
static float nova_light[4] = {0.45f, 0.85f, 0.55f, 0.0f};
/* Row-major Q16.16 MVP arrives as 16 Ints; this is its column-major float
 * form, ready for a Metal float4x4. Identity until Nova sets a camera. */
static float nova_mvp[16] = {
    1.0f, 0.0f, 0.0f, 0.0f,
    0.0f, 1.0f, 0.0f, 0.0f,
    0.0f, 0.0f, 1.0f, 0.0f,
    0.0f, 0.0f, 0.0f, 1.0f,
};
static NSWindow *nova_window = nil;
static MTKView *nova_view = nil;
static int nova_running = 0;
static int nova_ready = 0;
/* Accumulated camera input. Nova polls these deltas and clears them after a
 * frame; no event callback crosses the FFI. */
static int64_t nova_input_orbit_dx = 0;
static int64_t nova_input_orbit_dy = 0;
static int64_t nova_input_pan_dx = 0;
static int64_t nova_input_pan_dy = 0;
static int64_t nova_input_scroll = 0;

static void nova_metal_fail(NSString *message) {
    const char *text = message ? message.UTF8String : "unknown Metal error";
    if (!text) text = "unknown Metal error";
    snprintf(nova_metal_error, sizeof(nova_metal_error), "%s", text);
}

static nova_str nova_metal_copy(const char *text) {
    if (!text) text = "";
    size_t length = strlen(text);
    char *copy = (char *)malloc(length + 1);
    if (!copy) {
        nova_str empty = {"", 0};
        return empty;
    }
    memcpy(copy, text, length + 1);
    nova_str result = {copy, (int64_t)length};
    return result;
}

/* Vertex layout matches the shader below: float3 position, float2 UV,
 * float4 colour, float3 normal. Twelve floats is 48 bytes, so the packed
 * types in the shader have the same stride with no hidden padding. The colour
 * is a tint; the textured quad uses white, the colour cube uses per-face
 * colours, and the lit cube uses white plus per-face normals. */
typedef struct {
    float x;
    float y;
    float z;
    float u;
    float v;
    float r;
    float g;
    float b;
    float a;
    float nx;
    float ny;
    float nz;
} NovaMetalVertex;

#define NOVA_METAL_QUAD_VERTICES 6
#define NOVA_METAL_CUBE_VERTICES 36
#define NOVA_METAL_LIT_VERTICES 36

static const char *nova_metal_shader =
    "#include <metal_stdlib>\n"
    "using namespace metal;\n"
    "struct VIn { packed_float3 pos; packed_float2 uv; packed_float4 colour; packed_float3 normal; };\n"
    "struct VOut { float4 position [[position]]; float2 uv; float4 colour; float3 normal; };\n"
    "vertex VOut nova_vertex(uint vid [[vertex_id]], constant VIn *verts [[buffer(0)]], constant float4x4 &mvp [[buffer(1)]]) {\n"
    "    VOut out;\n"
    "    float4 clip = mvp * float4(float3(verts[vid].pos), 1.0);\n"
    "    clip.z = clip.z * 0.5 + clip.w * 0.5;\n"
    "    out.position = clip;\n"
    "    out.uv = float2(verts[vid].uv);\n"
    "    out.colour = float4(verts[vid].colour);\n"
    "    out.normal = float3(verts[vid].normal);\n"
    "    return out;\n"
    "}\n"
    "fragment float4 nova_fragment(VOut in [[stage_in]], texture2d<float> tex [[texture(0)]], constant float4 &light [[buffer(0)]]) {\n"
    "    constexpr sampler s(address::clamp_to_edge, filter::linear);\n"
    "    float4 base = in.colour * tex.sample(s, in.uv);\n"
    "    if (light.w > 0.5) {\n"
    "        float3 n = normalize(in.normal);\n"
    "        float3 l = normalize(light.xyz);\n"
    "        float diffuse = max(dot(n, l), 0.0);\n"
    "        base.rgb = base.rgb * (0.30 + 0.70 * diffuse);\n"
    "    }\n"
    "    return base;\n"
    "}\n";

@interface NovaMetalView : MTKView
@end

@implementation NovaMetalView
- (BOOL)acceptsFirstResponder {
    return YES;
}
- (BOOL)isOpaque {
    return YES;
}
- (BOOL)acceptsFirstMouse:(NSEvent *)event {
    (void)event;
    return YES;
}
- (void)mouseDown:(NSEvent *)event {
    (void)event;
    [self.window makeFirstResponder:self];
}
- (void)rightMouseDown:(NSEvent *)event {
    (void)event;
    [self.window makeFirstResponder:self];
}
- (void)mouseDragged:(NSEvent *)event {
    int64_t dx = (int64_t)llround(event.deltaX);
    int64_t dy = (int64_t)llround(event.deltaY);
    if (event.modifierFlags & NSEventModifierFlagShift) {
        nova_input_pan_dx += dx;
        nova_input_pan_dy += dy;
    } else {
        nova_input_orbit_dx += dx;
        nova_input_orbit_dy += dy;
    }
}
- (void)rightMouseDragged:(NSEvent *)event {
    nova_input_pan_dx += (int64_t)llround(event.deltaX);
    nova_input_pan_dy += (int64_t)llround(event.deltaY);
}
- (void)scrollWheel:(NSEvent *)event {
    nova_input_scroll += (int64_t)llround(event.scrollingDeltaY);
}
@end

@interface NovaMetalRenderer : NSObject <MTKViewDelegate>
@end

static id<MTLBuffer> nova_metal_vertex_buffer(void) {
    if (nova_geometry == 2) return nova_lit_vertices;
    if (nova_geometry == 1) return nova_cube_vertices;
    return nova_vertices;
}

static int nova_metal_vertex_count(void) {
    if (nova_geometry == 2) return NOVA_METAL_LIT_VERTICES;
    if (nova_geometry == 1) return NOVA_METAL_CUBE_VERTICES;
    return NOVA_METAL_QUAD_VERTICES;
}

@implementation NovaMetalRenderer

/* MTKView owns drawable acquisition and presentation. Doing that by hand
 * returns the same drawable after it was already presented, which Metal
 * rejects. The delegate renders exactly one frame per `[view draw]`. */
- (void)drawInMTKView:(MTKView *)view {
    MTLRenderPassDescriptor *pass = view.currentRenderPassDescriptor;
    id<CAMetalDrawable> drawable = view.currentDrawable;
    if (!pass || !drawable) return;
    id<MTLCommandBuffer> buffer = [nova_queue commandBuffer];
    id<MTLRenderCommandEncoder> encoder = [buffer renderCommandEncoderWithDescriptor:pass];
    [encoder setRenderPipelineState:nova_pipeline];
    [encoder setDepthStencilState:nova_depth_state];
    [encoder setVertexBuffer:nova_metal_vertex_buffer() offset:0 atIndex:0];
    [encoder setVertexBytes:nova_mvp length:sizeof(nova_mvp) atIndex:1];
    [encoder setFragmentTexture:nova_texture atIndex:0];
    [encoder setFragmentBytes:nova_light length:sizeof(nova_light) atIndex:0];
    [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:nova_metal_vertex_count()];
    [encoder endEncoding];
    [buffer presentDrawable:drawable];
    [buffer commit];
}

- (void)mtkView:(MTKView *)view drawableSizeWillChange:(CGSize)size {
    (void)view;
    (void)size;
}

@end

static NovaMetalRenderer *nova_renderer = nil;

@interface NovaMetalDelegate : NSObject <NSWindowDelegate, NSApplicationDelegate>
@end

@implementation NovaMetalDelegate
- (BOOL)windowShouldClose:(NSWindow *)sender {
    (void)sender;
    nova_running = 0;
    return NO;
}
- (BOOL)applicationShouldSaveApplicationState:(NSApplication *)sender {
    (void)sender;
    return NO;
}
- (BOOL)applicationShouldRestoreApplicationState:(NSApplication *)sender {
    (void)sender;
    return NO;
}
- (BOOL)applicationSupportsSecureRestorableState:(NSApplication *)app {
    (void)app;
    return YES;
}
@end

static NovaMetalDelegate *nova_delegate = nil;

static void nova_metal_build_menu(void) {
    NSMenu *bar = [[NSMenu alloc] init];
    NSMenuItem *app_item = [[NSMenuItem alloc] init];
    [bar addItem:app_item];
    NSMenu *app = [[NSMenu alloc] initWithTitle:@"Nova"];
    [app addItemWithTitle:@"Quit Nova Metal Lab"
                  action:@selector(terminate:)
           keyEquivalent:@"q"];
    app_item.submenu = app;
    [NSApp setMainMenu:bar];
}

static int nova_metal_build_pipeline(void) {
    NSError *error = nil;
    id<MTLLibrary> library = [nova_device newLibraryWithSource:[NSString stringWithUTF8String:nova_metal_shader]
                                                       options:nil
                                                         error:&error];
    if (!library) {
        nova_metal_fail(error.localizedDescription ?: @"shader compilation failed");
        return 0;
    }
    id<MTLFunction> vertex = [library newFunctionWithName:@"nova_vertex"];
    id<MTLFunction> fragment = [library newFunctionWithName:@"nova_fragment"];
    if (!vertex || !fragment) {
        nova_metal_fail(@"shader entry points are missing");
        return 0;
    }
    MTLRenderPipelineDescriptor *descriptor = [[MTLRenderPipelineDescriptor alloc] init];
    descriptor.vertexFunction = vertex;
    descriptor.fragmentFunction = fragment;
    descriptor.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
    descriptor.depthAttachmentPixelFormat = MTLPixelFormatDepth32Float;
    nova_pipeline = [nova_device newRenderPipelineStateWithDescriptor:descriptor error:&error];
    if (!nova_pipeline) {
        nova_metal_fail(error.localizedDescription ?: @"pipeline creation failed");
        return 0;
    }

    MTLDepthStencilDescriptor *depth = [[MTLDepthStencilDescriptor alloc] init];
    depth.depthCompareFunction = MTLCompareFunctionLess;
    depth.depthWriteEnabled = YES;
    nova_depth_state = [nova_device newDepthStencilStateWithDescriptor:depth];
    if (!nova_depth_state) {
        nova_metal_fail(@"could not create the depth state");
        return 0;
    }
    return 1;
}

/* Caller must ensure width and height are positive. */
static void nova_metal_render_pass(MTLRenderPassDescriptor *pass, int draw_triangle) {
    id<MTLCommandBuffer> buffer = [nova_queue commandBuffer];
    id<MTLRenderCommandEncoder> encoder = [buffer renderCommandEncoderWithDescriptor:pass];
    if (draw_triangle) {
        [encoder setRenderPipelineState:nova_pipeline];
        [encoder setDepthStencilState:nova_depth_state];
        [encoder setVertexBuffer:nova_metal_vertex_buffer() offset:0 atIndex:0];
        [encoder setVertexBytes:nova_mvp length:sizeof(nova_mvp) atIndex:1];
        [encoder setFragmentTexture:nova_texture atIndex:0];
        [encoder setFragmentBytes:nova_light length:sizeof(nova_light) atIndex:0];
        [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:nova_metal_vertex_count()];
    }
    [encoder endEncoding];
    [buffer commit];
    [buffer waitUntilCompleted];
}

/* Decodes one PNG with ImageIO into a BGRA8 texture. A CGBitmapContext keeps
 * the image's first row at scanline 0, which is Metal's texture row 0, so the
 * upload needs no extra flip: UV (0,0) is the image's top-left. Returns 1 on
 * success. */
static int nova_metal_load_texture(NSURL *url) {
    CGImageSourceRef source = CGImageSourceCreateWithURL((__bridge CFURLRef)url, NULL);
    if (!source) {
        nova_metal_fail(@"texture: could not open the PNG path");
        return 0;
    }
    CGImageRef image = CGImageSourceCreateImageAtIndex(source, 0, NULL);
    CFRelease(source);
    if (!image) {
        nova_metal_fail(@"texture: could not decode the PNG");
        return 0;
    }
    size_t width = CGImageGetWidth(image);
    size_t height = CGImageGetHeight(image);
    if (width == 0 || height == 0 || width > 8192 || height > 8192) {
        CGImageRelease(image);
        nova_metal_fail(@"texture: PNG size is unsupported");
        return 0;
    }

    size_t bytes_per_row = width * 4;
    void *pixels = calloc(bytes_per_row, height);
    CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    CGContextRef context = (pixels && space)
        ? CGBitmapContextCreate(pixels, width, height, 8, bytes_per_row, space,
                                kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little)
        : NULL;
    if (!context) {
        if (space) CGColorSpaceRelease(space);
        free(pixels);
        CGImageRelease(image);
        nova_metal_fail(@"could not allocate the texture bitmap");
        return 0;
    }
    CGContextSetBlendMode(context, kCGBlendModeCopy);
    CGContextDrawImage(context, CGRectMake(0, 0, (CGFloat)width, (CGFloat)height), image);
    CGContextRelease(context);
    CGColorSpaceRelease(space);
    CGImageRelease(image);

    MTLTextureDescriptor *descriptor =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                           width:width
                                                          height:height
                                                       mipmapped:NO];
    descriptor.usage = MTLTextureUsageShaderRead;
    descriptor.storageMode = MTLStorageModeShared;
    id<MTLTexture> texture = [nova_device newTextureWithDescriptor:descriptor];
    if (!texture) {
        free(pixels);
        nova_metal_fail(@"could not create the texture");
        return 0;
    }
    [texture replaceRegion:MTLRegionMake2D(0, 0, width, height)
               mipmapLevel:0
                 withBytes:pixels
               bytesPerRow:bytes_per_row];
    free(pixels);
    nova_texture = texture;
    return 1;
}

/* Loads a PNG into the single texture slot. The ABI stays an Int handle, a
 * path and a 0/1 result; no Metal object or pointer crosses the boundary. */
int64_t nova_metal_texture(int64_t handle, nova_str path) {
    if (handle != NOVA_METAL_SESSION || !nova_ready || !nova_device) return 0;
    if (!path.ptr || path.len <= 0) {
        nova_metal_fail(@"texture needs a file path");
        return 0;
    }
    @autoreleasepool {
        NSString *file_path = [[NSString alloc] initWithBytes:path.ptr
                                                       length:(NSUInteger)path.len
                                                     encoding:NSUTF8StringEncoding];
        if (!file_path) {
            nova_metal_fail(@"texture path is not valid UTF-8");
            return 0;
        }
        return nova_metal_load_texture([NSURL fileURLWithPath:file_path]);
    }
}

/* Selects the quad (0), colour cube (1) or textured lit cube (2). All three
 * buffers exist from init, so the switch is one Int and cannot fail after
 * validating the handle. */
int64_t nova_metal_geometry(int64_t handle, int64_t kind) {
    if (handle != NOVA_METAL_SESSION || !nova_ready) return 0;
    if (kind == 0) {
        nova_geometry = 0;
        return 1;
    }
    if (kind == 1) {
        nova_geometry = 1;
        return 1;
    }
    if (kind == 2) {
        nova_geometry = 2;
        return 1;
    }
    nova_metal_fail(@"geometry must be 0 (quad), 1 (cube) or 2 (lit cube)");
    return 0;
}

/* Enables or disables the directional light and sets its direction toward the
 * light. x, y and z are Q16.16; the shader normalizes them. Lighting defaults
 * off, so the unlit quad and colour cube keep their previous appearance. */
int64_t nova_metal_light(int64_t handle, int64_t enabled, int64_t x, int64_t y, int64_t z) {
    if (handle != NOVA_METAL_SESSION || !nova_ready) return 0;
    if (enabled && x == 0 && y == 0 && z == 0) {
        nova_metal_fail(@"light direction must not be zero");
        return 0;
    }
    nova_light[0] = (float)((double)x / 65536.0);
    nova_light[1] = (float)((double)y / 65536.0);
    nova_light[2] = (float)((double)z / 65536.0);
    nova_light[3] = enabled ? 1.0f : 0.0f;
    return 1;
}

/* Accepts the 16 row-major Q16.16 cells of model-view-projection. They are
 * transposed into Metal's column-major float4x4, so the shader's `mvp * p`
 * matches math.nova's row-major matrix product. */
int64_t nova_metal_mvp(int64_t handle,
                       int64_t m00, int64_t m01, int64_t m02, int64_t m03,
                       int64_t m10, int64_t m11, int64_t m12, int64_t m13,
                       int64_t m20, int64_t m21, int64_t m22, int64_t m23,
                       int64_t m30, int64_t m31, int64_t m32, int64_t m33) {
    if (handle != NOVA_METAL_SESSION || !nova_ready) return 0;
    int64_t cells[16] = {
        m00, m01, m02, m03,
        m10, m11, m12, m13,
        m20, m21, m22, m23,
        m30, m31, m32, m33,
    };
    int row = 0;
    while (row < 4) {
        int column = 0;
        while (column < 4) {
            nova_mvp[column * 4 + row] = (float)((double)cells[row * 4 + column] / 65536.0);
            column++;
        }
        row++;
    }
    return 1;
}

int64_t nova_metal_init(nova_str title, int64_t width, int64_t height) {
    @autoreleasepool {
        if (nova_ready) return NOVA_METAL_SESSION;
        if (width < 64) width = 64;
        if (height < 64) height = 64;
        nova_metal_error[0] = '\0';
        nova_geometry = 0;
        nova_input_orbit_dx = 0;
        nova_input_orbit_dy = 0;
        nova_input_pan_dx = 0;
        nova_input_pan_dy = 0;
        nova_input_scroll = 0;
        nova_light[0] = 0.45f;
        nova_light[1] = 0.85f;
        nova_light[2] = 0.55f;
        nova_light[3] = 0.0f;
        int axis = 0;
        while (axis < 16) {
            nova_mvp[axis] = (axis % 5 == 0) ? 1.0f : 0.0f;
            axis++;
        }

        nova_device = MTLCreateSystemDefaultDevice();
        if (!nova_device) {
            nova_metal_fail(@"no Metal device is available");
            return 0;
        }
        nova_queue = [nova_device newCommandQueue];
        if (!nova_queue) {
            nova_metal_fail(@"could not create a command queue");
            return 0;
        }

        NSApplication *app = [NSApplication sharedApplication];
        [app setActivationPolicy:NSApplicationActivationPolicyRegular];
        [app finishLaunching];
        nova_metal_build_menu();

        NSString *window_title = title.ptr && title.len > 0
            ? [[NSString alloc] initWithBytes:title.ptr length:(NSUInteger)title.len encoding:NSUTF8StringEncoding]
            : @"Nova Metal Lab";
        NSRect frame = NSMakeRect(0, 0, (CGFloat)width, (CGFloat)height);
        NSWindowStyleMask style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                                  NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable;
        nova_window = [[NSWindow alloc] initWithContentRect:frame
                                                  styleMask:style
                                                    backing:NSBackingStoreBuffered
                                                      defer:NO];
        if (!nova_window) {
            nova_metal_fail(@"could not create a window");
            return 0;
        }
        nova_window.title = window_title ? window_title : @"Nova Metal Lab";
        [nova_window setRestorable:NO];
        nova_window.backgroundColor = [NSColor colorWithCalibratedRed:0.05 green:0.06 blue:0.08 alpha:1.0];

        nova_view = [[NovaMetalView alloc] initWithFrame:frame device:nova_device];
        nova_view.colorPixelFormat = MTLPixelFormatBGRA8Unorm;
        nova_view.depthStencilPixelFormat = MTLPixelFormatDepth32Float;
        nova_view.clearColor = MTLClearColorMake(0.05, 0.06, 0.08, 1.0);
        nova_view.clearDepth = 1.0;
        nova_view.autoResizeDrawable = YES;
        nova_view.paused = YES;
        nova_view.enableSetNeedsDisplay = NO;
        nova_view.framebufferOnly = YES;
        nova_renderer = [[NovaMetalRenderer alloc] init];
        nova_view.delegate = nova_renderer;
        nova_window.contentView = nova_view;
        nova_delegate = [[NovaMetalDelegate alloc] init];
        nova_window.delegate = nova_delegate;
        [app setDelegate:nova_delegate];

        if (!nova_metal_build_pipeline()) {
            return 0;
        }

        /* Two triangles, one quad. The top-left vertex carries UV (0,0) so
         * the PNG is upright instead of mirrored or rotated. */
        NovaMetalVertex vertices[NOVA_METAL_QUAD_VERTICES] = {
            { -0.62f, 0.62f, 0.0f, 0.0f, 0.0f, 1.0f, 1.0f, 1.0f, 1.0f },
            { -0.62f, -0.62f, 0.0f, 0.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f },
            { 0.62f, -0.62f, 0.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f },
            { -0.62f, 0.62f, 0.0f, 0.0f, 0.0f, 1.0f, 1.0f, 1.0f, 1.0f },
            { 0.62f, -0.62f, 0.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f },
            { 0.62f, 0.62f, 0.0f, 1.0f, 0.0f, 1.0f, 1.0f, 1.0f, 1.0f },
        };
        nova_vertices = [nova_device newBufferWithBytes:vertices
                                                 length:sizeof(vertices)
                                                options:MTLResourceStorageModeShared];
        if (!nova_vertices) {
            nova_metal_fail(@"could not create the vertex buffer");
            return 0;
        }

        /* Unit cube at the origin, six faces with distinct colours. The -Z
         * magenta face is last on purpose: if depth testing is broken it
         * paints over the front face, which the capture test can detect. */
        NovaMetalVertex cube[NOVA_METAL_CUBE_VERTICES] = {
            /* +X orange */
            { 0.65f, -0.65f, -0.65f, 0.0f, 1.0f, 0.96f, 0.36f, 0.22f, 1.0f },
            { 0.65f, -0.65f, 0.65f, 1.0f, 1.0f, 0.96f, 0.36f, 0.22f, 1.0f },
            { 0.65f, 0.65f, 0.65f, 1.0f, 0.0f, 0.96f, 0.36f, 0.22f, 1.0f },
            { 0.65f, -0.65f, -0.65f, 0.0f, 1.0f, 0.96f, 0.36f, 0.22f, 1.0f },
            { 0.65f, 0.65f, 0.65f, 1.0f, 0.0f, 0.96f, 0.36f, 0.22f, 1.0f },
            { 0.65f, 0.65f, -0.65f, 0.0f, 0.0f, 0.96f, 0.36f, 0.22f, 1.0f },
            /* -X olive */
            { -0.65f, -0.65f, 0.65f, 0.0f, 1.0f, 0.45f, 0.72f, 0.25f, 1.0f },
            { -0.65f, -0.65f, -0.65f, 1.0f, 1.0f, 0.45f, 0.72f, 0.25f, 1.0f },
            { -0.65f, 0.65f, -0.65f, 1.0f, 0.0f, 0.45f, 0.72f, 0.25f, 1.0f },
            { -0.65f, -0.65f, 0.65f, 0.0f, 1.0f, 0.45f, 0.72f, 0.25f, 1.0f },
            { -0.65f, 0.65f, -0.65f, 1.0f, 0.0f, 0.45f, 0.72f, 0.25f, 1.0f },
            { -0.65f, 0.65f, 0.65f, 0.0f, 0.0f, 0.45f, 0.72f, 0.25f, 1.0f },
            /* +Y green */
            { -0.65f, 0.65f, -0.65f, 0.0f, 1.0f, 0.25f, 0.82f, 0.38f, 1.0f },
            { 0.65f, 0.65f, -0.65f, 1.0f, 1.0f, 0.25f, 0.82f, 0.38f, 1.0f },
            { 0.65f, 0.65f, 0.65f, 1.0f, 0.0f, 0.25f, 0.82f, 0.38f, 1.0f },
            { -0.65f, 0.65f, -0.65f, 0.0f, 1.0f, 0.25f, 0.82f, 0.38f, 1.0f },
            { 0.65f, 0.65f, 0.65f, 1.0f, 0.0f, 0.25f, 0.82f, 0.38f, 1.0f },
            { -0.65f, 0.65f, 0.65f, 0.0f, 0.0f, 0.25f, 0.82f, 0.38f, 1.0f },
            /* -Y teal */
            { -0.65f, -0.65f, 0.65f, 0.0f, 1.0f, 0.18f, 0.65f, 0.72f, 1.0f },
            { 0.65f, -0.65f, 0.65f, 1.0f, 1.0f, 0.18f, 0.65f, 0.72f, 1.0f },
            { 0.65f, -0.65f, -0.65f, 1.0f, 0.0f, 0.18f, 0.65f, 0.72f, 1.0f },
            { -0.65f, -0.65f, 0.65f, 0.0f, 1.0f, 0.18f, 0.65f, 0.72f, 1.0f },
            { 0.65f, -0.65f, -0.65f, 1.0f, 0.0f, 0.18f, 0.65f, 0.72f, 1.0f },
            { -0.65f, -0.65f, -0.65f, 0.0f, 0.0f, 0.18f, 0.65f, 0.72f, 1.0f },
            /* +Z blue */
            { -0.65f, -0.65f, 0.65f, 0.0f, 1.0f, 0.24f, 0.42f, 0.95f, 1.0f },
            { 0.65f, -0.65f, 0.65f, 1.0f, 1.0f, 0.24f, 0.42f, 0.95f, 1.0f },
            { 0.65f, 0.65f, 0.65f, 1.0f, 0.0f, 0.24f, 0.42f, 0.95f, 1.0f },
            { -0.65f, -0.65f, 0.65f, 0.0f, 1.0f, 0.24f, 0.42f, 0.95f, 1.0f },
            { 0.65f, 0.65f, 0.65f, 1.0f, 0.0f, 0.24f, 0.42f, 0.95f, 1.0f },
            { -0.65f, 0.65f, 0.65f, 0.0f, 0.0f, 0.24f, 0.42f, 0.95f, 1.0f },
            /* -Z magenta */
            { 0.65f, -0.65f, -0.65f, 0.0f, 1.0f, 0.82f, 0.24f, 0.86f, 1.0f },
            { -0.65f, -0.65f, -0.65f, 1.0f, 1.0f, 0.82f, 0.24f, 0.86f, 1.0f },
            { -0.65f, 0.65f, -0.65f, 1.0f, 0.0f, 0.82f, 0.24f, 0.86f, 1.0f },
            { 0.65f, -0.65f, -0.65f, 0.0f, 1.0f, 0.82f, 0.24f, 0.86f, 1.0f },
            { -0.65f, 0.65f, -0.65f, 1.0f, 0.0f, 0.82f, 0.24f, 0.86f, 1.0f },
            { 0.65f, 0.65f, -0.65f, 0.0f, 0.0f, 0.82f, 0.24f, 0.86f, 1.0f },
        };
        nova_cube_vertices = [nova_device newBufferWithBytes:cube
                                                      length:sizeof(cube)
                                                     options:MTLResourceStorageModeShared];
        if (!nova_cube_vertices) {
            nova_metal_fail(@"could not create the cube vertex buffer");
            return 0;
        }

        /* The lit cube shares the colour cube's positions and UVs, but uses a
         * white tint so the checker stays readable and adds one normal per
         * face. It is a separate buffer, so toggling lighting never changes
         * the ENG-02.004 cube acceptance. */
        NovaMetalVertex lit[NOVA_METAL_LIT_VERTICES];
        int lit_index = 0;
        while (lit_index < NOVA_METAL_LIT_VERTICES) {
            lit[lit_index] = cube[lit_index];
            lit[lit_index].r = 1.0f;
            lit[lit_index].g = 1.0f;
            lit[lit_index].b = 1.0f;
            lit[lit_index].a = 1.0f;
            int face = lit_index / 6;
            if (face == 0) {
                lit[lit_index].nx = 1.0f;
                lit[lit_index].ny = 0.0f;
                lit[lit_index].nz = 0.0f;
            }
            if (face == 1) {
                lit[lit_index].nx = -1.0f;
                lit[lit_index].ny = 0.0f;
                lit[lit_index].nz = 0.0f;
            }
            if (face == 2) {
                lit[lit_index].nx = 0.0f;
                lit[lit_index].ny = 1.0f;
                lit[lit_index].nz = 0.0f;
            }
            if (face == 3) {
                lit[lit_index].nx = 0.0f;
                lit[lit_index].ny = -1.0f;
                lit[lit_index].nz = 0.0f;
            }
            if (face == 4) {
                lit[lit_index].nx = 0.0f;
                lit[lit_index].ny = 0.0f;
                lit[lit_index].nz = 1.0f;
            }
            if (face == 5) {
                lit[lit_index].nx = 0.0f;
                lit[lit_index].ny = 0.0f;
                lit[lit_index].nz = -1.0f;
            }
            lit_index++;
        }
        nova_lit_vertices = [nova_device newBufferWithBytes:lit
                                                     length:sizeof(lit)
                                                    options:MTLResourceStorageModeShared];
        if (!nova_lit_vertices) {
            nova_metal_fail(@"could not create the lit cube vertex buffer");
            return 0;
        }

        /* A white 1x1 fallback keeps the sampler valid before a PNG is loaded,
         * so drawing never depends on texture load order. */
        MTLTextureDescriptor *white_descriptor =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                               width:1
                                                              height:1
                                                           mipmapped:NO];
        white_descriptor.usage = MTLTextureUsageShaderRead;
        white_descriptor.storageMode = MTLStorageModeShared;
        nova_texture = [nova_device newTextureWithDescriptor:white_descriptor];
        if (!nova_texture) {
            nova_metal_fail(@"could not create the fallback texture");
            return 0;
        }
        uint32_t white = 0xFFFFFFFFu;
        [nova_texture replaceRegion:MTLRegionMake2D(0, 0, 1, 1)
                        mipmapLevel:0
                          withBytes:&white
                        bytesPerRow:4];

        [nova_window center];
        [nova_window makeKeyAndOrderFront:nil];
        [nova_window makeFirstResponder:nova_view];
        [app activateIgnoringOtherApps:YES];
        nova_running = 1;
        nova_ready = 1;
        return NOVA_METAL_SESSION;
    }
}

/* Drains AppKit events without blocking and reports whether the window is
 * still open. Closing the window stops the Nova loop instead of calling back
 * into it. */
int64_t nova_metal_poll(int64_t handle) {
    if (handle != NOVA_METAL_SESSION || !nova_ready) return 0;
    @autoreleasepool {
        NSApplication *app = [NSApplication sharedApplication];
        while (1) {
            NSEvent *event = [app nextEventMatchingMask:NSEventMaskAny
                                              untilDate:[NSDate distantPast]
                                                 inMode:NSDefaultRunLoopMode
                                                dequeue:YES];
            if (!event) break;
            [app sendEvent:event];
        }
    }
    return nova_running ? 1 : 0;
}

/* Polled camera input. The caller reads the five slots, applies them to its
 * camera and then calls reset. Deltas accumulate between polls, so a fast drag
 * is not lost if the render loop is busy. */
int64_t nova_metal_input(int64_t handle, int64_t slot) {
    if (handle != NOVA_METAL_SESSION || !nova_ready) return 0;
    if (slot == 0) return nova_input_orbit_dx;
    if (slot == 1) return nova_input_orbit_dy;
    if (slot == 2) return nova_input_pan_dx;
    if (slot == 3) return nova_input_pan_dy;
    if (slot == 4) return nova_input_scroll;
    return 0;
}

void nova_metal_input_reset(int64_t handle) {
    if (handle != NOVA_METAL_SESSION) return;
    nova_input_orbit_dx = 0;
    nova_input_orbit_dy = 0;
    nova_input_pan_dx = 0;
    nova_input_pan_dy = 0;
    nova_input_scroll = 0;
}

/* Returns 1 when a frame was submitted, 0 when the drawable is not ready yet
 * (window still ordering, or resized to zero). The caller retries. */
int64_t nova_metal_draw(int64_t handle, int64_t red, int64_t green, int64_t blue) {
    if (handle != NOVA_METAL_SESSION || !nova_ready || !nova_view) return 0;
    @autoreleasepool {
        nova_view.clearColor = MTLClearColorMake(
            (double)(red < 0 ? 0 : (red > 255 ? 255 : red)) / 255.0,
            (double)(green < 0 ? 0 : (green > 255 ? 255 : green)) / 255.0,
            (double)(blue < 0 ? 0 : (blue > 255 ? 255 : blue)) / 255.0,
            1.0);
        [nova_view draw];
    }
    return 1;
}

/* Resizes the window and its drawable explicitly. MTKView would catch up on
 * the next layout pass, but a capture taken immediately after a resize needs
 * the new size now. Returns 1 when the new size was applied. */
int64_t nova_metal_resize(int64_t handle, int64_t width, int64_t height) {
    if (handle != NOVA_METAL_SESSION || !nova_ready || !nova_window || !nova_view) return 0;
    if (width < 64) width = 64;
    if (height < 64) height = 64;
    @autoreleasepool {
        [nova_window setContentSize:NSMakeSize((CGFloat)width, (CGFloat)height)];
        [nova_view setFrame:NSMakeRect(0, 0, (CGFloat)width, (CGFloat)height)];
        CGFloat scale = nova_window.backingScaleFactor > 0 ? nova_window.backingScaleFactor : 1.0;
        nova_view.drawableSize = NSMakeSize((CGFloat)width * scale, (CGFloat)height * scale);
        [nova_window displayIfNeeded];
    }
    return 1;
}

/* Renders the same textured quad into an offscreen texture and writes it as
 * PNG.
 * This is the stable frame a test can diff; it does not depend on the window
 * server compositing the drawable. */
int64_t nova_metal_capture(int64_t handle, nova_str path) {
    if (handle != NOVA_METAL_SESSION || !nova_ready || !nova_view) return 0;
    if (!path.ptr || path.len <= 0) {
        nova_metal_fail(@"capture needs a file path");
        return 0;
    }
    @autoreleasepool {
        NSUInteger width = (NSUInteger)(nova_view.drawableSize.width > 0 ? nova_view.drawableSize.width : nova_view.bounds.size.width);
        NSUInteger height = (NSUInteger)(nova_view.drawableSize.height > 0 ? nova_view.drawableSize.height : nova_view.bounds.size.height);
        if (width == 0 || height == 0) {
            nova_metal_fail(@"capture has no drawable size");
            return 0;
        }

        MTLTextureDescriptor *descriptor =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                               width:width
                                                              height:height
                                                           mipmapped:NO];
        descriptor.usage = MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead;
        descriptor.storageMode = MTLStorageModePrivate;
        id<MTLTexture> texture = [nova_device newTextureWithDescriptor:descriptor];
        if (!texture) {
            nova_metal_fail(@"could not create the capture texture");
            return 0;
        }

        MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture = texture;
        pass.colorAttachments[0].loadAction = MTLLoadActionClear;
        pass.colorAttachments[0].storeAction = MTLStoreActionStore;
        pass.colorAttachments[0].clearColor = MTLClearColorMake(0.05, 0.06, 0.08, 1.0);

        MTLTextureDescriptor *depth_descriptor =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatDepth32Float
                                                               width:width
                                                              height:height
                                                           mipmapped:NO];
        depth_descriptor.usage = MTLTextureUsageRenderTarget;
        depth_descriptor.storageMode = MTLStorageModePrivate;
        id<MTLTexture> depth = [nova_device newTextureWithDescriptor:depth_descriptor];
        if (!depth) {
            nova_metal_fail(@"could not create the capture depth texture");
            return 0;
        }
        pass.depthAttachment.texture = depth;
        pass.depthAttachment.loadAction = MTLLoadActionClear;
        pass.depthAttachment.storeAction = MTLStoreActionDontCare;
        pass.depthAttachment.clearDepth = 1.0;
        nova_metal_render_pass(pass, 1);

        NSUInteger bytes_per_row = ((width * 4 + 255) / 256) * 256;
        id<MTLBuffer> readback = [nova_device newBufferWithLength:bytes_per_row * height
                                                        options:MTLResourceStorageModeShared];
        if (!readback) {
            nova_metal_fail(@"could not create the capture buffer");
            return 0;
        }
        id<MTLCommandBuffer> buffer = [nova_queue commandBuffer];
        id<MTLBlitCommandEncoder> blit = [buffer blitCommandEncoder];
        [blit copyFromTexture:texture
                  sourceSlice:0
                  sourceLevel:0
                 sourceOrigin:MTLOriginMake(0, 0, 0)
                   sourceSize:MTLSizeMake(width, height, 1)
                     toBuffer:readback
            destinationOffset:0
       destinationBytesPerRow:bytes_per_row
     destinationBytesPerImage:bytes_per_row * height];
        [blit endEncoding];
        [buffer commit];
        [buffer waitUntilCompleted];

        CGColorSpaceRef space = CGColorSpaceCreateDeviceRGB();
        CGDataProviderRef provider = CGDataProviderCreateWithData(NULL, readback.contents,
                                                                  bytes_per_row * height, NULL);
        CGImageRef image = CGImageCreate(width, height, 8, 32, bytes_per_row, space,
                                         kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little,
                                         provider, NULL, false, kCGRenderingIntentDefault);
        if (!image) {
            CGDataProviderRelease(provider);
            CGColorSpaceRelease(space);
            nova_metal_fail(@"could not create the capture image");
            return 0;
        }
        NSString *file_path = [[NSString alloc] initWithBytes:path.ptr
                                                       length:(NSUInteger)path.len
                                                     encoding:NSUTF8StringEncoding];
        NSURL *url = file_path ? [NSURL fileURLWithPath:file_path] : nil;
        CGImageDestinationRef destination = url
            ? CGImageDestinationCreateWithURL((__bridge CFURLRef)url, CFSTR("public.png"), 1, NULL)
            : NULL;
        if (!destination) {
            CGImageRelease(image);
            CGDataProviderRelease(provider);
            CGColorSpaceRelease(space);
            nova_metal_fail(@"could not open the capture path");
            return 0;
        }
        CGImageDestinationAddImage(destination, image, NULL);
        bool ok = CGImageDestinationFinalize(destination);
        CFRelease(destination);
        CGImageRelease(image);
        CGDataProviderRelease(provider);
        CGColorSpaceRelease(space);
        if (!ok) {
            nova_metal_fail(@"could not write the capture PNG");
            return 0;
        }
    }
    return 1;
}

nova_str nova_metal_env(nova_str name) {
    if (!name.ptr || name.len <= 0) return nova_metal_copy("");
    char key[256];
    int64_t length = name.len < 255 ? name.len : 255;
    memcpy(key, name.ptr, (size_t)length);
    key[length] = '\0';
    const char *value = getenv(key);
    return nova_metal_copy(value ? value : "");
}

nova_str nova_metal_last_error(void) {
    return nova_metal_copy(nova_metal_error);
}

void nova_metal_shutdown(int64_t handle) {
    if (handle != NOVA_METAL_SESSION) return;
    nova_running = 0;
    nova_ready = 0;
    @autoreleasepool {
        nova_view.delegate = nil;
        nova_window.delegate = nil;
        /* Closing a programmatically created window can release it outside
         * ARC, which would over-release the strong reference below. */
        nova_window.releasedWhenClosed = NO;
        [nova_window close];
        nova_window = nil;
        nova_renderer = nil;
        nova_view = nil;
        nova_texture = nil;
        nova_lit_vertices = nil;
        nova_cube_vertices = nil;
        nova_vertices = nil;
        nova_depth_state = nil;
        nova_pipeline = nil;
        nova_queue = nil;
        nova_device = nil;
    }
    nova_metal_error[0] = '\0';
}
