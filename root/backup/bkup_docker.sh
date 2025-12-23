#!/bin/bash -eu

echo Usage: "$0" PROJ VOLUMES...

# ------------------------------------------------------------------------------
# Config (can be overriden by ENVVAR)
BKUP_MP=${BKUP_MP:-"/mnt/bkup"}
KEEP_COUNT=${KEEP_COUNT:-"30"}
EXT=${EXT:-".tar.bz2"}
# ------------------------------------------------------------------------------

#SELF_DIR=$(dirname "$(realpath "$0")")
BKUP_ROOT="${BKUP_MP}/docker"
PROJ=$1
VOLUMES=( "${@:2}" )

# check if mount point is available
mountpoint "${BKUP_MP}"

echo --------------------------------------------------------------------------------
echo START
date -R
echo --------------------------------------------------------------------------------

for VOL in "${VOLUMES[@]}"; do
    VOL_FULL="${PROJ}_${VOL}"
    FILE="${VOL_FULL}_$(date +%Y%m%d).${EXT}"
    docker run --rm -it -v "${VOL_FULL}:/mnt/vol" -v "${BKUP_ROOT}:/mnt/bkup" \
        busybox tar caf "/mnt/bkup/${FILE}" -C /mnt/vol . &
done
wait

echo --------------------------------------------------------------------------------
echo END
date -R
echo --------------------------------------------------------------------------------
echo ""
