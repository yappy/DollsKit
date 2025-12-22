#!/bin/sh -ue

SELF_DIR=$(dirname "$(realpath "$0")")

find "${SELF_DIR}" -type f -name "*.sh" -print0 | xargs -0 -I{} sh -c "echo shellcheck {}; shellcheck {}"
find "${SELF_DIR}/root/cron" -type f -not -name "*.*" -print0 | xargs -0 -I{} sh -c "echo shellcheck {}; shellcheck {}"
