/*
 * nova-speech — a polled text-to-speech adapter for macOS.
 *
 * Nova has no C-to-Nova callbacks, so this file exposes speech as state that a
 * frame loop can poll: speak once, then read `nova_speech_speaking()` and
 * `nova_speech_paused()`. The synthesizer runs on the main thread; the page
 * text comes from the caller (the browser extracts it through WebKit).
 *
 * Voices are the system voices macOS already ships, including the downloadable
 * enhanced and premium ones. A neural voice (Kokoro) is a separate build: this
 * file only reports whether its model and runtime are present, so a slim build
 * never pretends to have a voice it cannot synthesize.
 */

#import <AVFoundation/AVFoundation.h>
#import <Foundation/Foundation.h>

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Matches the Nova runtime ABI: { const char *ptr; int64_t len; } UTF-8. */
typedef struct {
    const char *ptr;
    int64_t len;
} nova_str;

static AVSpeechSynthesizer *nova_synth = nil;
static AVSpeechUtterance *nova_utterance = nil;
static int64_t nova_speech_pause_flag = 0;
static NSString *nova_speech_error = nil;

static nova_str nova_speech_empty(void) {
    nova_str value = {NULL, 0};
    return value;
}

static nova_str nova_speech_copy(NSString *value) {
    if (!value) return nova_speech_empty();
    const char *utf8 = value.UTF8String;
    if (!utf8) return nova_speech_empty();
    size_t length = strlen(utf8);
    char *copy = (char *)malloc(length + 1);
    if (!copy) return nova_speech_empty();
    memcpy(copy, utf8, length + 1);
    nova_str result = {copy, (int64_t)length};
    return result;
}

static NSString *nova_speech_string(nova_str value) {
    if (!value.ptr || value.len <= 0) return @"";
    return [[NSString alloc] initWithBytes:value.ptr length:(NSUInteger)value.len encoding:NSUTF8StringEncoding] ?: @"";
}

/* Enhanced voices are downloaded on demand in System Settings, so the list is
 * ordered by quality and then alphabetically: the best installed voice is the
 * default instead of whichever one AVFoundation happens to return first. */
static NSArray<AVSpeechSynthesisVoice *> *nova_speech_voices(void) {
    NSArray *all = [AVSpeechSynthesisVoice speechVoices];
    NSMutableArray *sorted = [NSMutableArray arrayWithArray:all];
    [sorted sortUsingComparator:^NSComparisonResult(AVSpeechSynthesisVoice *a, AVSpeechSynthesisVoice *b) {
        if (a.quality != b.quality) {
            return a.quality > b.quality ? NSOrderedAscending : NSOrderedDescending;
        }
        BOOL aEnglish = [a.language hasPrefix:@"en"];
        BOOL bEnglish = [b.language hasPrefix:@"en"];
        if (aEnglish != bEnglish) return aEnglish ? NSOrderedAscending : NSOrderedDescending;
        NSComparisonResult byName = [a.name localizedCaseInsensitiveCompare:b.name];
        if (byName != NSOrderedSame) return byName;
        return [a.language compare:b.language];
    }];
    return sorted;
}

static AVSpeechSynthesisVoice *nova_speech_voice(NSString *identifier) {
    if (identifier.length == 0) return nil;
    AVSpeechSynthesisVoice *voice = [AVSpeechSynthesisVoice voiceWithIdentifier:identifier];
    if (voice) return voice;
    /* Accept a display name too: settings are hand-editable and a name is what
     * the UI shows. */
    for (AVSpeechSynthesisVoice *candidate in nova_speech_voices()) {
        if ([candidate.name caseInsensitiveCompare:identifier] == NSOrderedSame) return candidate;
    }
    return nil;
}

/* The bundle may carry a Kokoro model (a fat build). Without it the app falls
 * back to the system voice and says so, rather than silently changing voice. */
static NSString *nova_kokoro_model(void) {
    NSString *override = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_KOKORO_MODEL"];
    if (override.length > 0) {
        if ([[NSFileManager defaultManager] fileExistsAtPath:override]) return override;
        return @"";
    }
    NSString *path = [[NSBundle mainBundle] pathForResource:@"model" ofType:@"onnx" inDirectory:@"kokoro"];
    return path ?: @"";
}

int64_t nova_speech_available(void) {
    if (@available(macOS 10.14, *)) {
        return 1;
    }
    return 0;
}

int64_t nova_speech_voice_count(void) {
    return (int64_t)nova_speech_voices().count;
}

nova_str nova_speech_voice_id(int64_t index) {
    NSArray *voices = nova_speech_voices();
    if (index < 0 || index >= (int64_t)voices.count) return nova_speech_empty();
    return nova_speech_copy(((AVSpeechSynthesisVoice *)voices[(NSUInteger)index]).identifier);
}

nova_str nova_speech_voice_name(int64_t index) {
    NSArray *voices = nova_speech_voices();
    if (index < 0 || index >= (int64_t)voices.count) return nova_speech_empty();
    return nova_speech_copy(((AVSpeechSynthesisVoice *)voices[(NSUInteger)index]).name);
}

