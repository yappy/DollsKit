#!/bin/sh -eu

echo Usage  : "$0" ARGS...
echo Example: "$0" -v /host/path:/container/path

set -x
docker run --rm -it "$@" busybox
