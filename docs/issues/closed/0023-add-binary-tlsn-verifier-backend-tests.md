# Add Binary TLSN verifier backend tests

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

Add focused Deno tests for `src/tlsn_verifier.ts`, especially
`BinaryTlsnVerifierBackend`. The current Deno suite mostly verifies the wrapper
delegation path, leaving the binary verifier trust boundary thinly covered.

## Rationale

Coverage for `src/tlsn_verifier.ts` is low, and this module decides whether a
TLSNotary proof is accepted or rejected. Important branches include pending
proof rejection, base64 decoding limits, verifier process failures, malformed
verifier JSON, invalid verifier output, server-name mismatch, successful result
mapping, and temporary file cleanup.

## Plan

- Add tests that inject a fake verifier command or otherwise isolate
  `BinaryTlsnVerifierBackend` from the real Rust binary.
- Cover success, non-zero verifier exit, invalid JSON, `valid: false`, server
  name mismatch, pending envelopes, and oversized presentation data.
- Keep the tests under the Deno unit suite so `deno task test` exercises them.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `src/tlsn_verifier.ts`
- `src/tlsn_verifier_test.ts`

Verified with:

- `deno task test`
- `deno lint`
- `deno task lint:paths`

Harness update:

- `src/tlsn_verifier_test.ts` now covers `BinaryTlsnVerifierBackend` success,
  verifier failures, malformed JSON, invalid JSON, server-name mismatch,
  pending envelopes, oversized presentation data, and temp file cleanup.

Review residuals:

- None

Follow-up:

- None
