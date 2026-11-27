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
# A real Preferences file is tens of megabytes. A large filler proves the
# import reads only the keys it needs instead of parsing the whole document.
FILLER=$(dd if=/dev/zero bs=1024 count=1024 2>/dev/null | tr '\0' 'x')
cat > "$CHROME/Default/Preferences" <<JSON
{"junk":"$FILLER","default_search_provider_data":{"template_url_data":{"short_name":"DuckDuckGo","keyword":"duckduckgo.com"}},"homepage":"https://example.com/home","homepage_is_newtabpage":false,"session":{"restore_on_startup":4,"startup_urls":["https://fixture.nova.invalid/start","https://fixture.nova.invalid/start2","mailto:skip@example.com"]}}
JSON
sqlite3 "$CHROME/Default/History" <<'SQL'
create table urls (id integer primary key, url text, title text, visit_count integer default 0,
                   typed_count integer default 0, last_visit_time integer not null, hidden integer default 0);
insert into urls (url, title, visit_count, last_visit_time) values
  ('https://example.com/one', 'One', 3, 3),
  ('https://example.com/two', 'Two', 1, 2),
  ('chrome://settings', 'Settings', 9, 1);
SQL
# A completed Chrome download whose file still exists on disk is imported; the
# row points at the real path, so the shelf can reveal it.
printf 'report' > "$CHROME/Default/Downloaded Report.pdf"
sqlite3 "$CHROME/Default/History" "create table downloads (id integer primary key, target_path text, tab_url text, total_bytes integer, state integer, start_time integer default 0); insert into downloads (target_path, tab_url, total_bytes, state) values ('$CHROME/Default/Downloaded Report.pdf', 'https://example.com/one', 6, 1);"

# Chrome extensions are reported, not run: the same profile carries two
# installed extensions, one with a literal name and one behind an i18n key.
mkdir -p "$CHROME/Default/Extensions/clearcache/1.0" \
    "$CHROME/Default/Extensions/msgname/2.0/_locales/en"
cat > "$CHROME/Default/Extensions/clearcache/1.0/manifest.json" <<JSON
{"manifest_version":3,"name":"Clear Cache","version":"1.0"}
JSON
cat > "$CHROME/Default/Extensions/msgname/2.0/manifest.json" <<JSON
{"manifest_version":3,"name":"__MSG_extName__","version":"2.0"}
JSON
cat > "$CHROME/Default/Extensions/msgname/2.0/_locales/en/messages.json" <<JSON
{"extName":{"message":"Reading List Helper"}}
JSON

OUT4=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$SETTINGS_PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$CHROME/Default" 2>&1)
echo "$OUT4"
echo "$OUT4" | grep -q 'files=1 found=2 new=2 bookmarks=2 history=2 startup=2 homepage=https://example.com/home search=duckduckgo'
[ -s "$SETTINGS_PROFILE/history.tsv" ] || { echo "history: imported visits missing" >&2; exit 1; }
grep -q '^visit' "$SETTINGS_PROFILE/history.tsv"
grep -q '^search=duckduckgo$' "$SETTINGS_PROFILE/settings.tsv"
grep -q '^startup=pages$' "$SETTINGS_PROFILE/settings.tsv"
grep -q '^homepage=https://example.com/home$' "$SETTINGS_PROFILE/settings.tsv"
echo "$OUT4" | grep -q 'extensions=2'
grep -q 'Clear Cache' "$SETTINGS_PROFILE/chrome-extensions.tsv"
grep -q 'Reading List Helper' "$SETTINGS_PROFILE/chrome-extensions.tsv"
echo "$OUT4" | grep -q 'downloads=1'
grep -q 'Downloaded Report.pdf' "$SETTINGS_PROFILE/downloads.tsv"
[ "$(grep -c '^visit' "$SETTINGS_PROFILE/history.tsv")" = "2" ] || { echo "history: expected two visits" >&2; exit 1; }
# Startup pages keep Chrome's order; the mailto: entry is skipped.
grep -q 'tab\thttps://fixture.nova.invalid/start$' "$SETTINGS_PROFILE/startup.tsv"
[ "$(grep -c '^tab' "$SETTINGS_PROFILE/startup.tsv")" = "2" ] || { echo "history: expected two startup pages" >&2; exit 1; }

# The same profile again adds no bookmarks and no visits.
OUT5=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$SETTINGS_PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$CHROME/Default" 2>&1)
echo "$OUT5"
echo "$OUT5" | grep -q 'new=0 bookmarks=2 history=0 startup=2 homepage=https://example.com/home search=duckduckgo'

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
echo "$OUT6" | grep -q 'files=1 found=2 new=2 bookmarks=2 history=2 startup=2 homepage=https://example.com/home search=duckduckgo'

# The imported startup choice is what the next launch actually opens.
echo "--- startup pages at launch ---"
OUT7=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$SETTINGS_PROFILE" NOVA_BROWSER_STARTUP_PROBE=1 \
    NOVA_BROWSER_NO_HISTORY=1 NOVA_BROWSER_AUTOCLOSE=1 \
    "$ROOT/browser/target/nova/nova-browser" 2>&1)
echo "$OUT7" | grep -q 'startup: mode=pages tabs=2 first=https://fixture.nova.invalid/start'

# A dry run reports the same counts and writes nothing at all.
echo "--- dry run writes nothing ---"
DRY=$(mktemp -d /tmp/nova-browser-profile.XXXXXX)
trap 'rm -rf "$PROFILE" "$CHROME" "$SETTINGS_PROFILE" "$FAKEHOME" "$DISCOVERED" "$DRY"' EXIT
OUT8=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$DRY" NOVA_BROWSER_IMPORT_DRY=1 \
    "$ROOT/browser/target/nova/nova-browser" --import "$CHROME/Default" 2>&1)
echo "$OUT8"
echo "$OUT8" | grep -q 'import(dry): files=1 found=2 new=2 bookmarks=2 history=2 startup=2 homepage=https://example.com/home search=duckduckgo'
for f in bookmarks.tsv history.tsv startup.tsv settings.tsv session.tsv downloads.tsv chrome-extensions.tsv; do
    [ ! -e "$DRY/$f" ] || { echo "history: dry run wrote $f" >&2; exit 1; }
done
