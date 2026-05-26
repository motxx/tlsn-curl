# Bind verified request to claim

Created: 2026-05-16
Model: GPT-5 Codex

## Priority

bug

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

`tlsn-verify` should only return a successful claim when the verified TLS
presentation proves the same HTTP request target and method recorded in the
proof envelope.

## Rationale

The current verifier checks the TLS server name against the claimed URL host,
but it does not bind the claimed method, path, query, or Host header to the
revealed sent HTTP transcript. That leaves the URL-level claim weaker than the
proof envelope implies.

Detailed security notes are in the local CSO report under `.gstack/`.

Review finding: P1, confidence 9/10. The risky path is
`src/tlsn_verifier.ts` lines 101-122, where verification checks
`server_name` and then returns `proof.claim` without validating the revealed
sent request bytes.

## Plan

- Parse the verifier backend's revealed sent request bytes after successful
  TLSNotary verification.
- Compare the request line with `proof.claim.method` and the claimed URL path
  plus query string.
- Compare the revealed Host header with the claimed URL host.
- Fail closed when the sent request bytes are absent or redacted enough to
  prevent the comparison.
- Add tests for matching claims and same-host mismatched claims.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `src/tlsn_verifier.ts`
- `src/tlsn_verifier_test.ts`
- `scripts/gstack-hooks.test.ts`

Verified with:

- `deno test --allow-read src/tlsn_verifier_test.ts`
- `deno task test`
- `deno task test:scripts`
- `deno lint`
- `deno task lint:paths`

Harness update:

- `src/tlsn_verifier_test.ts` covers matching request binding, same-host method
  mismatch, same-host target mismatch, Host mismatch, and redacted binding data.
- `scripts/gstack-hooks.test.ts` now runs hook integration tests only when the
  required write/run permissions are present, keeping the default test task
  green while preserving coverage in `deno task test:scripts`.

Review residuals:

- None

Follow-up:

- None
