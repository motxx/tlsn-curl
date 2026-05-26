# Add recv byte range redaction

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- None

Blocks:
- 0016

## Summary

Add explicit received-transcript byte range redaction so callers can hide
selected response bytes while still proving the rest of the TLSNotary
presentation.

## Rationale

Header redaction only protects request secrets. Many useful proofs need to
show selected response facts while hiding nearby private response data. A byte
range API is the safest first step because TLSNotary selective disclosure is
range-based; higher-level JSON or selector redaction can build on it later.

## Plan

- Add CLI options for one or more received-transcript redaction ranges, using a
  strict `start:end` format with validation for ordering and non-negative
  offsets.
- Thread the ranges into the Rust prover and use them when building transcript
  commitments and presentation reveal ranges for received data.
- Keep request header redaction behavior unchanged.
- Add focused tests for range subtraction, out-of-bounds handling, and proof
  envelope verification behavior with partially revealed received data.
- Document that byte offsets apply to the received TLS transcript bytes, not a
  parsed JSON or decoded DOM view.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `src/cli.ts`
- `src/tlsn_prover.ts`
- `src/cli_test.ts`
- `crates/tlsn-prover/src/main.rs`
- `README.md`
- `docs/proof-format.md`

Verified with:

- `deno test --allow-read src/cli_test.ts`
- `deno lint`
- `deno task check`
- `cargo test --manifest-path crates/tlsn-prover/Cargo.toml`
- `cargo check --manifest-path crates/tlsn-prover/Cargo.toml`
- `deno task lint:paths`

Harness update:

- Added CLI argument tests and Rust range-subtraction/out-of-bounds tests.

Review residuals:

- None

Follow-up:

- `docs/issues/pending/0016-add-structured-response-redaction.md`
