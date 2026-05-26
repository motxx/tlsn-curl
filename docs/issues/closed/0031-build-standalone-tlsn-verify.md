# Build standalone tlsn-verify

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

`deno task build:cli` currently compiles only `dist/tlsn-curl`. Add
`tlsn-verify` to the standalone CLI build so users can run verification without
going through the Deno task or npm wrapper entrypoint.

## Rationale

The package exposes `tlsn-verify` as a Deno task and npm-compatible Node
wrapper, but the direct executable build only documents and emits
`dist/tlsn-curl`. Verification should have the same standalone distribution
path as proof generation.

## Plan

- Update the CLI build task to compile both `src/cli.ts` and
  `src/verify_cli.ts` into `dist/`.
- Extend the built CLI smoke test to cover `dist/tlsn-verify`.
- Update README and package metadata tests so the standalone verifier is
  documented and guarded.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `deno.json`
- `scripts/smoke-built-cli.sh`
- `scripts/package-entrypoints.test.ts`
- `README.md`

Verified with:

- `deno task lint:paths`
- `deno task test:cli-binary`
- `deno task test:scripts`
- `deno lint`
- `deno task build:cli`

Harness update:

- `scripts/smoke-built-cli.sh` now compiles and exercises
  `dist/tlsn-verify`.
- `scripts/package-entrypoints.test.ts` now checks README documentation for
  `./dist/tlsn-verify`.

Review residuals:

- None

Follow-up:

- None
