#!/bin/bash
set -euox pipefail

NANO_TEST=ON \
NANO_NETWORK=dev \
$(dirname "$BASH_SOURCE")/build.sh all_tests