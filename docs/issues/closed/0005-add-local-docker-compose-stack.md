# Add local Docker Compose stack

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- 0004

Blocks:
- 0006

## Summary

Add a repository-local `docker-compose.yml` and supporting tasks so developers
can start the TLSNotary infrastructure with one command and run `tlsn-fetch`
against it without external services beyond the target HTTPS site.

## Rationale

The reference stack in `motxx/anchr` uses a dedicated compose project name and
environment-overridable ports to keep local runs isolated. This repository
needs the same ergonomics, but trimmed down to the TLSN verifier server and any
small support services required for local proof generation.

## Plan

- Add a `docker-compose.yml` with a TLSN verifier server service.
- Use environment-overridable project name and host ports so multiple
  worktrees or CI jobs can run without collisions.
- Add Deno tasks for `local:up`, `local:down`, and local status or health
  checks.
- Document the default verifier address that `tlsn-fetch --verifier` should
  use.
- Ensure generated volumes and build artifacts are ignored or scoped
  appropriately.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `docker-compose.yml`
- `scripts/local-up.sh`
- `scripts/local-down.sh`
- `scripts/local-status.sh`
- `deno.json`
- `package.json`
- `scripts/package-entrypoints.test.ts`
- `README.md`
- `docs/development.md`

Verified with:

- `docker compose config`
- `deno task local:up`
- `deno task local:status`
- `deno task local:down`
- `deno task test:scripts`
- `deno task lint:paths`

Harness update:

- Updated package metadata tests to cover local Compose scripts and current
  MIT/package file metadata.

Review residuals:

- None

Follow-up:

- `docs/issues/pending/0006-add-compose-backed-e2e-smoke.md`
