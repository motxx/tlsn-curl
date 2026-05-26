# Add Unix stdio CLI mode

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

Make `tlsn-curl` and `tlsn-verify` compose cleanly as Unix-style tools.
`tlsn-curl --out -` should write the proof envelope JSON to stdout, and
`tlsn-verify -` should read a proof envelope from stdin.

## Rationale

The project is strongest as a CLI that emits and consumes stable proof JSON.
Supporting stdin/stdout lets users pipe through shell tools, CI jobs, queues,
and `jq` without needing an SDK or temporary files for every integration.

## Plan

- Add stdout proof output for `tlsn-curl --out -` while keeping diagnostics off
  stdout.
- Add stdin proof input for `tlsn-verify -`.
- Update help text, README, proof-format docs, and focused tests for the stdio
  workflow.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `src/cli.ts`
- `src/verify_cli.ts`
- `src/cli_test.ts`
- `scripts/smoke-built-cli.sh`
- `scripts/package-entrypoints.test.ts`
- `README.md`
- `docs/proof-format.md`

Verified with:

- `deno task lint:paths`
- `deno test --allow-read src/cli_test.ts`
- `deno task test:scripts`
- `deno task test:cli-binary`
- `deno check src/ scripts/`
- `deno lint`
- `deno task build:cli`
- `deno task test`
- `deno task lint:strict`

Harness update:

- `src/cli_test.ts` now covers `tlsn-curl --out -` and `tlsn-verify -`.
- `scripts/smoke-built-cli.sh` now exercises the standalone stdio pipeline.
- `scripts/package-entrypoints.test.ts` now guards README stdio examples.

Review residuals:

- None

Follow-up:

- None
