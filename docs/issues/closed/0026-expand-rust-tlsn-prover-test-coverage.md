# Expand Rust TLSN prover test coverage

Created: 2026-05-16
Model: GPT-5 Codex

## Priority

maintenance

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Expand unit coverage for `crates/tlsn-prover`. The crate has useful tests for
redaction helper behavior, but many CLI and sidecar paths remain untested.

## Rationale

`crates/tlsn-prover/src/main.rs` is the largest implementation file and contains
the sidecar logic that builds requests, applies sent and received transcript
redactions, chooses local or remote verifier flows, and emits the presentation.
Existing tests focus on selected redaction helpers and do not cover byte range
parsing, header parsing edge cases, response body extraction failures, or
configuration validation.

## Plan

- Add tests for `parse_byte_range`, including malformed input, non-numeric
  bounds, empty bounds, and `start >= end`.
- Add tests for response body extraction and JSON pointer redaction errors that
  are not already covered.
- Add tests around sent header value range matching, including repeated headers,
  case handling, missing headers, and headers with colons in values.
- Keep network and TLSNotary protocol execution in integration tests; prefer
  pure helper tests for this issue.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `crates/tlsn-prover/src/main.rs`

Verified with:

- `cargo fmt --manifest-path crates/tlsn-prover/Cargo.toml -- --check`
- `cargo test --manifest-path crates/tlsn-prover/Cargo.toml`
- `deno task lint:paths`

Harness update:

- `crates/tlsn-prover/src/main.rs` now has unit tests for byte range parsing,
  sent-header value matching, response JSON redaction errors, and conflicting
  reveal/redact configuration.

Review residuals:

- None

Follow-up:

- None
