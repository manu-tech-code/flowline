#!/bin/bash
# Install the on-device transcription engine Flowline uses by default.
#
#   scripts/install-whisper.sh            # ggml-small (multilingual, ~466 MB)
#   scripts/install-whisper.sh base       # smaller, faster, less accurate
#   scripts/install-whisper.sh large-v3-turbo
set -euo pipefail

MODEL="${1:-small}"
MODELS_DIR="$HOME/Library/Application Support/Flowline/models"

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required for the one-time whisper.cpp install." >&2
  echo "Install it from https://brew.sh and run this script again." >&2
  exit 1
fi

if ! brew list whisper.cpp >/dev/null 2>&1; then
  brew install whisper.cpp
fi

mkdir -p "$MODELS_DIR"
DEST="$MODELS_DIR/ggml-$MODEL.bin"
if [ ! -f "$DEST" ]; then
  echo "Downloading ggml-$MODEL.bin (~$(case "$MODEL" in
    small) echo 466 ;;
    base*) echo 142 ;;
    *) echo 800 ;;
  esac) MB)…"
  curl -L --fail --progress-bar -o "$DEST" \
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-$MODEL.bin"
fi

VAD="$MODELS_DIR/ggml-silero-v6.2.0.bin"
if [ ! -f "$VAD" ]; then
  echo "Downloading the Silero VAD model (864 KB)…"
  curl -L --fail --progress-bar -o "$VAD" \
    "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin"
fi

echo "Ready: $DEST"
echo "Flowline picks it up automatically under Settings → Transcription engine."
