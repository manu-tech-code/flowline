# Build and test environment for the Linux version of Flowline.
#
#   docker build -f docker/linux-build.Dockerfile -t flowline-linux-build .
#   scripts/linux-docker.sh test        # run the tests on Linux
#   scripts/linux-docker.sh package     # build dist/flowline-<version>-linux-<arch>.tar.gz
#
# Debian 12 (glibc 2.36) is old enough that the result runs on current Ubuntu,
# Fedora, Debian and Arch.
FROM rust:1-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
      cmake build-essential pkg-config git curl ca-certificates \
      libasound2-dev \
      xvfb xauth libgl1-mesa-dri libglx-mesa0 libegl1 libxkbcommon-x11-0 \
      libxcursor1 libxrandr2 libxi6 libwayland-client0 fonts-dejavu-core \
      patchelf file \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
