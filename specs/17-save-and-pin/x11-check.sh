#!/usr/bin/env bash
# Spec 17's manual checklist on X11, scripted, in WSL as user `cursora` (memory/dev-machines.md). From the repo on Windows,
# after `npm run build`:   wsl -d Ubuntu-24.04 -u cursora -- bash specs/17-save-and-pin/x11-check.sh
# Builds this checkout for Linux, runs it with the Echo test model on an X server of its own (Xvfb + Openbox), drives it
# through AT-SPI and xdotool, and prints PASS / FAIL / INFO per check. Only mousepad, on a test file, is the "other app".
# `... x11-check.sh light` runs only the light-theme check.
set -u
src="$(cd "$(dirname "$0")/../.." && pwd)"

if [ "${1:-}" != inner ]; then
  rsync -a --delete --exclude node_modules --exclude src-tauri/target --exclude src-tauri/llama --exclude .git "$src/" ~/cursora/
  echo "INFO building (a few minutes the first time)…"
  (cd ~/cursora && ~/.cargo/bin/cargo build --quiet --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml) || { echo "FAIL the build"; exit 1; }
  rm -rf ~/x11-sav && mkdir -p ~/x11-sav/config/com.cursora.app ~/x11-sav/data ~/x11-sav/cache
  echo '{"setupDone": true, "theme": "dark"}' > ~/x11-sav/config/com.cursora.app/settings.json
  echo "The meeting is on Friday." > ~/x11-sav/note.txt
  Xvfb :99 -screen 0 1600x1000x24 >/dev/null 2>&1 &
  xvfb=$!
  sleep 1
  DISPLAY=:99 dbus-run-session -- bash "$0" inner "${1:-all}"
  kill $xvfb 2>/dev/null
  exit
fi

only=${2:-all}
export DISPLAY=:99 GDK_BACKEND=x11 CURSORA_ECHO=1 XDG_CONFIG_HOME=~/x11-sav/config XDG_DATA_HOME=~/x11-sav/data XDG_CACHE_HOME=~/x11-sav/cache
unset WAYLAND_DISPLAY
/usr/libexec/at-spi-bus-launcher --launch-immediately >/dev/null 2>&1 &
openbox >/dev/null 2>&1 &
command -v stalonetray >/dev/null && stalonetray >/dev/null 2>&1 &
sleep 1
# With no desktop or screen reader, apps keep their windows' contents from AT-SPI until accessibility is on.
gsettings set org.gnome.desktop.interface toolkit-accessibility true 2>/dev/null
dbus-send --session --dest=org.a11y.Bus /org/a11y/bus org.freedesktop.DBus.Properties.Set string:org.a11y.Status string:IsEnabled variant:boolean:true 2>/dev/null
app=~/cursora/src-tauri/target/debug/cursora
saved=~/Documents/Cursora
had_saved=$([ -d "$saved" ] && echo yes || echo no)
before=$(ls "$saved" 2>/dev/null)

# AT-SPI: press a button by its accessible name, wait for one, or list what Cursora shows (for a failure).
cat > ~/x11-sav/at.py <<'PY'
import sys, time
import pyatspi
def apps():
    return [a for a in pyatspi.Registry.getDesktop(0) if a is not None and "cursora" in (a.name or "").lower()]
def walk(acc, depth=0):
    yield acc, depth
    for i in range(acc.childCount):
        c = acc.getChildAtIndex(i)
        if c is not None:
            yield from walk(c, depth + 1)
def find(name, role=None):
    for app in apps():
        for acc, _ in walk(app):
            try:
                if acc.name == name and (role is None or acc.getRoleName() == role):
                    return acc
            except Exception:
                pass
cmd, *args = sys.argv[1:]
if cmd in ("click", "has"):
    end = time.time() + float(args[1] if len(args) > 1 else 15)
    while time.time() < end:
        a = find(args[0], "push button" if cmd == "click" else None)
        if a:
            if cmd == "click":
                a.queryAction().doAction(0)
            sys.exit(0)
        time.sleep(0.4)
    sys.exit(1)
if cmd == "in":  # a button inside one window, by the window's title
    end = time.time() + 5
    while time.time() < end:
        for app in apps():
            for frame, _ in walk(app):
                if frame.getRoleName() == "frame" and frame.name == args[0]:
                    if any(a.name == args[1] for a, _ in walk(frame)):
                        sys.exit(0)
        time.sleep(0.4)
    sys.exit(1)
if cmd == "alerts":
    for app in apps():
        for acc, _ in walk(app):
            if acc.getRoleName() in ("alert", "notification"):
                try:
                    print(acc.queryText().getText(0, -1))
                except Exception:
                    print(acc.name)
