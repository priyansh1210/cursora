#!/usr/bin/env bash
# The open issues' X11 checks (65, 67, 68, 70, 72, 73), scripted, in WSL as user `cursora` on the stronger PC
# (memory/dev-machines.md). From the repo on Windows, after `npm run build`:
#   wsl -d Ubuntu-24.04 -u cursora -- bash scripts/x11-issues-check.sh <part>
# Parts: `before` (no tesseract-ocr installed yet), `after` (with it), `route` (the built-in engine, no Echo).
# Builds this checkout for Linux, runs it on an X server of its own (Xvfb + Openbox), drives it with xdotool and AT-SPI,
# and prints PASS / FAIL / INFO per check. Only xterm windows showing test text are the "other apps".
set -u
src="$(cd "$(dirname "$0")/.." && pwd)"
part=${1:-}
case "$part" in before | after | route) ;; *) echo "Say which part: before, after or route"; exit 2 ;; esac
dir=~/x11-iss
model=/mnt/c/Users/FITT/Models/cursora/qwen2.5-1.5b-instruct-q4_k_m.gguf

if [ "${2:-}" != inner ]; then
  rsync -a --delete --exclude node_modules --exclude src-tauri/target --exclude src-tauri/llama --exclude .git "$src/" ~/cursora/
  echo "INFO building (a few minutes the first time)…"
  # As CI does: whisper-rs-sys's bundled bindings need no libclang. Machine-specific tools, if any, come from ~/.cursora-env.
  [ -f ~/.cursora-env ] && . ~/.cursora-env
  export WHISPER_DONT_GENERATE_BINDINGS=1
  (cd ~/cursora && ~/.cargo/bin/cargo build --quiet --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml) || { echo "FAIL the build"; exit 1; }
  rm -rf "$dir" && mkdir -p "$dir/config/com.cursora.app" "$dir/data" "$dir/cache"
  Xvfb :99 -screen 0 1600x1000x24 >/dev/null 2>&1 &
  xvfb=$!
  sleep 1
  DISPLAY=:99 dbus-run-session -- bash "$0" "$part" inner
  kill $xvfb 2>/dev/null
  exit
fi

export DISPLAY=:99 GDK_BACKEND=x11 XDG_CONFIG_HOME=$dir/config XDG_DATA_HOME=$dir/data XDG_CACHE_HOME=$dir/cache
unset WAYLAND_DISPLAY
[ "$part" != route ] && export CURSORA_ECHO=1
/usr/libexec/at-spi-bus-launcher --launch-immediately >/dev/null 2>&1 &
openbox >/dev/null 2>&1 &
command -v stalonetray >/dev/null && stalonetray >/dev/null 2>&1 &
sleep 1
gsettings set org.gnome.desktop.interface toolkit-accessibility true 2>/dev/null
dbus-send --session --dest=org.a11y.Bus /org/a11y/bus org.freedesktop.DBus.Properties.Set string:org.a11y.Status string:IsEnabled variant:boolean:true 2>/dev/null
app=~/cursora/src-tauri/target/debug/cursora
settings=$dir/config/com.cursora.app/settings.json

# AT-SPI: press a button, wait for a text anywhere in Cursora's windows, or print the texts that match.
cat > "$dir/at.py" <<'PY'
import re, sys, time
import pyatspi
def apps():
    return [a for a in pyatspi.Registry.getDesktop(0) if a is not None and "cursora" in (a.name or "").lower()]
def walk(acc):
    yield acc
    for i in range(acc.childCount):
        c = acc.getChildAtIndex(i)
        if c is not None:
            yield from walk(c)
def texts():
    for app in apps():
        for acc in walk(app):
            try:
                t = acc.queryText().getText(0, -1)
            except Exception:
                t = ""
            yield acc, (acc.name or ""), t
cmd, *args = sys.argv[1:]
wait = float(args[1]) if len(args) > 1 else 15
end = time.time() + wait
if cmd == "click":
    while time.time() < end:
        for acc, name, _ in texts():
            if name == args[0] and acc.getRoleName() in ("push button", "page tab"):
                acc.queryAction().doAction(0)
                sys.exit(0)
        time.sleep(0.4)
    sys.exit(1)
