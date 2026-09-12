#!/bin/sh -ue

ROOT_DIR=$(dirname "$(realpath "$0")")/..

find "${ROOT_DIR}" -type f -name "*.sh" -print0 | xargs -0 -I{} sh -c "echo shellcheck {}; shellcheck {}"
find "${ROOT_DIR}/root/cron" -type f -not -name "*.*" -print0 | xargs -0 -I{} sh -c "echo shellcheck {}; shellcheck {}"
