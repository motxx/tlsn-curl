#!/usr/bin/env bash
# Build the standalone CLI executables and run network-free smoke tests.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$PROJECT_DIR/dist"
CURL_BIN="$DIST_DIR/tlsn-curl"
VERIFY_BIN="$DIST_DIR/tlsn-verify"
PROOF="$DIST_DIR/smoke-proof.json"
VERIFY_STDOUT="$DIST_DIR/smoke-verify.stdout.json"
VERIFY_STDERR="$DIST_DIR/smoke-verify.stderr.txt"

mkdir -p "$DIST_DIR"

"$PROJECT_DIR/scripts/build-cli.sh"

"$CURL_BIN" "https://example.com" --out - --pending >"$PROOF"

grep -q '"kind": "tlsnotary-fetch-proof"' "$PROOF"
grep -q '"status": "pending"' "$PROOF"

set +e
"$VERIFY_BIN" - <"$PROOF" >"$VERIFY_STDOUT" 2>"$VERIFY_STDERR"
VERIFY_STATUS=$?
set -e

if [ "$VERIFY_STATUS" -eq 0 ]; then
  echo "expected tlsn-verify to reject pending proof" >&2
  exit 1
fi

if [ -s "$VERIFY_STDERR" ]; then
  echo "unexpected verifier stderr:" >&2
  cat "$VERIFY_STDERR" >&2
  exit 1
fi
grep -q '"ok": false' "$VERIFY_STDOUT"
grep -q 'pending' "$VERIFY_STDOUT"
