#!/bin/sh
rust_code=`cloc --include-lang=Rust --exclude-dir=CMakeFiles,target --json rust | jq '.Rust.code'`
sum_code=`cloc --include-lang=Rust,C,C++,"C/C++ Header" --exclude-dir=CMakeFiles,target --json nano rust | jq '.SUM.code'`
perc=`echo "scale=2;$rust_code*100/$sum_code" |bc`
cpp_code=`echo "$sum_code-$rust_code" |bc`
echo "cpp  : $cpp_code"
echo "rust : $rust_code"
echo "%rust: $perc"

