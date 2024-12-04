#!/usr/bin/env bash

build_dir=${1-${PWD}}
TIMEOUT_DEFAULT=1800

BUSYBOX_BASH=${BUSYBOX_BASH-0}

if [[ ${FLAVOR-_} == "_" ]]; then
    FLAVOR=""
fi

if [[ "$OSTYPE" == "darwin"* ]]; then
    TIMEOUT_CMD=gtimeout
else
    TIMEOUT_CMD=timeout
fi

set -o nounset
set -o xtrace

run_tests()
{
    # when busybox pretends to be bash it needs different args for the timeout builtin
    #
    if [[ "${BUSYBOX_BASH}" -eq 1 ]]; then
        TIMEOUT_TIME_ARG="-t"
    else
        TIMEOUT_TIME_ARG=""
    fi

    ${TIMEOUT_CMD} ${TIMEOUT_TIME_ARG} ${TIMEOUT_DEFAULT} RUST_BACKTRACE=1 ./core_test
    core_test_res=${?}

	pushd ../rust
    ${TIMEOUT_CMD} ${TIMEOUT_TIME_ARG} ${TIMEOUT_DEFAULT} RUST_BACKTRACE=1 ~/.cargo/bin/cargo test -q
    cargo_test_res=${?}
	popd

    echo "Core Test return code: ${core_test_res}"
    echo "Cargo Test return code: ${cargo_test_res}"

    if [[ ${core_test_res} != 0 || ${cargo_test_res} != 0 ]]; then
        return 1
    else
        return 0
    fi
}

cd ${build_dir}
run_tests