if cmd == "text":
    while time.time() < end:
        if any(args[0] in name or args[0] in t for _, name, t in texts()):
            sys.exit(0)
        time.sleep(0.5)
    sys.exit(1)
if cmd == "grep":
    seen = set()
    for _, name, t in texts():
        for s in (name, t):
            s = s.strip()
            if s and s not in seen and re.search(args[0], s):
                seen.add(s)
                print("  | " + s.replace("\n", " ")[:300])
PY
at() { python3 "$dir/at.py" "$@"; }
pass() { echo "PASS $*"; }
fail() { echo "FAIL $*"; }
check() { if eval "$1"; then pass "$2"; else fail "$2"; fi; }
diag() {
  echo "--- visible windows:"
  for w in $(xdotool search --onlyvisible --name '.' 2>/dev/null); do echo "  '$(xdotool getwindowname "$w")'"; done
  echo "--- Cursora shows:"; at grep '.' | head -40
  import -window root "$dir/screen.png" 2>/dev/null && echo '--- screenshot: \\wsl$\Ubuntu-24.04\home\cursora\x11-iss\screen.png'
}
# The bubble by size: the tray icon's window is also called "cursora".
bubble() {
  for w in $(xdotool search --onlyvisible --name '^Cursora$' 2>/dev/null); do
    [ "$(xdotool getwindowgeometry --shell "$w" | sed -n 's/^WIDTH=//p')" -ge 300 ] 2>/dev/null && { echo "$w"; return; }
  done
}
wait_bubble() { for _ in $(seq 1 40); do [ -n "$(bubble)" ] && return 0; sleep 0.3; done; return 1; }
close_bubble() { [ -n "$(bubble)" ] && { $app --ask; sleep 1; }; }
type_in_bubble() {
  local b; b=$(bubble)
  [ -n "$b" ] || return 1
  [ "$(xdotool getactivewindow)" = "$b" ] || xdotool windowactivate --sync "$b"
  xdotool type --delay 20 "$1"; sleep 0.3; xdotool key Return
}
start_app() { $app >/dev/null 2>&1 & sleep 6; }
stop_app() { $app --quit; sleep 3; }
settings_json() { python3 -c "import json,sys; json.dump(json.loads(sys.argv[1]), open('$settings','w'))" "$1"; }
# A window of large text, so text recognition reads it as a person would see it.
words_window() {
  pkill -f "xterm -T words" 2>/dev/null
  xterm -T words -fa Monospace -fs 22 -geometry 50x6+80+80 -e bash -c 'printf "\n The ephemeral beauty of cherry blossoms.\n"; sleep 900' >/dev/null 2>&1 &
  xdotool search --sync --name '^words$' windowactivate --sync >/dev/null 2>&1
  sleep 1
}
# The screen position of a word, found with Tesseract on a screenshot.
word_xy() {
  import -window root "$dir/find.png" 2>/dev/null
  tesseract "$dir/find.png" stdout tsv 2>/dev/null | awk -F'\t' -v w="$1" '$1==5 && index($12, w)==1 {print int($7+$9/2), int($8+$10/2); exit}'
}

echo "== $part (tesseract: $(command -v tesseract >/dev/null && echo installed || echo missing))"

