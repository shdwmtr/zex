#!/bin/sh
# Assembles target/optimized-release/zex into a proper Zex.app bundle at
# target/macos/Zex.app. Run via `make app` from the repo root.
set -eu

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PROFILE="optimized-release"
BIN="target/$PROFILE/zex"
APP_DIR="target/macos/Zex.app"
CONTENTS="$APP_DIR/Contents"
ICON_SRC="assets/icon.png"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

if [ ! -f "$BIN" ]; then
    echo "error: $BIN not found; run 'cargo build --profile $PROFILE' first" >&2
    exit 1
fi

echo "==> Assembling $APP_DIR (version $VERSION)"
rm -rf "$APP_DIR"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

cp "$BIN" "$CONTENTS/MacOS/zex"

sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$CONTENTS/Info.plist"

# Build zex.icns from the source PNG. iconutil wants a *.iconset directory of
# exact power-of-two sizes; sips generates each from the 1080x1080 source.
ICONSET=$(mktemp -d)/zex.iconset
mkdir -p "$ICONSET"
for sz in 16 32 128 256 512; do
    sips -z "$sz" "$sz" "$ICON_SRC" --out "$ICONSET/icon_${sz}x${sz}.png" >/dev/null
    dbl=$((sz * 2))
    sips -z "$dbl" "$dbl" "$ICON_SRC" --out "$ICONSET/icon_${sz}x${sz}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$CONTENTS/Resources/zex.icns"
rm -rf "$(dirname "$ICONSET")"

# Ad-hoc sign so Gatekeeper/TCC (Full Disk Access, etc.) can identify the app
# by a stable identity instead of refusing it outright. Not a substitute for
# a real Developer ID signature if you plan to distribute this beyond your
# own machine.
codesign --force --deep --sign - "$APP_DIR" 2>&1 | grep -v "^replacing existing signature" || true

echo "==> Built $APP_DIR"
