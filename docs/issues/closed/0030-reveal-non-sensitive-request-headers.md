# Reveal non-sensitive request headers

Created: 2026-05-16
Model: GPT-5 Codex

## Priority

design

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Review the current request-header redaction policy so non-sensitive custom
request headers can remain revealed in TLSNotary presentations, while sensitive
header values stay hidden by default.

## Rationale

`tlsn-curl` is strongest when a verifier can confirm not only that a response
came from the claimed TLS server, but also the meaningful request conditions
that may affect that response. The verifier already requires the sent request
line and `Host` header to be revealed so the proof binds to the claimed URL.

Current CLI plumbing passes every custom request header name to
`--redact-sent-header`, including headers provided with `--header` rather than
`--header-env`. This is safe for secrets, but too conservative for public
headers such as feature flags, language preferences, or account-independent
API selectors whose values may be important to the verified claim.

## Plan

- Decide the intended public API for request-header disclosure, preserving
  default protection for sensitive headers such as `Authorization`, `Cookie`,
  and `X-Api-Key`.
- Change CLI/prover plumbing so `--header-env` values are redacted by default,
  while non-sensitive `--header` values are revealed unless explicitly redacted.
- Add or update tests proving that sensitive headers remain hidden, non-sensitive
  custom headers can be verified, and request binding still fails closed when
  required URL-binding bytes are hidden.
- Update `docs/proof-format.md` and `README.md` to describe the revised
  disclosure policy.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `src/cli.ts`
- `src/tlsn_prover.ts`
- `src/tlsn_verifier.ts`
- `src/cli_test.ts`
- `src/tlsn_verifier_test.ts`
- `README.md`
- `docs/proof-format.md`

Verified with:

- `deno test --allow-read src/cli_test.ts src/tlsn_verifier_test.ts`
- `deno task lint:paths`
- `deno lint`
- `deno task test:scripts`
- `deno check src/ scripts/`

Harness update:

- `src/cli_test.ts` covers public header claims and sidecar sent-header
  redaction selection.
- `src/tlsn_verifier_test.ts` covers public header verification and fail-closed
  behavior when a claimed public header is hidden.

Review residuals:

- None

Follow-up:

- None
