# Remove library publish surface

Created: 2026-05-16
Model: Codex GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Make the project clearly CLI-only by removing JSR/library publishing metadata
and public library export surface that imply SDK support.

## Rationale

The project direction is `tlsn-curl` and `tlsn-verify` as command-line tools,
with proof JSON as the integration boundary. Keeping JSR publish metadata and a
library entrypoint suggests an SDK commitment the project does not intend to
make.

## Plan

- Remove JSR package metadata and Deno publish tasks/scripts.
- Remove the top-level library export entrypoint if it is only present for SDK
  publishing.
- Update tests and development docs so they guard the CLI package surface
  instead of JSR/library publishing.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `deno.json`
- `package.json`
- `.github/workflows/publish.yml`
- `scripts/check-release-tag-version.ts`
- `scripts/package-entrypoints.test.ts`
- `scripts/lint-types.ts`
- `scripts/lint-no-dynamic-import.ts`
- `scripts/lint-no-test-sanitizer-bypass.ts`
- `scripts/lint-no-unit-network-listener.ts`
- `docs/development.md`

Removed:

- `jsr.json`
- `src/mod.ts`

Verified with:

- `deno task lint:paths`
- `deno task test:scripts`
- `deno task check`
- `deno task release:check-tag v0.1.0`
- `deno task publish:npm:dry-run`
- `deno task build:cli`
- `deno task test`
- `deno task lint:strict`
- `deno task test:cli-binary`

Harness update:

- `scripts/package-entrypoints.test.ts` now guards only npm-compatible CLI
  package metadata.
- Repository lint scripts now skip tracked files that have been deleted in the
  working tree.

Review residuals:

- Historical closed issues still mention prior JSR work; those records were
  left unchanged.

Follow-up:

- None
