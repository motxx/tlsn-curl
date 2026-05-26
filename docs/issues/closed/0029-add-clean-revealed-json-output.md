# Add clean revealed JSON output

Created: 2026-05-16
Model: Codex GPT-5

## Priority

feature

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Improve verifier output for `--reveal-response-json` so users do not need to
manually strip `[REDACTED]` markers from `revealedRecv` or `revealedBody`.
Expose clean revealed segments and JSON-parsed values while preserving the
existing transcript fields for debugging and compatibility.

## Rationale

`--reveal-response-json` intentionally hides most response bytes, so the current
`revealedRecv` output is dominated by `[REDACTED]` markers. That is correct for
debugging the transcript proof but poor UX for users who want the extracted
verified value or values.

## Plan

- Add marker-free revealed body segment output from the verifier backend.
- Parse JSON value segments when possible and include them in the verification
  result.
- Keep existing `revealedRecv` and `revealedBody` fields for compatibility.
- Add tests for marker-free segments and JSON value extraction.
- Document the new output fields.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `crates/tlsn-verifier/src/main.rs`
- `src/types.ts`
- `src/mod.ts`
- `src/tlsn_verifier.ts`
- `src/tlsn_verifier_test.ts`
- `src/cli_test.ts`
- `README.md`
- `docs/proof-format.md`

Verified with:

- `cargo test --manifest-path crates/tlsn-verifier/Cargo.toml`
- `deno test --allow-read src/tlsn_verifier_test.ts src/cli_test.ts`
- `deno task lint:paths`
- `deno lint`
- `cargo fmt --manifest-path crates/tlsn-verifier/Cargo.toml -- --check`

Harness update:

- `crates/tlsn-verifier/src/main.rs` unit tests cover marker-free transcript and body segments.
- `src/tlsn_verifier_test.ts` covers JSON value extraction from multiple revealed body segments.
- `src/cli_test.ts` covers surfaced `revealedJsonValues` in verifier CLI JSON.

Review residuals:

- None

Follow-up:

- None
