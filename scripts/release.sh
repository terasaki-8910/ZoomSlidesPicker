#!/usr/bin/env bash
# Builds, signs, and publishes a release (docs/impl/updates.md > Release side).
#
#   bash scripts/release.sh <version> <notes.md>
#
# - Builds the macOS DMG and app update bundle, and the Windows installer (cargo-xwin),
#   with the updater key: ~/.tauri/zoomslidespicker-updater.key, its password from the
#   Keychain item "zoomslidespicker-updater-key".
# - Writes latest.json for the updater (darwin-aarch64, windows-x86_64).
# - Tags v<version>, pushes main and the tag to origin (this private repository), and
#   creates its release with the installers.
# - Creates the same release in the public repository the app polls
#   (terasaki-8910/ZoomSlidesPicker-releases) with the installers, the update bundles, and
#   latest.json. This step goes last: installed copies see the new version from then on.
#
# Set ZSP_RELEASE_DRY_RUN=1 to build and write latest.json without tagging or uploading.
set -euo pipefail
cd "$(dirname "$0")/.."

PUBLIC_REPO="terasaki-8910/ZoomSlidesPicker-releases"
KEY_FILE="$HOME/.tauri/zoomslidespicker-updater.key"
KEYCHAIN_ITEM="zoomslidespicker-updater-key"

version="${1:?usage: release.sh <version> <notes.md>}"
notes_file="${2:?usage: release.sh <version> <notes.md>}"
tag="v$version"
dry="${ZSP_RELEASE_DRY_RUN:-}"

die() { printf 'release.sh: %s\n' "$*" >&2; exit 1; }
step() { printf '\n== %s\n' "$*"; }

[ -f "$notes_file" ] || die "no notes file: $notes_file"
[ -f "$KEY_FILE" ] || die "no updater key: $KEY_FILE"
conf_version=$(python3 -c 'import json; print(json.load(open("src-tauri/tauri.conf.json"))["version"])')
[ "$conf_version" = "$version" ] || die "tauri.conf.json says $conf_version, not $version"
if [ -z "$dry" ]; then
  [ "$(git branch --show-current)" = main ] || die "not on main"
  [ -z "$(git status --porcelain)" ] || die "working tree is not clean"
  ! git rev-parse -q --verify "refs/tags/$tag" >/dev/null || die "tag $tag exists"
fi

TAURI_SIGNING_PRIVATE_KEY=$(cat "$KEY_FILE")
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$(security find-generic-password -s "$KEYCHAIN_ITEM" -w) ||
  die "no Keychain item $KEYCHAIN_ITEM"
export TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD

step "macOS: DMG and update bundle"
# The app bundle is listed too: with only "dmg", the bundler deletes the .app it built for
# the DMG, and the update bundle (.app.tar.gz) is made from that .app.
CARGO_TARGET_DIR="$PWD/target-dmg" pnpm tauri build --bundles app,dmg
mac_dir="target-dmg/release/bundle"
dmg="$mac_dir/dmg/ZoomSlidesPicker_${version}_aarch64.dmg"
mac_tar="$mac_dir/macos/ZoomSlidesPicker.app.tar.gz"
[ -f "$dmg" ] && [ -f "$mac_tar" ] && [ -f "$mac_tar.sig" ] || die "macOS outputs missing"

step "Windows: installer and its signature"
XWIN_CACHE_DIR="$HOME/Library/Caches/cargo-xwin" PATH="/opt/homebrew/opt/llvm/bin:$PATH" \
  pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc
win_exe="target/x86_64-pc-windows-msvc/release/bundle/nsis/ZoomSlidesPicker_${version}_x64-setup.exe"
[ -f "$win_exe" ] && [ -f "$win_exe.sig" ] || die "Windows outputs missing"

step "latest.json"
out="target/release-$version"
mkdir -p "$out"
cp "$dmg" "$out/"
cp "$mac_tar" "$out/ZoomSlidesPicker_${version}_aarch64.app.tar.gz"
cp "$win_exe" "$out/"
base="https://github.com/$PUBLIC_REPO/releases/download/$tag"
python3 - "$version" "$notes_file" "$base" "$mac_tar.sig" "$win_exe.sig" "$out/latest.json" <<'PY'
import json, sys
from datetime import datetime, timezone
version, notes, base, mac_sig, win_sig, dest = sys.argv[1:]
manifest = {
    "version": version,
    "notes": open(notes, encoding="utf-8").read().strip(),
    "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {
        "darwin-aarch64": {
            "signature": open(mac_sig).read().strip(),
            "url": f"{base}/ZoomSlidesPicker_{version}_aarch64.app.tar.gz",
        },
        "windows-x86_64": {
            "signature": open(win_sig).read().strip(),
            "url": f"{base}/ZoomSlidesPicker_{version}_x64-setup.exe",
        },
    },
}
with open(dest, "w", encoding="utf-8") as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
PY
ls -la "$out"

if [ -n "$dry" ]; then
  printf '\ndry run: built into %s; nothing tagged or uploaded\n' "$out"
  exit 0
fi

step "private repository: tag, push, release"
git tag -a "$tag" -m "ZoomSlidesPicker $version"
git push origin main
git push origin "$tag"
gh release create "$tag" "$out/$(basename "$dmg")" "$out/$(basename "$win_exe")" \
  --title "ZoomSlidesPicker $version" --notes-file "$notes_file"

step "public repository: release the app polls"
gh release create "$tag" --repo "$PUBLIC_REPO" --target main \
  "$out/$(basename "$dmg")" "$out/$(basename "$win_exe")" \
  "$out/ZoomSlidesPicker_${version}_aarch64.app.tar.gz" "$out/latest.json" \
  --title "ZoomSlidesPicker $version" --notes-file "$notes_file"

step "check the endpoint"
served=$(curl -fsSL "https://github.com/$PUBLIC_REPO/releases/latest/download/latest.json" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["version"])')
[ "$served" = "$version" ] || die "the endpoint serves $served, not $version"
printf '\nreleased %s\n' "$version"