if cmd == "apps":  # every app AT-SPI knows, with its top-level children
    for app in pyatspi.Registry.getDesktop(0):
        if app is None:
            continue
        print(f"  app '{app.name}': {app.childCount} children")
        for i in range(min(app.childCount, 8)):
            c = app.getChildAtIndex(i)
            print(f"    {c.getRoleName()}: '{c.name}', {c.childCount} children")
if cmd == "dump":
    for app in apps():
        for acc, d in walk(app):
            if acc.name or acc.getRoleName() in ("frame", "push button"):
                print("  " * min(d, 10) + f"{acc.getRoleName()}: {acc.name}")
PY
at() { python3 ~/x11-sav/at.py "$@"; }
pass() { echo "PASS $*"; }
fail() { echo "FAIL $*"; }
check() { if eval "$1"; then pass "$2"; else fail "$2"; fi; }
diag() {
  echo "--- visible windows:"
  for w in $(xdotool search --onlyvisible --name '.' 2>/dev/null); do echo "  $w '$(xdotool getwindowname "$w")' $(xdotool getwindowgeometry "$w" | tail -2 | tr -s ' \n' ' ')"; done
  echo "--- active window: '$(xdotool getactivewindow getwindowname 2>/dev/null)'"
  echo "--- AT-SPI:"; at apps
  import -window root ~/x11-sav/screen.png 2>/dev/null && echo '--- screenshot: \\wsl$\Ubuntu-24.04\home\cursora\x11-sav\screen.png'
}
notes() { xdotool search --onlyvisible --name "$1" 2>/dev/null | wc -l; }
# By size: xdotool ignores letter case, and the tray icon's window (24 px) is called "cursora".
bubble() {
  for w in $(xdotool search --onlyvisible --name '^Cursora$' 2>/dev/null); do
    [ "$(xdotool getwindowgeometry --shell "$w" | sed -n 's/^WIDTH=//p')" -ge 300 ] 2>/dev/null && { echo "$w"; return; }
  done
}
geom() { xdotool getwindowgeometry --shell "$1" | tr '\n' ' '; }
brightness() { import -window "$1" -crop 1x1+4+4 -depth 8 txt:- 2>/dev/null | grep -o '#[0-9A-F]\{6\}' | head -1 | cut -c2-3 | xargs -I{} printf '%d' 0x{}; }

# A conversation in the bubble about mousepad's selected text, with the question typed in.
ask() {
  [ -n "$(bubble)" ] && { $app --ask; sleep 1; }
  xdotool search --sync --name 'note.txt' windowactivate --sync >/dev/null 2>&1
  xdotool key --clearmodifiers ctrl+a; sleep 0.5
  $app --ask
  for _ in $(seq 1 30); do [ -n "$(bubble)" ] && break; sleep 0.3; done
  sleep 1
  b=$(bubble)
  [ -n "$b" ] || { echo "INFO the bubble didn't open"; return 1; }
  [ "$(xdotool getactivewindow)" = "$b" ] || xdotool windowactivate --sync "$b"
  echo "INFO typing into '$(xdotool getactivewindow getwindowname 2>/dev/null)'"
  xdotool type --delay 15 "$1"; xdotool key Return
  at has "Pin this answer on screen in a note" 20
}

mousepad ~/x11-sav/note.txt >/dev/null 2>&1 &
sleep 3

if [ "$only" = all ]; then
$app >/dev/null 2>&1 &
sleep 5

echo "== SAV-1.1 Pin, from the bubble"
if ask "When is the report due?"; then pass "a finished answer offers Pin and Save"; else fail "no Pin button after the answer"; diag; $app --quit; exit 1; fi
b=$(bubble); bg=$(geom "$b")
at click "Pin this answer on screen in a note"; sleep 2
check '[ "$(notes "When is the report due?")" -eq 1 ]' "Pin opened a note titled with the question"
n=$(xdotool search --onlyvisible --name "When is the report due?" | head -1)
if [ -n "$n" ] && [ -n "$b" ]; then
  ng=$(geom "$n"); echo "INFO bubble: $bg"; echo "INFO note:   $ng"
  eval "$(xdotool getwindowgeometry --shell "$b" | sed 's/^/B_/')"; eval "$(xdotool getwindowgeometry --shell "$n" | sed 's/^/N_/')"
  check '[ "$N_X" -ge $((B_X + B_WIDTH)) ] || [ $((N_X + N_WIDTH)) -le "$B_X" ]' "the note sits beside the bubble"
  check 'xprop -id "$n" _NET_WM_STATE | grep -q _NET_WM_STATE_ABOVE' "the note stays on top"
  check '[ "$(xdotool getactivewindow)" != "$n" ]' "the note didn't take the typing focus"
  check '[ "$(brightness "$n")" -lt 80 ]' "the note is dark, like the settings (SAV-1.4)"
