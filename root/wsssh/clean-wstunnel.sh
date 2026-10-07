#!/bin/sh -eu

SELF_DIR=$(dirname "$(realpath "$0")")

git -C "$SELF_DIR" clean -ffdx -- wstunnel/
