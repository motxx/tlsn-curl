# Add curl-like CLI interface

Created: 2026-05-16
Model: Codex (GPT-5)

## Priority

feature

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Make the CLI feel familiar to users who already know `curl`, while keeping the
project clearly proof-first rather than claiming full curl compatibility.
Renaming the command is allowed if the new name improves positioning without
overstating compatibility. Do not add shell-quoted curl command import.

## Rationale

The project is strongest as a non-browser, curl-like CLI for TLSNotary fetch
proofs. A familiar interface and name can reduce adoption friction, but full curl
compatibility would create incorrect expectations around redirects, cookies,
compression, HTTP/2, proxy behavior, multipart uploads, certificate options,
and other features that may not be supported safely by the prover.

The proof output semantics should stay explicit: `--out` writes the proof
envelope, and curl's `-o` response-output meaning should not be reused for
proof output.

## Plan

- Decide whether to keep `tlsn-fetch` or rename the command/package to a
  curl-like name such as `tlsn-curl`. If renaming, document the compatibility
  boundary prominently.
- Add curl-like aliases for already supported behavior, starting with
  `-H/--header` for request headers.
- Keep `--out` as the proof envelope output option; avoid assigning `-o` to
  proof output.
- Document the command as curl-like, not curl-compatible.
- Explicitly reject unsupported curl-like flags if they are introduced in the
  parser, instead of silently ignoring them.
- Consider follow-up support for common request options such as `-X/--request`,
  `-d/--data`, and `--json` only when verifier-side request binding can safely
  cover the resulting method and body semantics.
- Add CLI parser tests for the supported aliases and unsupported-option failure
  behavior.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `src/cli.ts`
- `src/cli_test.ts`
- `deno.json`
- `package.json`
- `bin/tlsn-curl.js`
- `scripts/package-entrypoints.test.ts`
- `scripts/smoke-built-cli.sh`
- `scripts/e2e-tlsn.sh`
- `scripts/local-up.sh`
- `README.md`

Verified with:

- `deno test --allow-read src/cli_test.ts`
- `deno test --allow-read --allow-write --allow-run=/bin/sh scripts/package-entrypoints.test.ts`
- `deno task lint:paths`
- `deno task test:scripts`
- `deno lint`
- `deno task test`
- `deno task test:cli-binary`

Harness update:

- `src/cli_test.ts` now covers the `-H` alias and unsupported curl-like option
  failures.
- `scripts/package-entrypoints.test.ts` now covers the `tlsn-curl` npm bin and
  README usage.

Review residuals:

- Full package/product renaming remains tracked in
  `docs/issues/pending/0028-rename-tlsn-fetch-to-tlsn-curl.md`.

Follow-up:

- `docs/issues/pending/0028-rename-tlsn-fetch-to-tlsn-curl.md`
