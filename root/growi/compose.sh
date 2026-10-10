#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

exec docker compose \
  --env-file /root/growi/growi.env \
  -f growi-docker-compose/docker-compose.yml \
  -f compose.yaml \
  "$@"
