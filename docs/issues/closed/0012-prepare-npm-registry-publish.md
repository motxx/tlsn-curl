# Prepare npm registry publish

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- None

Blocks:
- 0013

## Summary

Make the existing npm-compatible package safe to publish to the npm registry.

## Rationale

`package.json` already defines `bin` entries for `tlsn-fetch` and
`tlsn-verify`, but registry publishing needs an explicit package boundary and
repeatable verification so generated artifacts, local files, proofs, or
development-only material are not shipped unintentionally. npm also supports
provenance for packages published from GitHub Actions.

References:
- https://docs.npmjs.com/cli/v11/commands/npm-publish/
- https://docs.npmjs.com/generating-provenance-statements

## Plan

- Decide whether the package should remain unscoped as `tlsn-fetch` or move to
  a scoped public package name before the first publish.
- Add package metadata needed for registry consumers, including repository,
  license, files/package exclusion policy, and publish access if scoped.
- Add a deterministic package preview check such as `npm pack --dry-run` and
  assert that only intended files are included.
- Document npm install and binary usage, including the Deno and Rust sidecar
  prerequisites.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `package.json`
- `deno.json`
- `.gitignore`
- `README.md`
- `docs/development.md`
- `scripts/package-entrypoints.test.ts`

Verified with:

- `deno fmt --check .gitignore jsr.json package.json deno.json src/mod.ts scripts/package-entrypoints.test.ts README.md docs/development.md`
- `deno task test:scripts`
- `deno task publish:npm:dry-run`
- `deno task publish:jsr:dry-run`
- `deno lint`
- `deno task check`
- `deno task lint:paths`

Harness update:

- `scripts/package-entrypoints.test.ts` now checks npm package metadata and
  bounded publish contents.

Review residuals:

- The package is marked `UNLICENSED` because no repository license has been
  selected yet.

Follow-up:

- 0013 closed tag-triggered publish workflow setup.
- 0014 tracks an explicit license decision before switching away from
  `UNLICENSED`.
