#!/usr/bin/env bash
# Build Rust tlsn-curl, tlsn-verify, and sidecar executables into dist/.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$PROJECT_DIR/dist"
TARGET_DIR="$PROJECT_DIR/target/release"

mkdir -p "$DIST_DIR"

cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

install -m 0755 "$TARGET_DIR/tlsn-curl" "$DIST_DIR/tlsn-curl"
install -m 0755 "$TARGET_DIR/tlsn-verify" "$DIST_DIR/tlsn-verify"
install -m 0755 "$TARGET_DIR/tlsn-prove" "$DIST_DIR/tlsn-prove"
install -m 0755 "$TARGET_DIR/tlsn-verifier" "$DIST_DIR/tlsn-verifier"
