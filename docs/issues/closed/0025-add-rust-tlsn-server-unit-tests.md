# Add Rust TLSN server unit tests

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

Add unit tests for `crates/tlsn-server`. The crate currently builds, but
`cargo test --manifest-path crates/tlsn-server/Cargo.toml` reports zero tests.

## Rationale

The local TLSNotary verifier server coordinates MPC and attestation flows. Most
of that behavior is integration-heavy, but several smaller decisions can be
tested without a live TLSNotary session, including transcript length
calculation, session waiting behavior, and connection mode validation.

## Plan

- Add tests for `application_data_len` so non-application records are excluded
  and multiple application records are summed correctly.
- Refactor small pieces only where needed to test unknown connection mode
  handling or session wait timeout behavior without opening a long-lived
  listener.
- Run `cargo test --manifest-path crates/tlsn-server/Cargo.toml` and keep the
  Compose E2E smoke test unchanged unless a regression is found.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `crates/tlsn-server/src/main.rs`

Verified with:

- `cargo fmt --manifest-path crates/tlsn-server/Cargo.toml -- --check`
- `cargo test --manifest-path crates/tlsn-server/Cargo.toml`
- `deno task lint:paths`

Harness update:

- `crates/tlsn-server/src/main.rs` now has unit tests for application-data
  transcript length and connection mode validation.

Review residuals:

- None

Follow-up:

- None