if [ "$part" = before ]; then
  settings_json '{"setupDone": true, "theme": "dark", "hotkeys": {"ask": "Ctrl+G", "snip": "Ctrl+Shift+G", "define": "Ctrl+Alt+G", "offIn": [{"process": "xterm"}]}}'
  start_app

  echo "== Issue 67: OFF-1.2 the off list on X11 (xterm on the list)"
  rm -f "$dir/key.txt"
  xterm -T keytest -geometry 60x5+900+80 -e bash -c "stty raw -echo; head -c 1 | od -An -tx1 > $dir/key.txt; sleep 30" >/dev/null 2>&1 &
  xdotool search --sync --name '^keytest$' windowactivate --sync >/dev/null 2>&1; sleep 1
  xdotool key --clearmodifiers ctrl+g; sleep 2
  check '[ -z "$(bubble)" ]' "Ctrl+G in xterm opened no bubble"
  check 'grep -q 07 "$dir/key.txt" 2>/dev/null' "xterm got Ctrl+G itself (key: '$(cat "$dir/key.txt" 2>/dev/null | tr -d ' ')')"
  # Elsewhere, a window that isn't on the list: mousepad.
  printf 'Elsewhere the hotkey opens the bubble.\n' > "$dir/note.txt"
  mousepad "$dir/note.txt" >/dev/null 2>&1 &
  xdotool search --sync --name 'note.txt' windowactivate --sync >/dev/null 2>&1; sleep 2
  xdotool key --clearmodifiers ctrl+g
  check 'wait_bubble' "the hotkey was taken back: Ctrl+G opened the bubble in mousepad"
  close_bubble

  echo "== Issue 67: OFF-2.1 the tray item on X11"
  tray=$(xdotool search --class stalonetray 2>/dev/null | head -1)
  if [ -n "$tray" ]; then
    eval "$(xdotool getwindowgeometry --shell "$tray")"
    xdotool mousemove $((X + 12)) $((Y + 12)) click 1; sleep 1.5
    echo "INFO the tray menu's items about turning off:"; at grep 'Turn Cursora off|Block Cursora'
    check 'at text "Turn Cursora off in" 3' "the tray menu has 'Turn Cursora off in <app>'"
    xdotool key Escape; sleep 0.5
  else
    fail "no tray host window (stalonetray)"
  fi
  pkill -f "mousepad $dir/note.txt" 2>/dev/null
  stop_app

  # The rest without the off list, which holds xterm.
  settings_json '{"setupDone": true, "theme": "dark"}'
  start_app

  echo "== Issue 73: WEB-1.4 /web on X11 without a key"
  words_window; $app --ask; wait_bubble; sleep 1
  type_in_bubble "/web tauri release"
  check 'at text "Web search needs a Tavily API key" 10' "without a key, /web says where to add one"
  close_bubble

  echo "== Issue 65: OCR-1.6 snip with a model that can't see, without tesseract-ocr"
  $app --snip; sleep 3
  check 'wait_bubble' "the bubble opened instead of the snip overlay"
  check 'at text "install tesseract-ocr" 8' "it says that installing tesseract-ocr lets any model use snips"
  [ -n "$(bubble)" ] || diag
  close_bubble

  echo "== Issue 72: HOV-1.4 Define without tesseract-ocr"
  words_window
  xdotool mousemove 300 160; sleep 0.5
  xdotool key --clearmodifiers ctrl+alt+g
  check 'wait_bubble' "Ctrl+Alt+G opened the bubble"
  check 'at text "Install tesseract-ocr so Cursora can read the word under the pointer" 8' "it says to install tesseract-ocr"
  close_bubble
  stop_app

  echo "== Issue 73: WEB-1.4 /web in local-only mode"
  settings_json '{"setupDone": true, "theme": "dark", "localOnly": true}'
  start_app
  words_window; $app --ask; wait_bubble; sleep 1
  type_in_bubble "/web tauri release"
  check 'at text "Web search is off in local-only mode" 10' "in local-only mode, /web sends nothing and says so"
  close_bubble
  stop_app
fi

if [ "$part" = after ]; then
  command -v tesseract >/dev/null || { echo "FAIL tesseract-ocr isn't installed yet"; exit 1; }
  settings_json '{"setupDone": true, "theme": "dark"}'
  start_app

  echo "== Issue 72: HOV-1.2, HOV-1.3 Define the word under the pointer on X11"
  words_window
  xy=$(word_xy ephemeral)
  echo "INFO 'ephemeral' is at ${xy:-nowhere} on the screen"
  if [ -n "$xy" ]; then
    xdotool mousemove $xy; sleep 0.5
    xdotool key --clearmodifiers ctrl+alt+g
    check 'wait_bubble' "Ctrl+Alt+G opened the bubble"
    check 'at text "ephemeral" 8' "the bubble has the word 'ephemeral' as its context"
    check 'at text "/define" 15' "it asked /define by itself"
    echo "INFO the bubble shows:"; at grep 'ephemeral|define|characters|No word'
    close_bubble
    xy=$(word_xy blossoms)
    xdotool mousemove $xy; sleep 0.5; xdotool key --clearmodifiers ctrl+alt+g
    check 'wait_bubble && at text "blossoms" 8' "another word: 'blossoms'"
    close_bubble
  else
    fail "Tesseract didn't find the test word on the screen"; diag
  fi

  echo "== Issue 65: OCR-1.3 snip text with a model that can't see"
  words_window
  $app --snip; sleep 3
  eval "$(xdotool search --name '^words$' getwindowgeometry --shell 2>/dev/null | head -5)"
  xdotool mousemove $((X + 5)) $((Y + 5)) mousedown 1 mousemove $((X + 300)) $((Y + 60)) mousemove $((X + 640)) $((Y + 130)) mouseup 1; sleep 3
  check 'wait_bubble' "the bubble opened with the snip"
  check 'at text "Read as text" 10' "it says the snip was read as text"
  echo "INFO the bubble shows:"; at grep 'Read as text|characters|No text'
  type_in_bubble "What does it say?"
  check 'at text "You asked" 20' "a question about it was answered"
  [ -n "$(bubble)" ] || diag
  close_bubble
  stop_app
