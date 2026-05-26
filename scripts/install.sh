#!/usr/bin/env bash
# Build and install tlsn-curl, tlsn-verify, and their TLSNotary sidecars.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PREFIX="${PREFIX:-${HOME}/.local}"
BINDIR="${BINDIR:-$PREFIX/bin}"

mkdir -p "$BINDIR"

"$PROJECT_DIR/scripts/build-cli.sh"

install -m 0755 "$PROJECT_DIR/dist/tlsn-curl" "$BINDIR/tlsn-curl"
install -m 0755 "$PROJECT_DIR/dist/tlsn-verify" "$BINDIR/tlsn-verify"
install -m 0755 "$PROJECT_DIR/dist/tlsn-prove" "$BINDIR/tlsn-prove"
install -m 0755 "$PROJECT_DIR/dist/tlsn-verifier" "$BINDIR/tlsn-verifier"

printf 'Installed tlsn-curl and tlsn-verify to %s\n' "$BINDIR"
printf 'Ensure %s is on PATH before running them from another directory.\n' "$BINDIR"
