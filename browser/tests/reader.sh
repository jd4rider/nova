#!/bin/sh
# Reading aloud: page text reaches the speech host, the controls paint above the
# page, the Kokoro synthesizer reports real phases, and the voice-model install
# accepts a real file and rejects a stub.
#
# Audio is not verifiable headlessly, so the checks read the host's state
# (`speaking`, chars, rate, synth phases) with NOVA_SPEECH_SILENT=1 muting the
# output. The model and synthesizer checks use a local file and a stub command
# so they never touch the network.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "reader: missing $BIN (build the browser first)"
    exit 1
fi

pass=0
fail=0
check_in() {
    if grep -qF "$1" "$2"; then
        echo "PASS $3"
        pass=$((pass + 1))
    else
        echo "FAIL $3"
        fail=$((fail + 1))
    fi
}

echo "--- page reading ---"
PROFILE=$(mktemp -d /tmp/nova-reader.XXXXXX)
LOG="$PROFILE/run.log"
NOVA_BROWSER_PROFILE="$PROFILE" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_READER=1 \
NOVA_SPEECH_SILENT=1 \
    "$BIN" >"$LOG" 2>&1

check_in 'backend=system' "$LOG" 'the built-in voice is the default engine'
check_in 'speaking=1' "$LOG" 'the page starts speaking'
check_in 'rate=50' "$LOG" 'the default reading rate is the calm 50%'
chars=$(sed -n 's/.*reader: chars=\([0-9]*\).*/\1/p' "$LOG")
if [ "${chars:-0}" -gt 100 ]; then
    echo "PASS WebKit returned the article text ($chars chars)"
    pass=$((pass + 1))
else
    echo "FAIL expected article text from WebKit, found '${chars:-}'"
    fail=$((fail + 1))
fi

echo "--- panel above the page ---"
PANEL=$(mktemp -d /tmp/nova-reader-panel.XXXXXX)
PANEL_LOG="$PANEL/run.log"
TRACE="$PANEL/gui.log"
NOVA_BROWSER_PROFILE="$PANEL" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_READER_PANEL=1 \
NOVA_SPEECH_SILENT=1 \
NOVA_GUI_TRACE="$TRACE" \
NOVA_BROWSER_UI_SNAPSHOT="$PANEL/panel.png" \
    "$BIN" >"$PANEL_LOG" 2>&1

check_in 'ui=true' "$PANEL_LOG" 'the panel frame is saved'
if grep -q 'overlay rects=1 top=1' "$TRACE"; then
    echo "PASS the panel is ordered above the page view"
    pass=$((pass + 1))
else
    echo "FAIL expected one overlay rectangle above the page"
    fail=$((fail + 1))
fi

echo "--- voice model install ---"
MODEL=$(mktemp -d /tmp/nova-reader-model.XXXXXX)
STUB="$MODEL/stub.onnx"
printf 'stub-model-bytes-for-the-install-check' >"$STUB"
MODEL_LOG="$MODEL/run.log"
NOVA_BROWSER_PROFILE="$MODEL" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_VOICE=kokoro \
NOVA_BROWSER_VOICE_FETCH=1 \
NOVA_KOKORO_URL="file://$STUB" \
NOVA_KOKORO_MIN_BYTES=16 \
    "$BIN" >"$MODEL_LOG" 2>&1

check_in "path=$MODEL/kokoro/model.onnx" "$MODEL_LOG" 'the model lands in the profile'
check_in 'ready=1' "$MODEL_LOG" 'the host reports the downloaded model'
if [ -f "$MODEL/kokoro/model.onnx" ]; then
    echo "PASS the partial file was promoted to model.onnx"
    pass=$((pass + 1))
else
    echo "FAIL model.onnx is missing"
    fail=$((fail + 1))
fi

echo "--- choosing Kokoro without a synthesizer falls back ---"
NOVA_BROWSER_PROFILE="$MODEL" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_ENGINE=kokoro \
NOVA_BROWSER_READER=1 \
NOVA_SPEECH_SILENT=1 \
    "$BIN" >"$MODEL/again.log" 2>&1
