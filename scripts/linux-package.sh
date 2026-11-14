#!/bin/bash
# Build the Linux release archive. Run it on Linux, or through Docker:
#
#   scripts/linux-docker.sh package
#
# Result: dist/flowline-<version>-linux-<arch>.tar.gz (+ .sha256)
#
# The archive holds the binary, the bundled speech engine (when
# vendor/whisper/whisper-linux-<arch> exists, see scripts/build-whisper-linux.sh),
# the icon and desktop file, the one-time setup script and the licences.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=$(uname -m)                      # x86_64 or aarch64
NAME="flowline-${VERSION}-linux-${ARCH}"
OUT="${CARGO_TARGET_DIR:-target}"
DIST="${DIST_DIR:-dist}"

# Rewrite the builder's home folder out of the binary, as on macOS.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=/home/builder --remap-path-prefix=$(pwd)=/flowline"
cargo build --release

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
PKG="$STAGE/$NAME"
mkdir -p "$PKG/bin"
cp "$OUT/release/flowline" "$PKG/flowline"
strip "$PKG/flowline" 2>/dev/null || true

ENGINE="vendor/whisper/whisper-linux-${ARCH}"
if [ -f "$ENGINE" ]; then
  cp "$ENGINE" "$PKG/bin/whisper-cli"
  chmod +x "$PKG/bin/whisper-cli"
  echo "bundled engine: $ENGINE"
else
  rmdir "$PKG/bin"
  echo "warning: no bundled engine ($ENGINE). Run scripts/build-whisper-linux.sh first, or users need whisper-cli on their PATH." >&2
fi

cp packaging/linux/flowline.desktop packaging/linux/flowline-setup.sh packaging/linux/install.sh packaging/linux/README.txt "$PKG/"
cp dist/icon-source.png "$PKG/flowline.png"
cp LICENSE THIRD-PARTY-NOTICES.md "$PKG/"
chmod +x "$PKG/flowline-setup.sh" "$PKG/install.sh"

mkdir -p "$DIST"
tar -C "$STAGE" -czf "$DIST/$NAME.tar.gz" "$NAME"
(cd "$DIST" && sha256sum "$NAME.tar.gz" | tee "$NAME.tar.gz.sha256")
echo "packaged $DIST/$NAME.tar.gz"
