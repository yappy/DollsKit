#!/bin/sh -eu

SELF_DIR=$(dirname "$(realpath "$0")")
WSTUNNEL_BIN="${SELF_DIR}/wstunnel/target/release/wstunnel"
WSTUNNEL_INSTALL_PATH="${1:-/opt/wstunnel/wstunnel}"

if [ ! -x "$WSTUNNEL_BIN" ]; then
    echo "Built wstunnel binary not found: $WSTUNNEL_BIN" >&2
    exit 1
fi

install -D -m 0755 "$WSTUNNEL_BIN" "$WSTUNNEL_INSTALL_PATH"
echo "Installed: $WSTUNNEL_INSTALL_PATH"
