#!/bin/bash
# Run Flowline's Linux build, tests and packaging inside Docker, from any host.
#
#   scripts/linux-docker.sh image          # build the build image (once)
#   scripts/linux-docker.sh check          # cargo check
#   scripts/linux-docker.sh test           # cargo test
#   scripts/linux-docker.sh build          # cargo build --release
#   scripts/linux-docker.sh whisper        # build the bundled speech engine (once)
#   scripts/linux-docker.sh package        # dist/flowline-<version>-linux-<arch>.tar.gz
#   scripts/linux-docker.sh shell          # a shell in the build environment
#
# Cargo's download cache and the Linux build output live in Docker volumes, so
# repeat runs are fast and nothing is written into your source tree's target/.
set -euo pipefail
cd "$(dirname "$0")/.."

IMAGE="${FLOWLINE_BUILD_IMAGE:-flowline-linux-build}"
run() {
  docker run --rm \
    -v "$PWD":/src \
    -v flowline-cargo-registry:/usr/local/cargo/registry \
    -v flowline-linux-target:/target \
    -e CARGO_TARGET_DIR=/target \
    "$@"
}

cmd="${1:-}"
shift || true
case "$cmd" in
  image)   docker build -f docker/linux-build.Dockerfile -t "$IMAGE" . ;;
  check)   run "$IMAGE" cargo check --release "$@" ;;
  test)    run "$IMAGE" cargo test --release "$@" ;;
  build)   run "$IMAGE" cargo build --release "$@" ;;
  whisper) run "$IMAGE" scripts/build-whisper-linux.sh "$@" ;;
  package) run -e DIST_DIR=/src/dist "$IMAGE" scripts/linux-package.sh "$@" ;;
  shell)   run -it "$IMAGE" bash ;;
  *) echo "usage: $0 {image|check|test|build|whisper|package|shell}" >&2; exit 2 ;;
esac
