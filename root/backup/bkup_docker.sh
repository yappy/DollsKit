#!/bin/bash -eu

echo Usage: "$0" PROJ VOLUMES...

# ------------------------------------------------------------------------------
# Config (can be overriden by ENVVAR)
BKUP_MP=${BKUP_MP:-"/mnt/bkup"}
KEEP_DAYS=${KEEP_DAYS:-"180"}
EXT=${EXT:-"tar.bz2"}
# ------------------------------------------------------------------------------

#SELF_DIR=$(dirname "$(realpath "$0")")
BKUP_ROOT="${BKUP_MP}/docker"
PROJ=$1
VOLUMES=( "${@:2}" )
DSTDIR="${BKUP_ROOT}/${PROJ}"

# check if mount point is available
mountpoint "${BKUP_MP}"
mkdir -p "${DSTDIR}"

echo --------------------------------------------------------------------------------
echo START
date -R
echo --------------------------------------------------------------------------------

# cleanup
echo Delete files older than "${KEEP_DAYS}" days in "${DSTDIR}"
find "${DSTDIR}" -type f -mtime "+${KEEP_DAYS}" -exec rm {} \;

# tar
for VOL in "${VOLUMES[@]}"; do
    FILE="${PROJ}_${VOL}_$(date +%Y%m%d%H%M ).${EXT}"
    set -x
    docker run --rm -v "${PROJ}_${VOL}:/mnt/vol" -v "${DSTDIR}:/mnt/bkup" \
        busybox tar caf "/mnt/bkup/${FILE}" -C /mnt/vol . &
    set +x
done
wait

echo --------------------------------------------------------------------------------
echo END
date -R
echo --------------------------------------------------------------------------------
echo ""
