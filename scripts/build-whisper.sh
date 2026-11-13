#!/bin/bash
# Build the on-device speech engine (whisper.cpp's whisper-cli) as a standalone,
# statically linked binary that Flowline ships inside the app, so people who
# download Flowline do not need Homebrew.
#
#   scripts/build-whisper.sh          # → vendor/whisper/whisper-cli (arm64 + x86_64)
#   WHISPER_REF=v1.8.2 scripts/build-whisper.sh     # pin a whisper.cpp tag or commit
#   ARCHS="arm64" scripts/build-whisper.sh          # one architecture only
#
# Needs: git, cmake (brew install cmake), Xcode command line tools, network access
# to github.com for the source.
set -euo pipefail
cd "$(dirname "$0")/.."

REF="${WHISPER_REF:-master}"
ARCHS="${ARCHS:-arm64 x86_64}"
WORK="vendor/whisper/build"
OUT="vendor/whisper/whisper-cli"

for tool in git cmake lipo; do
  command -v "$tool" >/dev/null 2>&1 || { echo "missing tool: $tool" >&2; exit 1; }
done

mkdir -p vendor/whisper
if [ ! -d "$WORK/src/.git" ]; then
  rm -rf "$WORK/src"
  mkdir -p "$WORK"
  git clone --depth 1 --branch "$REF" https://github.com/ggml-org/whisper.cpp "$WORK/src"
fi
echo "whisper.cpp $(git -C "$WORK/src" describe --tags --always)"

# Keep the builder's home folder out of the binary: C/C++ assert messages embed
# source paths, so rewrite them to neutral ones.
MAP="-ffile-prefix-map=$(pwd)=/flowline -ffile-prefix-map=$HOME=/home/builder"

SLICES=()
for arch in $ARCHS; do
  BUILD="$WORK/build-$arch"
  EXTRA=()
  if [ "$arch" = "arm64" ]; then
    # Apple Silicon: Metal GPU with the shaders embedded in the binary.
    EXTRA+=(-DGGML_METAL=ON -DGGML_METAL_EMBED_LIBRARY=ON)
  else
    # Intel: a conservative CPU baseline so it runs on Macs without AVX2.
    EXTRA+=(-DGGML_METAL=OFF -DGGML_AVX=OFF -DGGML_AVX2=OFF -DGGML_FMA=OFF -DGGML_F16C=OFF)
  fi
  cmake -S "$WORK/src" -B "$BUILD" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_OSX_ARCHITECTURES="$arch" \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0 \
    -DCMAKE_C_FLAGS="$MAP" -DCMAKE_CXX_FLAGS="$MAP" \
    -DBUILD_SHARED_LIBS=OFF \
    -DGGML_NATIVE=OFF \
    -DWHISPER_BUILD_EXAMPLES=ON -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF \
    "${EXTRA[@]}"
  cmake --build "$BUILD" --config Release -j "$(sysctl -n hw.ncpu)" --target whisper-cli
  cp "$BUILD/bin/whisper-cli" "$BUILD/whisper-cli-$arch"
  SLICES+=("$BUILD/whisper-cli-$arch")
done

lipo -create -output "$OUT" "${SLICES[@]}"
strip -x "$OUT" 2>/dev/null || true
chmod +x "$OUT"
echo "built $OUT ($(lipo -archs "$OUT"))"
# The shipped engine must not carry the builder's user name.
if strings -a "$OUT" | grep -q "/Users/\|$USER"; then
  echo "error: $OUT still contains the builder's paths:" >&2
  strings -a "$OUT" | grep "/Users/\|$USER" | head -3 >&2
  exit 1
fi
# A static build must not depend on Homebrew's libraries.
if otool -L "$OUT" | grep -q "/opt/homebrew\|/usr/local"; then
  echo "error: $OUT still links Homebrew libraries:" >&2
  otool -L "$OUT" >&2
  exit 1
fi
