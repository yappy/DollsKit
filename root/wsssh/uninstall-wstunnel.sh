#!/bin/sh -eu

WSTUNNEL_INSTALL_PATH="${1:-/opt/wstunnel/wstunnel}"

if [ -e "$WSTUNNEL_INSTALL_PATH" ] || [ -L "$WSTUNNEL_INSTALL_PATH" ]; then
    rm -f "$WSTUNNEL_INSTALL_PATH"
    echo "Removed wstunnel binary: $WSTUNNEL_INSTALL_PATH"
fi

WSTUNNEL_INSTALL_DIR=$(dirname "$WSTUNNEL_INSTALL_PATH")
if rmdir "$WSTUNNEL_INSTALL_DIR" 2>/dev/null; then
    echo "Removed empty directory: $WSTUNNEL_INSTALL_DIR"
fi

echo "Removed the wstunnel binary. systemd configuration was left unchanged."
