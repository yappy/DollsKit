#!/bin/sh -eu

echo Usage: "$0" VOLUME

set -x
FILE="$1.$(date +%Y%m%d_%H%M%S).tar.bz2"
docker run --rm -it -v "$1:/mnt/vol" -v ./ar:/mnt/ar busybox tar cjf "/mnt/ar/$FILE" -C /mnt/vol .
