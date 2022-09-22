#!/bin/bash

sudo mkdir -p /etc/docker && echo '{"ipv6":true,"fixed-cidr-v6":"2001:db8:1::/64"}' | sudo tee /etc/docker/daemon.json && sudo service docker restart

ci/build-docker-image.sh docker/ci/Dockerfile-base simpago/rsnano-env:base
if [[ "${COMPILER:-}" != "" ]]; then
    ci/build-docker-image.sh docker/ci/Dockerfile-gcc simpago/rsnano-env:${COMPILER}
else
    ci/build-docker-image.sh docker/ci/Dockerfile-gcc simpago/rsnano-env:gcc
    ci/build-docker-image.sh docker/ci/Dockerfile-clang-6 simpago/rsnano-env:clang-6
    ci/build-docker-image.sh docker/ci/Dockerfile-centos simpago/rsnano-env:centos
fi
