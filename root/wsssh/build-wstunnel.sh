#!/bin/bash -eu

SELF_DIR=$(dirname "$(realpath "$0")")
WSTUNNEL_DIR="${SELF_DIR}/wstunnel"
WSTUNNEL_TAG="v11.0.0"

if [ -e "$WSTUNNEL_DIR" ]; then
    # if $WSTUNNEL_DIR exists, check HEAD = $WSTUNNEL_TAG
    if [ "$(git -C "$WSTUNNEL_DIR" rev-parse --show-toplevel 2>/dev/null || true)" != "$WSTUNNEL_DIR" ]; then
        echo "Not a Git checkout: $WSTUNNEL_DIR" >&2
        exit 1
    fi
    TAG_COMMIT=$(git -C "$WSTUNNEL_DIR" rev-parse -q --verify "refs/tags/$WSTUNNEL_TAG^{commit}" 2>/dev/null || true)
    if [ -z "$TAG_COMMIT" ] || [ "$(git -C "$WSTUNNEL_DIR" rev-parse HEAD)" != "$TAG_COMMIT" ]; then
        echo "Not at tag $WSTUNNEL_TAG: $WSTUNNEL_DIR" >&2
        exit 1
    fi
    if ! git -C "$WSTUNNEL_DIR" diff --quiet ||
       ! git -C "$WSTUNNEL_DIR" diff --staged --quiet ||
       [ -n "$(git -C "$WSTUNNEL_DIR" ls-files --others --exclude-standard)" ]; then
        echo "Git checkout has uncommitted changes: $WSTUNNEL_DIR" >&2
        exit 1
    fi
else
    # elsewhere, newly git clone
    git clone --branch "$WSTUNNEL_TAG" https://github.com/erebe/wstunnel.git "$WSTUNNEL_DIR"
fi

pushd "$WSTUNNEL_DIR"
cargo build --release
popd

echo "Binary file: $WSTUNNEL_DIR/target/release/wstunnel"
