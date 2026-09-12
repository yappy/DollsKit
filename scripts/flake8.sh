#!/bin/sh -ue

ROOT_DIR=$(dirname "$(realpath "$0")")/..

find "${ROOT_DIR}" -type f -name "*.py" -print0 | xargs -0 -I{} sh -c "echo flake8 {}; flake8 {}"
