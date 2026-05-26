#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

USE_EXISTING_COMPOSE=0
if [ "${1:-}" = "--use-existing-compose" ]; then
  USE_EXISTING_COMPOSE=1
fi

# shellcheck source=/dev/null
source "$SCRIPT_DIR/docker-compose-env.sh"

TMP_DIR="$(mktemp -d)"
PROOF_PATH="$TMP_DIR/proof.json"
VERIFY_PATH="$TMP_DIR/verify.json"

cleanup() {
  rm -rf "$TMP_DIR"
  if [ "$USE_EXISTING_COMPOSE" = "0" ]; then
    docker compose down -v --remove-orphans --timeout 10 >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [ "$USE_EXISTING_COMPOSE" = "0" ]; then
  docker compose up -d --build
fi

for _ in $(seq 1 60); do
  if (echo >"/dev/tcp/127.0.0.1/$TLSN_CURL_TLSN_TCP_PORT") >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! (echo >"/dev/tcp/127.0.0.1/$TLSN_CURL_TLSN_TCP_PORT") >/dev/null 2>&1; then
  docker compose logs || true
  echo "TLSN verifier did not become reachable on localhost:$TLSN_CURL_TLSN_TCP_PORT" >&2
  exit 1
fi

"$SCRIPT_DIR/build-cli.sh"

./dist/tlsn-curl "https://api.coinbase.com/v2/prices/BTC-USD/spot" \
  --out "$PROOF_PATH" \
  --verifier "localhost:$TLSN_CURL_TLSN_TCP_PORT" \
  --reveal-response-json /data/amount \
  --max-recv-data 8192

./dist/tlsn-verify "$PROOF_PATH" >"$VERIFY_PATH"

grep -q '"ok": true' "$VERIFY_PATH"
grep -q '"serverName": "api.coinbase.com"' "$VERIFY_PATH"
REVEALED_RECV_LINE="$(grep '"revealedRecv":' "$VERIFY_PATH" || true)"
printf '%s\n' "$REVEALED_RECV_LINE" | grep -Eq '"revealedRecv": "\[REDACTED\]\\"[0-9]+(\.[0-9]+)?\\"\[REDACTED\]"'
if printf '%s\n' "$REVEALED_RECV_LINE" | grep -Eiq 'base|currency|BTC|USD|set-cookie'; then
  echo "verification output leaked hidden Coinbase fields" >&2
  cat "$VERIFY_PATH" >&2
  exit 1
fi

echo "Compose TLSN E2E smoke passed."