nova_str nova_speech_voice_language(int64_t index) {
    NSArray *voices = nova_speech_voices();
    if (index < 0 || index >= (int64_t)voices.count) return nova_speech_empty();
    return nova_speech_copy(((AVSpeechSynthesisVoice *)voices[(NSUInteger)index]).language);
}

/* 0 default, 1 enhanced, 2 premium. */
int64_t nova_speech_voice_quality(int64_t index) {
    NSArray *voices = nova_speech_voices();
    if (index < 0 || index >= (int64_t)voices.count) return 0;
    return (int64_t)((AVSpeechSynthesisVoice *)voices[(NSUInteger)index]).quality;
}

/* The best installed English voice, which is what a fresh profile should read
 * with instead of the compact default. */
nova_str nova_speech_default_voice(void) {
    for (AVSpeechSynthesisVoice *voice in nova_speech_voices()) {
        if ([voice.language hasPrefix:@"en"]) return nova_speech_copy(voice.identifier);
    }
    return nova_speech_copy(@"");
}

/* rate is a percentage of the system default (100 = normal, 0 = slowest). */
int64_t nova_speech_speak(nova_str text, nova_str voice_id, int64_t rate) {
    if (![NSThread isMainThread]) return 0;
    NSString *body = nova_speech_string(text);
    if (body.length == 0) {
        nova_speech_error = @"empty text";
        return 0;
    }
    if (!nova_synth) nova_synth = [[AVSpeechSynthesizer alloc] init];
    if (!nova_synth) return 0;
    if (nova_synth.isSpeaking) [nova_synth stopSpeakingAtBoundary:AVSpeechBoundaryImmediate];
    nova_speech_pause_flag = 0;

    AVSpeechUtterance *utterance = [AVSpeechUtterance speechUtteranceWithString:body];
    AVSpeechSynthesisVoice *voice = nova_speech_voice(nova_speech_string(voice_id));
    if (voice) utterance.voice = voice;
    if (rate < 10) rate = 10;
    if (rate > 200) rate = 200;
    utterance.rate = AVSpeechUtteranceDefaultSpeechRate * (float)rate / 100.0f;
    /* Tests cannot verify audio, so they mute the synthesizer and check the
     * state transitions instead of making the machine talk. */
    NSString *silent = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_SPEECH_SILENT"];
    utterance.volume = [silent isEqualToString:@"1"] ? 0.0f : 1.0f;

    nova_utterance = utterance;
    nova_speech_error = nil;
    [nova_synth speakUtterance:utterance];
    return 1;
}

void nova_speech_stop(void) {
    if (!nova_synth) return;
    if (nova_synth.isSpeaking || nova_synth.isPaused) {
        [nova_synth stopSpeakingAtBoundary:AVSpeechBoundaryImmediate];
    }
    nova_speech_pause_flag = 0;
    nova_utterance = nil;
}

void nova_speech_pause(void) {
    if (!nova_synth || !nova_synth.isSpeaking) return;
    if ([nova_synth pauseSpeakingAtBoundary:AVSpeechBoundaryWord]) nova_speech_pause_flag = 1;
}

void nova_speech_resume(void) {
    if (!nova_synth || !nova_synth.isPaused) return;
    if ([nova_synth continueSpeaking]) nova_speech_pause_flag = 0;
}

int64_t nova_speech_speaking(void) {
    return (nova_synth && nova_synth.isSpeaking) ? 1 : 0;
}

int64_t nova_speech_paused(void) {
    return (nova_synth && nova_synth.isPaused) ? 1 : 0;
}

/* "system" or "kokoro". Kokoro only counts when the model is actually in the
 * bundle; the neural runtime itself is not part of the slim build. */
nova_str nova_speech_backend(void) {
    NSString *requested = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_BROWSER_VOICE"];
    if ([requested isEqualToString:@"kokoro"] && nova_kokoro_model().length > 0) {
        return nova_speech_copy(@"kokoro");
    }
    return nova_speech_copy(@"system");
}

int64_t nova_speech_kokoro_ready(void) {
    return nova_kokoro_model().length > 0 ? 1 : 0;
}

/* True when the environment asks for the neural voice (fat build or an
 * explicit NOVA_BROWSER_VOICE=kokoro). */
int64_t nova_speech_kokoro_wanted(void) {
    NSString *requested = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_BROWSER_VOICE"];
    NSString *fat = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_BROWSER_FAT"];
    return ([requested isEqualToString:@"kokoro"] || [fat isEqualToString:@"1"]) ? 1 : 0;
}

/* The reason a requested voice is not being used, or "". The app shows this
 * instead of quietly switching, so the difference between "slim build" and
 * "this page has no text" stays visible. */
nova_str nova_speech_note(void) {
    if (nova_speech_error.length > 0) return nova_speech_copy(nova_speech_error);
    if (nova_speech_kokoro_wanted() && !nova_speech_kokoro_ready()) {
        return nova_speech_copy(@"Kokoro model is not in this build");
    }
    if (nova_speech_kokoro_wanted() && nova_speech_kokoro_ready()) {
        return nova_speech_copy(@"Kokoro model present, neural runtime not bundled");
    }
    return nova_speech_copy(@"");
}
