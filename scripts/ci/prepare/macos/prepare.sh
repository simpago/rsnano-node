#!/bin/bash
set -euox pipefail

#Homebrew randomly fails to update. Retry 5 times with 15s interval
for i in {1..5}; do brew update && break || { echo "Update failed, retrying..."; sleep 15; }; done

brew install coreutils

