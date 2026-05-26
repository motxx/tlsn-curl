# Prepare JSR publish metadata

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

Make the Deno package publishable to JSR by adding explicit package metadata
and an export surface suitable for `deno publish` or `jsr publish`.

## Rationale

The repository currently has Deno tasks and npm-compatible wrappers, but
`deno.json` does not declare a JSR package name, version, or exports. Deno's
publish command requires `name`, `version`, and `exports` in `deno.json` or
`jsr.json`, and JSR expects an ESM-compatible package surface.

References:
- https://docs.deno.com/runtime/reference/cli/publish/
- https://jsr.io/docs/publishing-packages

## Plan

- Choose the public JSR scope and package name, then add `name`, `version`, and
  `exports` to `deno.json` or a dedicated `jsr.json`.
- Expose stable library entrypoints for proof types, proof construction, and
  verification helpers without publishing internal CLI-only modules as public
  API by accident.
- Add a dry-run check such as `deno publish --dry-run` or `npx jsr publish
  --dry-run` to the release verification path.
- Document the JSR package name and install/import examples in `README.md`.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `jsr.json`
- `src/mod.ts`
- `deno.json`
- `README.md`
- `docs/development.md`
- `scripts/package-entrypoints.test.ts`

Verified with:

- `deno fmt --check .gitignore jsr.json package.json deno.json src/mod.ts scripts/package-entrypoints.test.ts README.md docs/development.md`
- `deno task test:scripts`
- `deno task publish:jsr:dry-run`
- `deno task publish:npm:dry-run`
- `deno lint`
- `deno task check`
- `deno task lint:paths`

Harness update:

- `scripts/package-entrypoints.test.ts` now checks JSR metadata, exports, and
  publish include boundaries.

Review residuals:

- JSR package must still be pre-created and linked to this GitHub repository
  before real tokenless publish.

Follow-up:

- 0013 closed tag-triggered publish workflow setup.
- 0014 tracks an explicit license decision before switching away from
  `UNLICENSED`.
