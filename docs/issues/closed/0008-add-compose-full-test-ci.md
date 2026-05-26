# Add Compose full test CI

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- 0006
- 0007

Blocks:
- None

## Summary

Add a full CI job that uses the local Docker Compose TLSNotary stack to run a
real proof-generation and verification test end to end.

## Rationale

The fast CI workflow can prove that code builds, but it cannot prove that the
local TLSNotary infrastructure, prover CLI, verifier CLI, and envelope format
work together. Once the compose-backed smoke harness exists, CI should execute
that same path so the repository can validate the complete workflow in a
repeatable environment.

## Plan

- Add a CI job with Docker Compose available.
- Start the local TLSNotary compose stack using isolated project names and
  ports.
- Run the compose-backed E2E smoke task from issue 0006.
- Upload logs or proof metadata on failure, excluding private proofs,
  credentials, and sensitive transcript data.
- Always tear down the compose stack at the end of the job.
- Gate full-test CI separately from the fast baseline if runtime or network
  stability requires it.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `.github/workflows/ci.yml`
- `.gitignore`
- `scripts/test-all.sh`

Verified with:

- `deno task test:scripts`
- `deno task lint:paths`
- `deno task ci:docker`

Harness update:

- The Docker CI job is now a separate Compose TLSN E2E gate using worktree
  isolation. On failure, it captures Compose `ps` and logs under
  `ci-artifacts/` and uploads only those logs.

Review residuals:

- The job depends on a public HTTPS endpoint and can fail if external network
  access is unavailable.

Follow-up:

- None
