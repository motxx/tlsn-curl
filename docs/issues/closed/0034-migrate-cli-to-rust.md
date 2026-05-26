# Migrate CLI to Rust

Created: 2026-05-16
Model: Codex GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Replace the TypeScript/Deno CLI implementation with a Rust CLI so the project
ships as a conventional native command-line tool without a Deno runtime or
compiled-Deno wrapper in the primary path.

## Rationale

The project is now positioned as `tlsn-curl` and `tlsn-verify` command-line
tools rather than a TypeScript SDK. The core TLSNotary integration already
lives in Rust sidecars, while the Deno layer mainly handles argument parsing,
proof envelope JSON, subprocess orchestration, and verification result checks.
Keeping that layer in TypeScript adds build, install, permission, and sidecar
resolution complexity without much product benefit unless the project chooses
to expose a TypeScript SDK.

Moving the CLI to Rust should make installation and release artifacts simpler,
reduce the number of runtimes users need to understand, and allow the CLI,
prover, verifier, and local server crates to share internal Rust types and
logic.

## Plan

- Define the target Rust crate layout for `tlsn-curl`, `tlsn-verify`,
  `tlsn-prove`, `tlsn-verifier`, and shared proof-envelope types.
- Port TypeScript CLI parsing and proof-envelope generation from `src/cli.ts`,
  `src/proof.ts`, and `src/types.ts` into Rust.
- Port verification-envelope handling and claim validation from
  `src/verify_cli.ts` and `src/tlsn_verifier.ts` into Rust.
- Preserve the existing command-line surface, JSON proof format, stdout/stderr
  behavior, pending mode, header redaction semantics, and pipeline behavior.
- Replace Deno-based binary builds and install scripts with Cargo-native build
  and install workflows.
- Update README, development docs, and runtime-entrypoint tests so Rust is the
  primary implementation path and Deno is removed or limited to migration-only
  test fixtures.
- Run the existing Deno and Rust tests during the migration, then remove or
  rewrite Deno tests once equivalent Rust coverage exists.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `crates/tlsn-cli/Cargo.toml`
- `Cargo.toml`
- `crates/tlsn-cli/src/lib.rs`
- `crates/tlsn-cli/src/bin/tlsn-curl.rs`
- `crates/tlsn-cli/src/bin/tlsn-verify.rs`
- `scripts/build-cli.sh`
- `scripts/install.sh`
- `scripts/smoke-built-cli.sh`
- `scripts/e2e-tlsn.sh`
- `scripts/test-all.sh`
- `scripts/local-up.sh`
- `README.md`
- `docs/development.md`
- `docs/review-harness.md`
- `docs/issues/README.md`
- `scripts/check-docs.sh`
- `scripts/check-no-local-paths.sh`
- `scripts/git-hooks/pre-commit`
- `scripts/git-hooks/commit-msg`

Removed:

- `deno.json`
- `deno.lock`
- `src/cli.ts`
- `src/cli_test.ts`
- `src/proof.ts`
- `src/tlsn_prover.ts`
- `src/tlsn_verifier.ts`
- `src/tlsn_verifier_test.ts`
- `src/types.ts`
- `src/verify_cli.ts`
- `scripts/check-gstack-review-gate.ts`
- `scripts/check-gstack-review-gate.test.ts`
- `scripts/check-no-local-paths.ts`
- `scripts/check-no-local-paths.test.ts`
- `scripts/gstack-hooks.test.ts`
- `scripts/lint-no-dynamic-import.ts`
- `scripts/lint-no-dynamic-import.test.ts`
- `scripts/lint-no-test-sanitizer-bypass.ts`
- `scripts/lint-no-test-sanitizer-bypass.test.ts`
- `scripts/lint-no-unit-network-listener.ts`
- `scripts/lint-no-unit-network-listener.test.ts`
- `scripts/lint-types.ts`
- `scripts/lint-types.test.ts`
- `scripts/runtime-entrypoints.test.ts`

Verified with:

- `cargo test --manifest-path Cargo.toml`
- `cargo fmt --manifest-path Cargo.toml -- --check`
- `./scripts/check-no-local-paths.sh`
- `./scripts/check-docs.sh`
- `./scripts/smoke-built-cli.sh`
- `./scripts/test-all.sh --local`
- `BINDIR=<tmp-install-dir> ./scripts/install.sh`
- `cargo install --path . --root <tmp-cargo-root>`

Harness update:

- `crates/tlsn-cli/src/lib.rs` now has Rust unit coverage for CLI parsing,
  proof-envelope generation, sidecar argument mapping, request-claim
  validation, and JSON value extraction.
- `scripts/check-docs.sh` now guards the Cargo-first README commands and
  rejects legacy Deno CLI entrypoint examples.
- `scripts/smoke-built-cli.sh` now builds and exercises the Rust CLI binaries.
- `scripts/check-no-local-paths.sh` now provides the local-path guard without
  requiring a Deno runtime.

Review residuals:

- None

Follow-up:

- None
