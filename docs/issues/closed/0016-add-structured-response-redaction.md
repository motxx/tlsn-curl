# Add structured response redaction

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- 0015

Blocks:
- None

## Summary

Add a higher-level response redaction interface for common JSON or text proofs,
implemented by resolving structured selections to received byte ranges.

## Rationale

Byte range redaction is correct but not ergonomic. Users usually know they want
to hide a JSON field, token-like substring, or response section rather than a
specific transcript offset. The structured API should be added only after the
range-based mechanism exists, so safety-critical transcript reveal behavior has
a single lower-level implementation.

## Plan

- Design a conservative syntax for response redaction, starting with JSON
  fields or exact text matches only if byte offset mapping is unambiguous.
- Reject ambiguous matches by default rather than redacting an unintended
  occurrence.
- Convert structured selections into the received byte ranges introduced by
  0015.
- Add tests covering encoding, duplicate values, missing paths or text, and
  unchanged proof verification for revealed data.
- Document limitations around compression, chunking, repeated values, and the
  difference between raw transcript bytes and parsed response views.

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

- Added CLI tests plus Rust tests for unique JSON values, duplicate values,
  chunked rejection, and reveal-only JSON output.

Review residuals:

- Structured redaction intentionally supports unchunked UTF-8 JSON Pointer
  values only.

Follow-up:

- `docs/issues/pending/0017-prove-bitcoin-usd-price-with-response-redaction.md`
