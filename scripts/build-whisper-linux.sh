#!/bin/bash
# Build the speech engine (whisper.cpp's whisper-cli) for Linux as a standalone,
# statically linked binary: vendor/whisper/whisper-linux-<arch>.
#
#   scripts/linux-docker.sh whisper     # inside the Docker build image
#   scripts/build-whisper-linux.sh      # directly on a Linux machine
#
# Needs: git, cmake, a C++ compiler, network access to github.com for the source.
set -euo pipefail
cd "$(dirname "$0")/.."

REF="${WHISPER_REF:-master}"
ARCH=$(uname -m)
WORK="vendor/whisper/build-linux-$ARCH"
OUT="vendor/whisper/whisper-linux-$ARCH"

mkdir -p vendor/whisper
if [ ! -d "$WORK/src/.git" ]; then
  rm -rf "$WORK/src"
  mkdir -p "$WORK"
  git clone --depth 1 --branch "$REF" https://github.com/ggml-org/whisper.cpp "$WORK/src"
fi
echo "whisper.cpp $(git -C "$WORK/src" describe --tags --always)"

# Keep the builder's paths out of the binary.
MAP="-ffile-prefix-map=$(pwd)=/flowline -ffile-prefix-map=$HOME=/home/builder"
EXTRA=()
if [ "$ARCH" = "x86_64" ]; then
  # A portable CPU baseline (AVX2 + FMA, 2013 and later) rather than this
  # machine's exact features.
  EXTRA+=(-DGGML_AVX=ON -DGGML_AVX2=ON -DGGML_FMA=ON -DGGML_F16C=ON -DGGML_AVX512=OFF)
fi
cmake -S "$WORK/src" -B "$WORK/build" \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=OFF \
  -DGGML_NATIVE=OFF \
  -DGGML_OPENMP=OFF \
  -DCMAKE_C_FLAGS="$MAP" -DCMAKE_CXX_FLAGS="$MAP" \
  -DCMAKE_EXE_LINKER_FLAGS="-static-libstdc++ -static-libgcc" \
  -DWHISPER_BUILD_EXAMPLES=ON -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF \
  "${EXTRA[@]}"
cmake --build "$WORK/build" --config Release -j "$(nproc)" --target whisper-cli
cp "$WORK/build/bin/whisper-cli" "$OUT"
strip "$OUT" 2>/dev/null || true
chmod +x "$OUT"
echo "built $OUT"

# It must run on a stock system: only the C library and friends may be linked.
echo "links:"
ldd "$OUT" || true
if ldd "$OUT" | grep -q "libggml\|libwhisper\|libgomp\|libstdc++\|not found"; then
  echo "error: $OUT depends on libraries that will not be on other machines" >&2
  exit 1
fi
# ...and it must not carry the builder's user name or home folder.
BUILDER="$(id -un)"
if strings -a "$OUT" | grep -q "/home/$BUILDER\|/Users/"; then
  echo "error: $OUT still contains the builder's paths:" >&2
  strings -a "$OUT" | grep "/home/$BUILDER\|/Users/" | head -3 >&2
  exit 1
fi
echo "ok: self-contained and clean"
