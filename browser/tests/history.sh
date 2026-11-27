#!/bin/sh
# Headless tests for the Nova Browser library, plus a real import run against
# an on-disk Chrome profile export.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "history: compiler missing" >&2; exit 1; }

echo "--- library model ---"
(cd "$HERE/library" && "$TOOL" build >/dev/null)
"$HERE/library/target/nova/nova-browser-library-test" | tee /tmp/nova-browser-history.txt
if grep -q '^FAIL ' /tmp/nova-browser-history.txt; then
    exit 1
fi
grep -q '^failures=0$' /tmp/nova-browser-history.txt

echo "--- google import (app CLI, temp profile) ---"
(cd "$ROOT/browser" && "$TOOL" build >/dev/null)
PROFILE=$(mktemp -d /tmp/nova-browser-profile.XXXXXX)
CHROME=$(mktemp -d /tmp/nova-chrome-profile.XXXXXX)
SETTINGS_PROFILE=$(mktemp -d /tmp/nova-browser-profile.XXXXXX)
trap 'rm -rf "$PROFILE" "$CHROME" "$SETTINGS_PROFILE"' EXIT
OUT=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$HERE/fixtures/chrome-bookmarks.json" 2>&1)
echo "$OUT"
echo "$OUT" | grep -q 'import: files=1 found=2 new=2 bookmarks=2'
[ -s "$PROFILE/bookmarks.tsv" ] || { echo "history: bookmarks file missing" >&2; exit 1; }

# A second import must not duplicate anything.
OUT2=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$HERE/fixtures/chrome-bookmarks.json" 2>&1)
echo "$OUT2"
echo "$OUT2" | grep -q 'new=0 bookmarks=2'

# A Netscape export imports through the same entry point.
OUT3=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$HERE/fixtures/bookmarks-export.html" 2>&1)
echo "$OUT3"
echo "$OUT3" | grep -q 'found=1 new=1 bookmarks=3'

echo "--- chrome profile (bookmarks, history, search) ---"
# A real Chrome profile shape: the JSON bookmarks file, a Preferences file with
# the default search engine, and a SQLite History database. sqlite3 ships with
# macOS, which is where this profile format exists.
mkdir -p "$CHROME/Default"
cp "$HERE/fixtures/chrome-bookmarks.json" "$CHROME/Default/Bookmarks"
cat > "$CHROME/Default/Preferences" <<'JSON'
{"default_search_provider_data":{"template_url_data":{"short_name":"DuckDuckGo","keyword":"duckduckgo.com"}}}
JSON
sqlite3 "$CHROME/Default/History" <<'SQL'
create table urls (id integer primary key, url text, title text, visit_count integer default 0,
                   typed_count integer default 0, last_visit_time integer not null, hidden integer default 0);
insert into urls (url, title, visit_count, last_visit_time) values
  ('https://example.com/one', 'One', 3, 3),
  ('https://example.com/two', 'Two', 1, 2),
  ('chrome://settings', 'Settings', 9, 1);
SQL
OUT4=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$SETTINGS_PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$CHROME/Default" 2>&1)
echo "$OUT4"
echo "$OUT4" | grep -q 'files=1 found=2 new=2 bookmarks=2 history=2 search=duckduckgo'
[ -s "$SETTINGS_PROFILE/history.tsv" ] || { echo "history: imported visits missing" >&2; exit 1; }
grep -q '^visit' "$SETTINGS_PROFILE/history.tsv"
grep -q '^search=duckduckgo$' "$SETTINGS_PROFILE/settings.tsv"
[ "$(grep -c '^visit' "$SETTINGS_PROFILE/history.tsv")" = "2" ] || { echo "history: expected two visits" >&2; exit 1; }

# The same profile again adds no bookmarks and no visits.
OUT5=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$SETTINGS_PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$CHROME/Default" 2>&1)
echo "$OUT5"
echo "$OUT5" | grep -q 'new=0 bookmarks=2 history=0 search=duckduckgo'

# Cmd+Shift+I discovers profiles under the home directory. Point HOME at the
# fixture so the real Chrome profile is never touched.
echo "--- profile discovery (HOME override) ---"
FAKEHOME=$(mktemp -d /tmp/nova-fakehome.XXXXXX)
mkdir -p "$FAKEHOME/Library/Application Support/Google/Chrome/Default"
cp "$CHROME/Default/Bookmarks" "$CHROME/Default/Preferences" "$CHROME/Default/History" \
    "$FAKEHOME/Library/Application Support/Google/Chrome/Default/"
DISCOVERED=$(mktemp -d /tmp/nova-browser-profile.XXXXXX)
trap 'rm -rf "$PROFILE" "$CHROME" "$SETTINGS_PROFILE" "$FAKEHOME" "$DISCOVERED"' EXIT
OUT6=$(cd "$ROOT" && HOME="$FAKEHOME" NOVA_BROWSER_PROFILE="$DISCOVERED" \
    "$ROOT/browser/target/nova/nova-browser" --import 2>&1)
echo "$OUT6"
echo "$OUT6" | grep -q 'files=1 found=2 new=2 bookmarks=2 history=2 search=duckduckgo'
