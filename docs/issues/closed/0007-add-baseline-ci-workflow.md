# Add baseline CI workflow

Created: 2026-05-16
Model: GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- None

Blocks:
- 0008

## Summary

Add a CI workflow that runs the repository's fast, deterministic checks on
every pull request and push. The baseline workflow should cover Deno tests,
linting, local-path leak detection, Rust sidecar checks, and TLSNotary sidecar
builds.

## Rationale

The project now spans Deno and Rust. CI needs a fast layer that catches
formatting, type, unit-test, and sidecar build regressions before the slower
Docker Compose E2E workflow runs.

## Plan

- Add a CI workflow for pushes and pull requests.
- Install the required Deno and Rust toolchains.
- Run `deno task test`, `deno lint`, `deno task lint:paths`, and
  `deno task build:tlsn`.
- Run `cargo check` for the prover and verifier crates.
- Cache Deno and Cargo dependencies without caching generated proof artifacts.
- Document the CI commands in the README or a contributor note.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `.github/workflows/ci.yml`
- `deno.json`
- `scripts/test-all.sh`
- `scripts/docker-compose-env.sh`
- `scripts/lint-types.ts`
- `scripts/lint-no-dynamic-import.ts`
- `scripts/lint-no-test-sanitizer-bypass.ts`
- `scripts/lint-no-unit-network-listener.ts`
- `scripts/git-hooks/pre-commit`
- `README.md`
- `docs/review-harness.md`

Verified with:

- `deno task ci`
- `deno task ci:docker`
- `deno task lint:strict`
- `deno task test:scripts`
- `git diff --check`

Harness update:

- Added `deno task lint:strict`, `deno task test:all`, `deno task ci`, and
  `.github/workflows/ci.yml` as the baseline quality gate.

Review residuals:

- Full Docker Compose E2E remains owned by pending issue 0008 after issues
  0004, 0005, and 0006 are completed.

Follow-up:

- 0008
