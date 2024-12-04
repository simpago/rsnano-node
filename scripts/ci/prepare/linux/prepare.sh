#!/bin/bash
set -euox pipefail

COMPILER=${COMPILER:-gcc}

echo "Compiler: '${COMPILER}'"

# Common dependencies needed for building & testing
DEBIAN_FRONTEND=noninteractive apt-get update -qq

DEBIAN_FRONTEND=noninteractive apt-get install -yqq \
build-essential \
curl \
wget \
python3 \
git \
libssl-dev \
pkg-config

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y

