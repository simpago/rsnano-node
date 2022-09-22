#!/bin/sh
cloc --include-lang=Rust,C,C++,"C/C++ Header" --exclude-dir=CMakeFiles nano rust

