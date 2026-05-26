#!/usr/bin/env bash
# Guard README command examples for the Cargo-first CLI surface.

set -euo pipefail

README="${1:-README.md}"

require() {
  local text="$1"
  if ! grep -Fq -- "$text" "$README"; then
    echo "README is missing: $text" >&2
    exit 1
  fi
}

reject() {
  local text="$1"
  if grep -Fq -- "$text" "$README"; then
    echo "README still advertises: $text" >&2
    exit 1
  fi
}

require "cargo build --release"
require "cargo install --path ."
require "./target/release/tlsn-curl"
require "./target/release/tlsn-verify"
require "implementation sidecars"
require "--out - --pending"
require "docs/proof-format.md"
require "docs/development.md"

reject "deno task tlsn-curl"
reject "deno task tlsn-verify"
reject "deno compile"
reject "cargo install --path crates/tlsn-cli"
reject "npm install"
reject "npm run"
reject "Node wrappers"

echo "README command surface looks current"
