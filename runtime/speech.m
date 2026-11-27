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

#include <signal.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>

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
 * default instead of whichever one AVFoundation happens to return first.
 *
 * The sorted list is cached: a panel reads every index once per frame, and
 * re-enumerating and re-sorting hundreds of voices per lookup made the reader
 * crawl. The installed set does not change while the app runs. */
static NSArray<AVSpeechSynthesisVoice *> *nova_speech_voices(void) {
    static NSArray<AVSpeechSynthesisVoice *> *cached = nil;
    if (cached) return cached;
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
    cached = sorted;
    return cached;
}

/* Count of installed English voices without materializing the list, so panel
 * geometry stays cheap even on machines with hundreds of voices installed. */
int64_t nova_speech_english_count(void) {
    int64_t count = 0;
    for (AVSpeechSynthesisVoice *voice in nova_speech_voices()) {
        if ([voice.language hasPrefix:@"en"]) count++;
    }
    return count;
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

/* The model can live in three places, in order: an explicit override, the
 * browser profile the app downloads into, or the app bundle (a fat build).
 * Presence is still not synthesis: the neural runtime is a separate build, and
 * `backend` keeps reporting `system` until one is linked in. */
static NSString *nova_kokoro_model(void) {
    NSString *override = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_KOKORO_MODEL"];
    if (override.length > 0) {
        if ([[NSFileManager defaultManager] fileExistsAtPath:override]) return override;
        return @"";
    }
    NSString *profile = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_BROWSER_PROFILE"];
    if (profile.length == 0) {
        profile = [NSHomeDirectory() stringByAppendingPathComponent:@"Library/Application Support/Nova Browser"];
    }
    NSString *downloaded = [profile stringByAppendingPathComponent:@"kokoro/model.onnx"];
    if ([[NSFileManager defaultManager] fileExistsAtPath:downloaded]) return downloaded;
    NSString *path = [[NSBundle mainBundle] pathForResource:@"model" ofType:@"onnx" inDirectory:@"kokoro"];
    return path ?: @"";
}

/* Kokoro speaks through a separate synthesizer command, the way the model and
 * the runtime are separate things: `NOVA_KOKORO_CMD` when set, otherwise a
 * `kokoro-tts` or `kokoro-cli` on the usual install paths. The contract is
 * `<command> <text-file> <out.wav>`; the model path arrives in
 * `NOVA_KOKORO_MODEL` so a wrapper can pass it on. Synthesis is slow, so the
 * job runs as a child process and the caller polls its phase per frame. */
static NSString *nova_kokoro_runner(void) {
    NSString *override = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_KOKORO_CMD"];
    if (override.length > 0) return override;
    NSArray *names = @[@"kokoro-tts", @"kokoro-cli"];
    NSArray *dirs = @[@"/opt/homebrew/bin", @"/usr/local/bin",
                      [NSHomeDirectory() stringByAppendingPathComponent:@".local/bin"]];
    for (NSString *dir in dirs) {
        for (NSString *name in names) {
            NSString *path = [dir stringByAppendingPathComponent:name];
            if ([[NSFileManager defaultManager] isExecutableFileAtPath:path]) return path;
        }
    }
    return @"";
}

int64_t nova_speech_kokoro_usable(void) {
    return (nova_kokoro_model().length > 0 && nova_kokoro_runner().length > 0) ? 1 : 0;
}

nova_str nova_speech_kokoro_runner_name(void) {
    return nova_speech_copy(nova_kokoro_runner());
}

/* 0 idle, 1 synthesizing, 3 playing, 4 failed, 5 synthesized but muted
 * (NOVA_SPEECH_SILENT), which is what the headless tests observe. */
static NSTask *nova_job_task = nil;
static NSTask *nova_job_play = nil;
static int64_t nova_job_phase = 0;
static int64_t nova_job_started = 0;
static int64_t nova_job_paused = 0;
static int64_t nova_job_silent = 0;
static NSString *nova_job_wav = nil;
static NSString *nova_job_log = nil;
static NSString *nova_job_detail = nil;
static NSString *nova_engine = nil;

static int64_t nova_now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + (int64_t)(ts.tv_nsec / 1000000);
}

static void nova_job_signal(NSTask *task, int sig) {
    if (task && task.isRunning) kill(task.processIdentifier, sig);
}

static void nova_job_stop(void) {
    /* A stopped child ignores SIGTERM until it resumes, so continue first. */
    nova_job_signal(nova_job_task, SIGCONT);
    nova_job_signal(nova_job_task, SIGTERM);
    nova_job_signal(nova_job_play, SIGCONT);
    nova_job_signal(nova_job_play, SIGTERM);
    nova_job_task = nil;
    nova_job_play = nil;
    nova_job_phase = 0;
    nova_job_paused = 0;
    nova_job_started = 0;
    nova_job_detail = nil;
}