fi
check 'at in "When is the report due?" "Copy the answer. Hold Shift for plain text"' "the note has Copy"

echo "== SAV-1.3 / R72 Pin after Regenerate"
at click "Regenerate the answer"; sleep 1
at has "Pin this answer on screen in a note" 15
at click "Pin this answer on screen in a note"; sleep 2
check '[ "$(notes "When is the report due?")" -eq 2 ]' "Pin works on a regenerated answer"
check '! at alerts | grep -q "Only a finished answer"' "no 'Only a finished answer' error"

echo "== SAV-2.1, 2.2, 2.4 Save"
at click "Save this answer as a Markdown file"; sleep 1.5
check 'at has "Show the saved answer in its folder" 3' "Save offers Show in folder"
file=$(ls -t "$saved"/*.md 2>/dev/null | head -1)
if [ -n "$file" ]; then
  echo "INFO saved: $file"; sed 's/^/  | /' "$file"
  check '[[ "$file" == "$saved/"*" When is the report due.md" ]]' "the name is the date and the question, safe and short"
  check 'grep -q "^> When is the report due?" "$file" && grep -q "^saved: \"" "$file" && grep -q "^app: " "$file"' "the file has the time, app, model and the quoted question"
  check '! grep -q "The meeting is on Friday" "$file"' "the selected text isn't in the file"
else
  fail "no file in $saved"; at alerts
fi
at click "Show the saved answer in its folder"; sleep 2
echo "INFO Show in folder: alerts: '$(at alerts | tr '\n' ' ')' windows: $(xdotool search --onlyvisible --name 'Cursora' 2>/dev/null | wc -l) (WSL has no file manager, so an error here is the environment)"

echo "== SAV-1.1 Pin, from the expanded window"
at click "Expand to a larger window"; sleep 2
at click "Pin this answer on screen in a note"; sleep 2
check '[ "$(notes "When is the report due?")" -eq 3 ]' "Pin works in the expanded window"
w=$(bubble); [ -n "$w" ] && xdotool windowactivate --sync "$w" key --clearmodifiers alt+F4; sleep 1   # the expanded window, closed as a user would

echo "== SAV-1.2 Notes stay; two at once"
ask "Who signs the contract?" >/dev/null && at click "Pin this answer on screen in a note"; sleep 2
$app --ask; sleep 1
check '[ "$(notes "Who signs the contract?")" -eq 1 ] && [ "$(notes "When is the report due?")" -ge 1 ]' "notes from two conversations stay with the bubble closed"

echo "== SAV-2.5 Settings → General"
$app --settings; sleep 3
check 'at has "Change…" 5 && at has "Open" 3' "Saved answers has Change… and Open"

echo "== SAV-1.2 Quit"
$app --quit; sleep 3
check '[ "$(notes "When is the report due?")" -eq 0 ] && [ "$(notes "Who signs the contract?")" -eq 0 ]' "the notes close with Cursora"

fi

echo "== SAV-1.4 Light theme"
python3 -c "import json,os;p=os.path.expanduser('~/x11-sav/config/com.cursora.app/settings.json');s=json.load(open(p));s['theme']='light';json.dump(s,open(p,'w'))"
$app >/dev/null 2>&1 &
sleep 5
if ask "Is it light?"; then at click "Pin this answer on screen in a note"; sleep 2; else fail "no answer to pin after the restart"; fi
n=$(xdotool search --onlyvisible --name "Is it light" | head -1)
echo "INFO note: '${n:-none}', its corner's red value (0 to 255): $( [ -n "$n" ] && brightness "$n")"
check '[ -n "$n" ] && [ "$(brightness "$n")" -gt 170 ]' "the note is light with the light theme"
[ -n "$n" ] || diag
$app --quit; sleep 2

# Tidy up what this run made; keep ~/x11-sav for a look afterwards.
pkill -f "mousepad $HOME/x11-sav/note.txt" 2>/dev/null
if [ "$had_saved" = no ]; then rm -rf "$saved"; else for f in "$saved"/*.md; do grep -qxF "$(basename "$f")" <<<"$before" || rm -f "$f"; done; fi
pkill openbox 2>/dev/null; pkill stalonetray 2>/dev/null
echo "== done"
