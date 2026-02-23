#!/bin/bash -eu

# ------------------------------------------------------------------------------
# Config (can be overriden by ENVVAR)
BKUP_MP=${BKUP_MP:-"/mnt/bkup"}
SRC_DIR=${SRC_DIR:-"/"}
KEEP_COUNT=${KEEP_COUNT:-"30"}
# Cloud upload (rclone) settings: set RCLONE_REMOTE to enable upload
RCLONE_REMOTE=${RCLONE_REMOTE:-""}
RCLONE_DST=${RCLONE_DST:-""}
# ------------------------------------------------------------------------------

SELF_DIR=$(dirname "$(realpath "$0")")
SCRIPT_DIR=${SELF_DIR}/bkup/src

BKUP_ROOT="${BKUP_MP}/full"
SYNC_DIR=${BKUP_ROOT}/sync
ARCHIVE_DIR=${BKUP_ROOT}/archive

# check if mount point is available
mountpoint "${BKUP_MP}"

echo --------------------------------------------------------------------------------
echo START
date -R
echo --------------------------------------------------------------------------------

python3 "${SCRIPT_DIR}/bkup.py" \
sync \
--src "${SRC_DIR}" \
--dst "${SYNC_DIR}" \
--exclude-from "${SELF_DIR}/bkup_exclude.txt" \
--force

python3 "${SCRIPT_DIR}/bkup.py" \
archive \
--src "${SYNC_DIR}" \
--dst "${ARCHIVE_DIR}"

python3 "${SCRIPT_DIR}/bkup.py" \
clean \
--dst "${ARCHIVE_DIR}" \
--keep-count "${KEEP_COUNT}"

# Upload to cloud if RCLONE_REMOTE is not empty
if [ -n "${RCLONE_REMOTE:-}" ]; then
    python3 "${SCRIPT_DIR}/bkup.py" \
    cloud \
    --src "${ARCHIVE_DIR}" \
    --remote "${RCLONE_REMOTE}" \
    --dst "${RCLONE_DST}"

    python3 "${SCRIPT_DIR}/bkup.py" \
    cloudclean \
    --src "${ARCHIVE_DIR}" \
    --remote "${RCLONE_REMOTE}" \
    --dst "${RCLONE_DST}"
fi

echo END
date -R
echo --------------------------------------------------------------------------------
echo ""
