#!/bin/bash -eu

# ------------------------------------------------------------------------------
# Config (can be overriden by ENVVAR)
BKUP_MP=${BKUP_MP:-"/mnt/bkup"}
KEEP_COUNT=${KEEP_COUNT:-"30"}
# ------------------------------------------------------------------------------

SELF_DIR=$(dirname "$(realpath "$0")")
SCRIPT_DIR="${SELF_DIR}/bkup/src"

BKUP_ROOT="${BKUP_MP}/docker"
PROJS=(growi-public growi-private)
VOLUME_ARGS=(-v es_data -v growi_data -v mongo_configdb -v mongo_db -v page_bulk_export_tmp)

# check if mount point is available
mountpoint "${BKUP_MP}"

echo --------------------------------------------------------------------------------
echo START
date -R
echo --------------------------------------------------------------------------------

for PROJ in "${PROJS[@]}" ; do
    ARCHIVE_DIR="${BKUP_ROOT}/${PROJ}"

    python3 "${SCRIPT_DIR}/bkup.py" \
    dockervol \
    --project "${PROJ}" \
    --dst "${ARCHIVE_DIR}/${PROJ}" \
    "${VOLUME_ARGS[@]}"

    python3 "${SCRIPT_DIR}/bkup.py" \
    clean \
    --dst "${ARCHIVE_DIR}" \
    --keep-count "${KEEP_COUNT}"

done

echo --------------------------------------------------------------------------------
echo END
date -R
echo --------------------------------------------------------------------------------
echo ""
