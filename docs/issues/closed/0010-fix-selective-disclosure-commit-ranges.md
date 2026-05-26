# Fix selective disclosure commit ranges

Created: 2026-05-16
Model: GPT-5

## Priority

bug

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Make request-header redaction cryptographically valid by aligning TLSNotary
transcript commitments with the ranges that `tlsn-fetch` later reveals in the
presentation.

## Rationale

`[REDACTED]` is only verifier-side rendering after `presentation.verify()`
succeeds; it should not be treated as proof rewriting. The real selective
disclosure boundary is the transcript proof: revealed ranges must be covered by
commitments created during the prove phase.

The current prover commits the full sent and received transcripts as coarse
ranges, then tries to build a presentation that excludes selected request
header values. TLSNotary's transcript proof builder requires reveal ranges to
be coverable by available commitment ranges, so coarse full-transcript
commitments are not sufficient for arbitrary redaction gaps.

## Plan

- Change the prover's transcript commit strategy so sent request data is
  committed in ranges that can cover the non-redacted reveal ranges without
  opening redacted header values.
- Keep received transcript handling explicit: either continue revealing the
  full response with a matching commitment strategy, or add response-side
  selective disclosure only with matching commit ranges.
- Add a focused test or smoke fixture that proves a custom sensitive request
  header is absent from verified `revealed_sent` while the presentation still
  verifies successfully.
- Document that `[REDACTED]` is verifier rendering of unauthenticated bytes from
  a verified partial transcript, not a mutation of the proof artifact.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `crates/tlsn-prover/src/main.rs`
- `README.md`

Verified with:

- `cargo fmt --manifest-path crates/tlsn-prover/Cargo.toml -- --check`
- `cargo test --manifest-path crates/tlsn-prover/Cargo.toml`
- `deno task build:tlsn`
- `deno task tlsn-fetch https://example.com --out <tmp-proof> --header-env "Authorization: SECRET_TOKEN" --max-recv-data 8192`
- `deno task tlsn-verify <tmp-proof>`
- `deno task test:scripts`
- `deno task lint:paths`

Harness update:

- Added Rust unit tests for request-header selective disclosure range handling.

Review residuals:

- None

Follow-up:

- None