check_in 'backend=system' "$MODEL/again.log" 'a model without a command still reports the system backend'
check_in 'needs a synthesizer command' "$MODEL/again.log" 'the missing command is named, not hidden'
check_in 'reading with Albert' "$MODEL/again.log" 'the fallback says which voice actually reads'

echo "--- Kokoro job phases ---"
KOKORO=$(mktemp -d /tmp/nova-reader-kokoro.XXXXXX)
RUNNER="$KOKORO/kokoro-tts"
cat >"$RUNNER" <<'RUNNER_EOF'
#!/bin/sh
# Stand-in for a real Kokoro command: <text-file> <out.wav>. It is slow on
# purpose, because the point of the test is the phase report while it works.
sleep 0.6
python3 - "$2" <<'PY'
import sys, wave
w = wave.open(sys.argv[1], "wb")
w.setnchannels(1)
w.setsampwidth(2)
w.setframerate(16000)
w.writeframes(b"\0\0" * 8000)
w.close()
PY
RUNNER_EOF
chmod +x "$RUNNER"
mkdir -p "$KOKORO/profile/kokoro"
printf 'a-model-file' >"$KOKORO/profile/kokoro/model.onnx"
KOKORO_LOG="$KOKORO/run.log"
NOVA_BROWSER_PROFILE="$KOKORO/profile" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_ENGINE=kokoro \
NOVA_BROWSER_READER=1 \
NOVA_BROWSER_SYNTH_WAIT=1 \
NOVA_KOKORO_CMD="$RUNNER" \
NOVA_SPEECH_SILENT=1 \
    "$BIN" >"$KOKORO_LOG" 2>&1

check_in 'backend=kokoro' "$KOKORO_LOG" 'the host reports Kokoro while it works'
check_in 'phase=1' "$KOKORO_LOG" 'the job reaches the synthesizing phase'
check_in 'phase=5' "$KOKORO_LOG" 'the finished wav is reported as synthesized'
check_in 'synth: end backend=system' "$KOKORO_LOG" 'the engine returns to the system backend afterwards'
check_in 'note=' "$KOKORO_LOG" 'a finished job carries no warning'
if [ -f "$KOKORO/profile/kokoro/read.wav" ]; then
    echo "PASS the command's wav is where the host looks for it"
    pass=$((pass + 1))
else
    echo "FAIL the synthesizer output is missing"
    fail=$((fail + 1))
fi

echo "--- a failing synthesizer is reported, not hidden ---"
BAD="$KOKORO/bad-tts"
printf '#!/bin/sh\necho "no such voice" >&2\nexit 3\n' >"$BAD"
chmod +x "$BAD"
NOVA_BROWSER_PROFILE="$KOKORO/profile" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_ENGINE=kokoro \
NOVA_BROWSER_READER=1 \
NOVA_BROWSER_SYNTH_WAIT=1 \
NOVA_KOKORO_CMD="$BAD" \
NOVA_SPEECH_SILENT=1 \
    "$BIN" >"$KOKORO/bad.log" 2>&1
check_in 'phase=4' "$KOKORO/bad.log" 'a failing command reaches the failed phase'
check_in 'no such voice' "$KOKORO/bad.log" 'the command output is shown as the reason'
check_in 'synth: end backend=system' "$KOKORO/bad.log" 'a failed job leaves the system backend'

echo "--- undersized download is rejected ---"
SMALL=$(mktemp -d /tmp/nova-reader-small.XXXXXX)
printf 'tiny' >"$SMALL/tiny.onnx"
NOVA_BROWSER_PROFILE="$SMALL" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_VOICE=kokoro \
NOVA_BROWSER_VOICE_FETCH=1 \
NOVA_KOKORO_URL="file://$SMALL/tiny.onnx" \
NOVA_KOKORO_MIN_BYTES=16 \
    "$BIN" >"$SMALL/run.log" 2>&1
check_in 'Voice model download failed' "$SMALL/run.log" 'a short file never becomes model.onnx'
if [ -f "$SMALL/kokoro/model.onnx" ]; then
    echo "FAIL a rejected download left model.onnx behind"
    fail=$((fail + 1))
else
    echo "PASS a rejected download leaves nothing behind"
    pass=$((pass + 1))
fi

echo "reader: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
