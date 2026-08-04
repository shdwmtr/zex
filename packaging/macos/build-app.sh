#!/bin/sh
# Assembles target/optimized-release/zex into a proper Zex.app bundle at
# target/macos/Zex.app. Run via `make app` from the repo root.
set -eu

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PROFILE="optimized-release"
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
    BIN="target/$CARGO_BUILD_TARGET/$PROFILE/zex"
else
    BIN="target/$PROFILE/zex"
fi
APP_DIR="target/macos/Zex.app"
CONTENTS="$APP_DIR/Contents"
ICON_SRC="assets/icon.svg"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

if [ ! -f "$BIN" ]; then
    if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
        echo "error: $BIN not found; run 'cargo build --profile $PROFILE --target $CARGO_BUILD_TARGET' first" >&2
    else
        echo "error: $BIN not found; run 'cargo build --profile $PROFILE' first" >&2
    fi
    exit 1
fi

if ! command -v rsvg-convert >/dev/null 2>&1; then
    echo "error: rsvg-convert not found; run 'brew install librsvg' first" >&2
    exit 1
fi

echo "==> Assembling $APP_DIR (version $VERSION)"
rm -rf "$APP_DIR"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

cp "$BIN" "$CONTENTS/MacOS/zex"

sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$CONTENTS/Info.plist"

# Build zex.icns from the source SVG. iconutil wants a *.iconset directory of
# exact power-of-two sizes; rsvg-convert rasterizes each straight from the
# vector source.
ICONSET=$(mktemp -d)/zex.iconset
mkdir -p "$ICONSET"
for sz in 16 32 128 256 512; do
    rsvg-convert -w "$sz" -h "$sz" "$ICON_SRC" -o "$ICONSET/icon_${sz}x${sz}.png"
    dbl=$((sz * 2))
    rsvg-convert -w "$dbl" -h "$dbl" "$ICON_SRC" -o "$ICONSET/icon_${sz}x${sz}@2x.png"
done
iconutil -c icns "$ICONSET" -o "$CONTENTS/Resources/zex.icns"
rm -rf "$(dirname "$ICONSET")"

# Ad-hoc sign so Gatekeeper/TCC (Full Disk Access, etc.) can identify the app
# by a stable identity instead of refusing it outright. Not a substitute for
# a real Developer ID signature if you plan to distribute this beyond your
# own machine.
codesign --force --deep --sign - "$APP_DIR" 2>&1 | grep -v "^replacing existing signature" || true

echo "==> Built $APP_DIR"