fi

if [ "$part" = route ]; then
  [ -f "$model" ] || { echo "FAIL no model file at $model"; exit 1; }
  echo "INFO the engine: $(ls -l ~/cursora/src-tauri/llama 2>/dev/null | head -2 | tail -1)"
  # A LAN server that is switched off as the default (issue 70), and the built-in engine as the quick model.
  settings_json '{"setupDone": true, "theme": "dark",
    "models": {"providers": [
      {"id": "p-engine", "name": "Built-in engine", "kind": "builtin", "preset": "builtin", "baseUrl": "", "models": ["qwen2.5-1.5b-instruct-q4_k_m"], "engine": {"gguf": "'"$model"'", "lifetime": "always", "gpu": true}},
      {"id": "p-lan", "name": "LAN server", "kind": "openai-compatible", "preset": "custom", "baseUrl": "http://192.168.250.250:1234/v1", "models": ["lan-model"]}],
      "default": {"provider": "p-lan", "model": "lan-model"}, "quick": {"provider": "p-engine", "model": "qwen2.5-1.5b-instruct-q4_k_m"},
      "trustedHosts": ["192.168.250.250"], "timeouts": {"connectSecs": 3, "firstReplyLocalSecs": 120, "firstReplyCloudSecs": 60, "pauseSecs": 60}}}'
  start_app; sleep 10   # the engine loads the model from C:

  echo "== Issue 68: GPU-1.x the Linux engine without a graphics card"
  log=$(find "$dir" -name '*.log' | head -1)
  grep -h "gpu:\|engine:" "$log" 2>/dev/null | tail -3 | sed 's/^/INFO log: /'
  check 'grep -q "on the processor" "$log" 2>/dev/null' "the engine runs on the processor"
  $app --settings; sleep 3
  at click "Models" 5; sleep 2
  check 'at text "No dedicated graphics card was found" 8' "Settings → Models says the processor answers"
  at grep 'graphics card|processor'
  xdotool search --name 'Cursora settings' windowactivate --sync key --clearmodifiers alt+F4 2>/dev/null; sleep 1

  echo "== Issue 70: ROU-1.2, 1.5 a quick job on the quick model"
  words_window
  $app --ask
  check 'wait_bubble' "the bubble kept its input with the default server off (ROU-2.3)"
  sleep 4
  check '! at text "No AI model is available" 1' "it doesn't say no model is available"
  type_in_bubble "/fix this"
  check 'at text "Quick job: answered by qwen2.5-1.5b-instruct-q4_k_m." 90' "/fix was answered by the quick model, with the note"

  echo "== Issue 70: ROU-2.1 the LAN default is off, so the quick model is asked"
  sleep 3
  type_in_bubble "What is 2 plus 2? One word."
  check 'at text "lan-model couldn'"'"'t be reached, so qwen2.5-1.5b-instruct-q4_k_m was asked instead." 90' "a free question fell back to the quick model, with the note"
  echo "INFO the bubble shows:"; at grep 'Quick job|reached|asked instead|LAN|Local'
  [ -n "$(bubble)" ] || diag
  close_bubble
  stop_app
fi

pkill -f "xterm -T words" 2>/dev/null; pkill -f "xterm -T keytest" 2>/dev/null
pkill openbox 2>/dev/null; pkill stalonetray 2>/dev/null
echo "== done"
