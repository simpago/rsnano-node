#!/bin/bash

brew update
brew install coreutils
brew install rust
brew install --cask xquartz
brew install boost
util/build_prep/macosx/build_qt.sh
git clone https://github.com/corrosion-rs/corrosion.git
cmake -Scorrosion -Bbuild -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release
sudo cmake --install build --config Release
rm -rf corrosion
rm -rf build