static NSString *nova_job_log_tail(void) {
    NSString *log = [NSString stringWithContentsOfFile:nova_job_log encoding:NSUTF8StringEncoding error:nil];
    if (log.length == 0) return @"";
    NSArray<NSString *> *lines = [log componentsSeparatedByString:@"\n"];
    for (NSInteger i = (NSInteger)lines.count - 1; i >= 0; i--) {
        NSString *line = [lines[(NSUInteger)i] stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceAndNewlineCharacterSet]];
        if (line.length > 0) return line.length > 160 ? [line substringToIndex:160] : line;
    }
    return @"";
}

static int64_t nova_job_start(NSString *body, NSString *voice) {
    NSString *model = nova_kokoro_model();
    NSString *runner = nova_kokoro_runner();
    if (model.length == 0) {
        nova_job_detail = @"the Kokoro model is not installed";
        return 0;
    }
    if (runner.length == 0) {
        nova_job_detail = @"no Kokoro synthesizer command";
        return 0;
    }
    nova_job_stop();
    NSString *dir = [model stringByDeletingLastPathComponent];
    NSString *txt = [dir stringByAppendingPathComponent:@"read.txt"];
    nova_job_wav = [dir stringByAppendingPathComponent:@"read.wav"];
    nova_job_log = [dir stringByAppendingPathComponent:@"say.log"];
    [[NSFileManager defaultManager] removeItemAtPath:nova_job_wav error:nil];
    if (![body writeToFile:txt atomically:YES encoding:NSUTF8StringEncoding error:nil]) {
        nova_job_detail = @"could not stage the page text";
        return 0;
    }
    NSString *launch = [[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_KOKORO_CMD"].length > 0
        ? runner
        : [NSString stringWithFormat:@"'%@'", runner];
    NSString *command = [NSString stringWithFormat:
        @"NOVA_KOKORO_MODEL='%@' NOVA_KOKORO_VOICE='%@' %@ '%@' '%@' >'%@' 2>&1",
        model, voice ?: @"", launch, txt, nova_job_wav, nova_job_log];
    NSTask *task = [[NSTask alloc] init];
    task.executableURL = [NSURL fileURLWithPath:@"/bin/sh"];
    task.arguments = @[@"-c", command];
    NSError *error = nil;
    if (![task launchAndReturnError:&error]) {
        nova_job_detail = @"could not start the Kokoro command";
        return 0;
    }
    nova_job_task = task;
    nova_job_phase = 1;
    nova_job_started = nova_now_ms();
    nova_job_paused = 0;
    nova_job_silent = [[[[NSProcessInfo processInfo] environment] objectForKey:@"NOVA_SPEECH_SILENT"] isEqualToString:@"1"] ? 1 : 0;
    nova_job_detail = nil;
    return 1;
}

/* Advances the job one frame. Nothing here guesses at progress: the loading
 * phase lasts until the wav appears, synthesizing until the child exits, and
 * playing until afplay does. */
static void nova_job_poll(void) {
    if (nova_job_phase == 5 && nova_job_started > 0 && nova_now_ms() - nova_job_started > 1500) {
        nova_job_phase = 0;
        nova_job_started = 0;
        return;
    }
    if (nova_job_phase == 4) return;
    if (nova_job_task) {
        if (nova_job_task.isRunning) {
            /* One synthesizing phase: a command that writes its wav at the end
             * spends almost all of its time here, and calling that "loading"
             * would misdescribe the slow part. */
            nova_job_phase = 1;
            return;
        }
        int status = nova_job_task.terminationStatus;
        nova_job_task = nil;
        struct stat st;
        BOOL ok = status == 0 && nova_job_wav.length > 0 &&
                  stat(nova_job_wav.UTF8String, &st) == 0 && st.st_size > 1024;
        if (!ok) {
            NSString *tail = nova_job_log_tail();
            nova_job_phase = 4;
            nova_job_detail = tail.length > 0 ? tail
                : [NSString stringWithFormat:@"synthesizer exited with %d", status];
            return;
        }
        if (nova_job_silent) {
            nova_job_phase = 5;
            return;
        }
        NSTask *play = [[NSTask alloc] init];
        play.executableURL = [NSURL fileURLWithPath:@"/usr/bin/afplay"];
        play.arguments = @[nova_job_wav];
        if ([play launchAndReturnError:nil]) {
            nova_job_play = play;
            nova_job_phase = 3;
            return;
        }
        nova_job_phase = 5;
        return;
    }
    if (nova_job_play) {
        if (nova_job_play.isRunning) {
            nova_job_phase = 3;
            return;
        }
        nova_job_play = nil;
        nova_job_phase = 0;
        nova_job_started = 0;
    }
}

int64_t nova_speech_synth_phase(void) {
    nova_job_poll();
    return nova_job_phase;
}

int64_t nova_speech_synth_elapsed_ms(void) {
    nova_job_poll();
    if (nova_job_started == 0 || nova_job_phase == 0) return 0;
    return nova_now_ms() - nova_job_started;
}

nova_str nova_speech_synth_detail(void) {
    nova_job_poll();
    if (nova_job_phase == 4) return nova_speech_copy(nova_job_detail ?: @"synthesis failed");
    if (nova_job_phase == 1) return nova_speech_copy(@"loading the model");
    if (nova_job_phase == 3) return nova_speech_copy(@"playing");
    if (nova_job_phase == 5) return nova_speech_copy(@"synthesized");
    return nova_speech_empty();
}

static BOOL nova_engine_kokoro(void) {
    if (nova_engine) return [nova_engine isEqualToString:@"kokoro"];
    NSDictionary *env = [[NSProcessInfo processInfo] environment];
    return [[env objectForKey:@"NOVA_BROWSER_ENGINE"] isEqualToString:@"kokoro"] ||
           [[env objectForKey:@"NOVA_BROWSER_VOICE"] isEqualToString:@"kokoro"] ||
           [[env objectForKey:@"NOVA_BROWSER_FAT"] isEqualToString:@"1"];
}

void nova_speech_set_engine(nova_str engine) {
    nova_engine = nova_speech_string(engine);
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
    if (nova_engine_kokoro() && nova_speech_kokoro_usable()) {
        if (nova_synth.isSpeaking) [nova_synth stopSpeakingAtBoundary:AVSpeechBoundaryImmediate];
        nova_speech_error = nil;
        return nova_job_start(body, nova_speech_string(voice_id));
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
    nova_job_stop();
    if (!nova_synth) return;
    if (nova_synth.isSpeaking || nova_synth.isPaused) {
        [nova_synth stopSpeakingAtBoundary:AVSpeechBoundaryImmediate];
    }
    nova_speech_pause_flag = 0;
    nova_utterance = nil;
}

void nova_speech_pause(void) {
    if (nova_job_task || nova_job_play) {
        nova_job_signal(nova_job_task, SIGSTOP);
        nova_job_signal(nova_job_play, SIGSTOP);
        nova_job_paused = 1;
        return;
    }
    if (!nova_synth || !nova_synth.isSpeaking) return;
    if ([nova_synth pauseSpeakingAtBoundary:AVSpeechBoundaryWord]) nova_speech_pause_flag = 1;
}

void nova_speech_resume(void) {
    if (nova_job_paused) {
        nova_job_signal(nova_job_task, SIGCONT);
        nova_job_signal(nova_job_play, SIGCONT);
        nova_job_paused = 0;
        return;
    }
    if (!nova_synth || !nova_synth.isPaused) return;
    if ([nova_synth continueSpeaking]) nova_speech_pause_flag = 0;
}

int64_t nova_speech_speaking(void) {
    nova_job_poll();
    if (nova_job_phase == 1 || nova_job_phase == 2 || nova_job_phase == 3) return 1;
    return (nova_synth && nova_synth.isSpeaking) ? 1 : 0;
}

int64_t nova_speech_paused(void) {
    if (nova_job_paused) return 1;
    return (nova_synth && nova_synth.isPaused) ? 1 : 0;
}

/* "kokoro" only while its job is actually producing audio; the system voice
 * before that and after it. A model on disk does not by itself change this. */
nova_str nova_speech_backend(void) {
    nova_job_poll();
    if (nova_job_phase == 1 || nova_job_phase == 2 || nova_job_phase == 3) {
        return nova_speech_copy(@"kokoro");
    }
    return nova_speech_copy(@"system");
}

int64_t nova_speech_kokoro_ready(void) {
    return nova_kokoro_model().length > 0 ? 1 : 0;
}

/* True when the engine in use is Kokoro, whether from the saved setting, the
 * environment, or a fat build. */
int64_t nova_speech_kokoro_wanted(void) {
    return nova_engine_kokoro() ? 1 : 0;
}

/* The reason a requested voice is not being used, or "". The app shows this
 * instead of quietly switching, so the difference between "slim build" and
 * "this page has no text" stays visible. */
nova_str nova_speech_note(void) {
    nova_job_poll();
    if (nova_speech_error.length > 0) return nova_speech_copy(nova_speech_error);
    if (nova_speech_kokoro_wanted()) {
        if (!nova_speech_kokoro_ready()) {
            return nova_speech_copy(@"the Kokoro model is not installed");
        }
        if (nova_kokoro_runner().length == 0) {
            return nova_speech_copy(@"Kokoro needs a synthesizer command (install kokoro-tts or set NOVA_KOKORO_CMD)");
        }
        if (nova_job_phase == 4) return nova_speech_copy(nova_job_detail ?: @"Kokoro synthesis failed");
    }
    return nova_speech_copy(@"");
}
