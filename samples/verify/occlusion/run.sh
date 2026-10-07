#!/bin/sh
# Does a fully covered Safari window keep being captured? (2026-10-07 request: "Terminal in
# front".) Plays the F1 fixture (88 s, 7 slides) as a slideshow in Safari twice while the
# app records the Safari window: once visible, once completely covered by this Terminal
# window. Prints the number of saves for each (7 expected). About 4 minutes; do not use the
# Mac meanwhile, and keep the screen unlocked (a locked screen stops all drawing).
#
# Run from the repository root, in Terminal.app, after `pnpm tauri build --debug --bundles app`:
#     sh samples/verify/occlusion/run.sh
set -e
REPO=$(pwd)
APP="$REPO/target/debug/bundle/macos/ZoomSlidesPicker.app/Contents/MacOS/zoom-slides-picker"
[ -x "$APP" ] || { echo "build the app first: pnpm tauri build --debug --bundles app"; exit 1; }
WORK=$(mktemp -d -t zsp-occl)
cp samples/verify/occlusion/show.html samples/verify/occlusion/serve.py "$WORK/"
mkdir -p "$WORK/frames"
ffmpeg -v error -i fixtures/f1_2fps.mp4 -q:v 3 "$WORK/frames/%04d.jpg"
N=$(ls "$WORK/frames" | wc -l | tr -d ' ')
(cd "$WORK" && python3 serve.py 8767 > /dev/null 2>&1) &
SERVER=$!
trap 'kill $SERVER 2>/dev/null' EXIT
sleep 1

run_phase() {  # $1 = label, $2 = 1 to cover with this Terminal window
  ID="$1-$(date +%H%M%S)"
  osascript -e "tell application \"Safari\" to make new document with properties {URL:\"http://127.0.0.1:8767/show.html?id=$ID&n=$N\"}" >/dev/null
  if [ "$2" = 1 ]; then
    # Put the Safari window inside this Terminal window's frame, then bring Terminal in front.
    B=$(osascript -e 'tell application "Terminal" to get bounds of front window')
    L=$(echo "$B" | cut -d, -f1 | tr -d ' '); T=$(echo "$B" | cut -d, -f2 | tr -d ' ')
    R=$(echo "$B" | cut -d, -f3 | tr -d ' '); BOT=$(echo "$B" | cut -d, -f4 | tr -d ' ')
    osascript -e "tell application \"Safari\" to set bounds of front window to {$((L + 20)), $((T + 40)), $((R - 20)), $((BOT - 20))}" \
              -e 'tell application "Terminal" to activate' >/dev/null
  else
    osascript -e 'tell application "Safari" to set bounds of front window to {80, 80, 1080, 680}' \
              -e 'tell application "Safari" to activate' >/dev/null
  fi
  sleep 2
  ZSP_AUTOSTART_TITLE="ZSP-OCCL-$ID" ZSP_AUTOSTART_SECONDS=95 ZSP_SAVE_ROOT="$WORK/out-$1" "$APP" > "$WORK/app-$1.log" 2>&1
  osascript -e "tell application \"Safari\" to close (every window whose name contains \"ZSP-OCCL-$ID\")" >/dev/null 2>&1 || true
  COUNT=$(python3 -c "import json,glob; print(len(json.load(open(glob.glob('$WORK/out-$1/*/session.json')[0]))['captures']))" 2>/dev/null || echo "?")
  echo "$1: $COUNT saves (7 expected)"
}

echo "1/2 visible (about 100 s) ..."
run_phase visible 0
echo "2/2 covered by this Terminal window (about 100 s) ..."
run_phase covered 1
echo "folders: $WORK"
