# Harden gstack pre-commit gate

Created: 2026-05-16
Model: GPT-5 Codex

## Priority

bug

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

The gstack pre-commit gate needs to handle fallback and missing-tool paths
without breaking supported installs or bypassing required checks.

## Rationale

`scripts/git-hooks/pre-commit` currently prefers the repo-local
`scripts/gstack` wrapper whenever it exists. If that wrapper cannot find a
gstack checkout, it exits under `set -e` before the hook can fall back to a
working `gstack` executable on `PATH`.

`scripts/gstack` also silently skips the review and `/cso` gate when the
selected gstack checkout has `gstack-update-check` but does not include
`gstack-review-read`. Older or incomplete installs can therefore pass the hook
without running the required gate.

Review finding: P2, confidence 10/10. The risky paths are
`scripts/git-hooks/pre-commit` lines 4-9 and `scripts/gstack` lines 58-68.

## Plan

- Update `scripts/git-hooks/pre-commit` so a failing repo wrapper can fall back
  to `gstack` on `PATH` when available.
- Update `scripts/gstack` to fail closed with an upgrade or setup message when
  `gstack-review-read` is unavailable.
- Add or update focused coverage for both install-layout paths.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `scripts/git-hooks/pre-commit`
- `scripts/gstack`
- `scripts/gstack-hooks.test.ts`
- `deno.json`

Verified with:

- `deno task test:scripts`
- `deno task lint:paths`

Harness update:

- `scripts/gstack-hooks.test.ts` covers PATH fallback, non-fallback failures,
  and missing `gstack-review-read` fail-closed behavior.

Review residuals:

- None

Follow-up:

- None
