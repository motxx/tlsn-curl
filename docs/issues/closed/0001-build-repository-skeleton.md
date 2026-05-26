# Build repository skeleton

Created: 2026-05-16
Model: GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Create the minimal skeleton for proving an HTTPS fetch with TLSNotary.

Add the README, toolchain, CLI shape, types, verifier wrapper, and basic tests.

## Rationale

The repository is nearly empty. Start with a small, testable surface before
adding real TLSNotary infrastructure.

## Plan

- Add `README.md` and `deno.json`.
- Add a `tlsn-fetch <url> --out <file>` CLI skeleton.
- Define proof/result types.
- Add a thin `tlsn-verifier` wrapper.
- Add tests that do not need TLSNotary infrastructure.
- Add an optional real TLSNotary E2E runbook.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `README.md`
- `deno.json`
- `deno.lock`
- `scripts/check-no-local-paths.test.ts`
- `src/cli.ts`
- `src/cli_test.ts`
- `src/proof.ts`
- `src/tlsn_verifier.ts`
- `src/types.ts`

Verified with:

- `deno fmt --check README.md deno.json src/types.ts src/proof.ts src/tlsn_verifier.ts src/cli.ts src/cli_test.ts docs/issues/closed/0001-build-repository-skeleton.md`
- `deno task test`
- `deno lint`
- `deno task test:scripts`
- `deno task lint:paths`

Harness update:

- `src/cli_test.ts` added to cover CLI parsing, proof envelope creation, and
  verifier delegation without TLSNotary infrastructure.

Review residuals:

- None

Follow-up:

- None
