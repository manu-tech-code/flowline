#!/bin/bash
# Build Flowline and wrap it in a proper .app bundle.
#
#   scripts/bundle.sh              # host architecture, ad-hoc signed (development)
#   scripts/bundle.sh --universal  # arm64 + x86_64, runs on Apple Silicon and Intel
#   scripts/bundle.sh --install    # also copy it to /Applications
#
# Environment:
#   CODESIGN_IDENTITY  a "Developer ID Application: …" identity. Without it the
#                      app is ad-hoc signed, which works on this Mac but makes
#                      Gatekeeper warn on other Macs (see README).
#   WHISPER_BIN        a whisper-cli to ship inside the app. Default:
#                      vendor/whisper/whisper-cli (made by scripts/build-whisper.sh).
set -euo pipefail
cd "$(dirname "$0")/.."

UNIVERSAL=0
INSTALL=0
for arg in "$@"; do
  case "$arg" in
    --universal) UNIVERSAL=1 ;;
    --install) INSTALL=1 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP="dist/Flowline.app"
# Run on the macOS release Flowline declares support for, not just this Mac's.
export MACOSX_DEPLOYMENT_TARGET="11.0"

# Keep the builder's home folder and user name out of the shipped binary: Rust
# embeds source paths in its panic messages, so rewrite them to neutral ones.
# (Most specific prefix last, so the project folder wins over $HOME.)
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=/home/builder --remap-path-prefix=$(pwd)=/flowline"

CARGO="${CARGO:-cargo}"
if ! command -v "$CARGO" >/dev/null 2>&1; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if [ "$UNIVERSAL" = 1 ]; then
  for target in aarch64-apple-darwin x86_64-apple-darwin; do
    if ! rustup target list --installed 2>/dev/null | grep -qx "$target"; then
      echo "Missing Rust target $target. Run: rustup target add $target" >&2
      exit 1
    fi
    "$CARGO" build --release --target "$target"
  done
  BIN="target/flowline-universal"
  lipo -create -output "$BIN" \
    target/aarch64-apple-darwin/release/flowline \
    target/x86_64-apple-darwin/release/flowline
else
  "$CARGO" build --release
  BIN="target/release/flowline"
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/bin"
cp "$BIN" "$APP/Contents/MacOS/flowline"
if [ -f dist/Flowline.icns ]; then
  cp dist/Flowline.icns "$APP/Contents/Resources/Flowline.icns"
fi

# The on-device engine travels inside the app so nothing else has to be
# installed. Without it, Flowline falls back to a Homebrew whisper-cli.
WHISPER="${WHISPER_BIN:-vendor/whisper/whisper-cli}"
if [ -f "$WHISPER" ]; then
  cp "$WHISPER" "$APP/Contents/Resources/bin/whisper-cli"
  chmod +x "$APP/Contents/Resources/bin/whisper-cli"
  ARCHS=$(lipo -archs "$APP/Contents/Resources/bin/whisper-cli" 2>/dev/null || true)
  echo "bundled engine: $WHISPER ($ARCHS)"
  if [ "$UNIVERSAL" = 1 ] && [ "$ARCHS" != "x86_64 arm64" ] && [ "$ARCHS" != "arm64 x86_64" ]; then
    echo "warning: the bundled engine is not universal ($ARCHS); the other kind of Mac will have no on-device engine" >&2
  fi
else
  rmdir "$APP/Contents/Resources/bin" 2>/dev/null || true
  echo "warning: no whisper-cli to bundle. People who install this build will need Homebrew's whisper-cpp. Run scripts/build-whisper.sh first for a self-contained app." >&2
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Flowline</string>
  <key>CFBundleDisplayName</key><string>Flowline</string>
  <key>CFBundleExecutable</key><string>flowline</string>
  <key>CFBundleIdentifier</key><string>com.flowline.app</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleIconFile</key><string>Flowline</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key>
  <string>Flowline listens only while you hold the dictation shortcut, so it can turn your speech into text on this Mac.</string>
  <key>NSAppleEventsUsageDescription</key>
  <string>Flowline uses system audio controls to mute other audio while dictating.</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
</dict>
</plist>
PLIST

IDENTITY="${CODESIGN_IDENTITY:--}"
if [ "$IDENTITY" = "-" ]; then
  # Ad-hoc: enough for macOS to keep one identity for permissions on this Mac.
  codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
  echo "signed ad-hoc (set CODESIGN_IDENTITY for a Developer ID build)"
else
  # Hardened runtime is required for notarization. Sign the engine first.
  if [ -f "$APP/Contents/Resources/bin/whisper-cli" ]; then
    codesign --force --options runtime --timestamp --sign "$IDENTITY" \
      "$APP/Contents/Resources/bin/whisper-cli"
  fi
  codesign --force --options runtime --timestamp \
    --entitlements scripts/Flowline.entitlements --sign "$IDENTITY" "$APP"
  codesign --verify --deep --strict "$APP"
  echo "signed with: $IDENTITY"
fi
echo "built $APP ($(lipo -archs "$APP/Contents/MacOS/flowline"))"

if [ "$INSTALL" = 1 ]; then
  rm -rf "/Applications/Flowline.app"
  cp -R "$APP" "/Applications/Flowline.app"
  echo "installed /Applications/Flowline.app"
fi
