# Add npm-compatible package entrypoints

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- None

Blocks:
- 0003

## Summary

Make `tlsn-fetch` installable and runnable from npm-compatible package
managers while keeping the current Deno workflow intact. Users should be able
to invoke the CLI through npm, yarn, pnpm, and bun without hand-wiring Deno
tasks.

## Rationale

The repository currently exposes Deno tasks, but does not define npm package
metadata, package-manager scripts, or bin entrypoints. Supporting the common
JavaScript package managers makes the tool easier to consume in projects that
do not standardize on Deno.

## Plan

- Add package metadata and bin entrypoints for `tlsn-fetch` and `tlsn-verify`.
- Decide whether npm-compatible entrypoints call Deno directly or use a built
  JavaScript artifact, and document any runtime prerequisite.
- Add scripts that work under npm, yarn, pnpm, and bun for tests, linting, and
  TLSNotary sidecar build commands.
- Keep the existing `deno task tlsn-fetch` and `deno task tlsn-verify`
  behavior unchanged.
- Add focused tests or smoke checks for the package-manager entrypoints.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `package.json`
- `bin/tlsn-fetch.js`
- `bin/tlsn-verify.js`
- `scripts/package-entrypoints.test.ts`
- `.gitignore`

Verified with:

- `deno task test:scripts`
- `deno task lint:paths`
- `deno lint`
- `node bin/tlsn-fetch.js --help`
- `node bin/tlsn-verify.js --help`

Harness update:

- Added `scripts/package-entrypoints.test.ts` to keep package metadata,
  scripts, and bin wrappers aligned with the Deno CLIs.

Review residuals:

- Runtime usage documentation remains owned by issue 0003.

Follow-up:

- 0003
