#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# shellcheck source=/dev/null
source "$SCRIPT_DIR/docker-compose-env.sh"

docker compose ps
printf 'TLSN verifier TCP: localhost:%s\n' "$TLSN_CURL_TLSN_TCP_PORT"
