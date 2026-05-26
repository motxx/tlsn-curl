# Add tag-triggered publish workflow

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- 0011
- 0012

Blocks:
- None

## Summary

Publish the package automatically when a release tag is pushed, covering both
JSR for Deno users and npm for Node package-manager users.

## Rationale

Release publishing should be tied to immutable version tags so the package
registry versions correspond to source history. GitHub Actions can publish JSR
packages with OIDC after the package is linked to the repository in JSR, and
npm publishing should use either trusted publishing/provenance or a narrowly
scoped publish token.

References:
- https://jsr.io/docs/publishing-packages
- https://jsr.io/docs/trust
- https://docs.npmjs.com/trusted-publishers
- https://docs.npmjs.com/generating-provenance-statements

## Plan

- Add a `.github/workflows/publish.yml` workflow triggered by version tags such
  as `v*`.
- Run the existing local gate before publishing, including Deno checks and the
  package dry-run checks added by 0011 and 0012.
- Publish to JSR with OIDC from GitHub Actions, requiring the JSR package to be
  pre-created and linked to this GitHub repository.
- Publish to npm with provenance or trusted publishing, documenting any one-time
  registry setup that cannot be represented safely in the repository.
- Ensure the workflow fails clearly when the tag version and package metadata
  version diverge.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `.github/workflows/publish.yml`
- `scripts/check-release-tag-version.ts`
- `scripts/check-release-tag-version.test.ts`
- `deno.json`
- `package.json`
- `docs/development.md`
- `scripts/package-entrypoints.test.ts`

Verified with:

- `deno fmt --check .github/workflows/publish.yml .gitignore jsr.json package.json deno.json src/mod.ts scripts/check-release-tag-version.ts scripts/check-release-tag-version.test.ts scripts/package-entrypoints.test.ts README.md docs/development.md`
- `deno task test:scripts`
- `deno task release:check-tag v0.1.0`
- `deno task publish:npm:dry-run`
- `deno task publish:jsr:dry-run`
- `deno lint`
- `deno task check`
- `deno task lint:paths`

Harness update:

- `scripts/check-release-tag-version.test.ts` covers tag/package version
  mismatch handling.
- `scripts/package-entrypoints.test.ts` checks release and publish scripts.

Review residuals:

- Real JSR and npm publishing still requires the one-time registry setup
  documented in `docs/development.md`.

Follow-up:

- None
