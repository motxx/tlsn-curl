# Build direct tlsn-fetch executable

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Make the fetch CLI buildable as a direct `tlsn-fetch` executable so users can
run `tlsn-fetch ...` instead of `deno task tlsn-fetch ...`.

## Rationale

The repository currently exposes `tlsn-fetch` through a Deno task that runs
`src/cli.ts`. This is useful for development, but it still requires users to
know the repository task name and invoke Deno explicitly. A buildable executable
would make local installs, release artifacts, and downstream package entrypoints
simpler while preserving the existing Deno task workflow.

## Plan

- Add a build command that produces a `tlsn-fetch` executable from `src/cli.ts`
  with the required runtime permissions.
- Decide the artifact location and make sure generated binaries are ignored by
  version control when appropriate.
- Add a focused smoke check that invokes the built executable in a mode that
  does not require a live TLSNotary proof.
- Update README usage so the direct `tlsn-fetch` command is documented
  alongside the existing `deno task tlsn-fetch` development path.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `deno.json`
- `package.json`
- `scripts/smoke-built-cli.sh`
- `scripts/test-all.sh`
- `scripts/package-entrypoints.test.ts`
- `.gitignore`
- `README.md`

Verified with:

- `deno task test:cli-binary`
- `deno task test:scripts`
- `deno task lint:paths`
- `deno lint`
- `npm run check`

Harness update:

- Added `deno task test:cli-binary`, which compiles `dist/tlsn-fetch` and
  runs it with `--pending` to avoid external TLSNotary dependencies.

Review residuals:

- None

Follow-up:

- None
