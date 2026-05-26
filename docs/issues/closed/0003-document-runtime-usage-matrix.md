# Document runtime usage matrix

Created: 2026-05-16
Model: GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- 0002

Blocks:
- None

## Summary

Document how to build, run, test, and verify `tlsn-fetch` with Deno and with
npm, yarn, pnpm, and bun. The README should make clear which commands are
equivalent across runtimes and which prerequisites are required.

## Rationale

Once npm-compatible entrypoints exist, users need a stable command matrix so
the project can be used from either Deno-native or Node-package-manager
workflows without guessing.

## Plan

- Add README examples for Deno, npm, yarn, pnpm, and bun.
- Include install/run commands for `tlsn-fetch`, `tlsn-verify`, and
  `build:tlsn`.
- Note runtime prerequisites such as Deno, Node-compatible package managers,
  and the Rust toolchain.
- Add a small verification checklist so future changes keep the command matrix
  accurate.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `README.md`
- `scripts/package-entrypoints.test.ts`

Verified with:

- `deno fmt --check`
- `deno task test:scripts`
- `deno task lint:paths`
- `deno lint`

Harness update:

- Extended `scripts/package-entrypoints.test.ts` to assert that README keeps
  Deno, npm, yarn, pnpm, and bun command examples documented.

Review residuals:

- None

Follow-up:

- None
