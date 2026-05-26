# Add Compose-backed E2E smoke

Created: 2026-05-16
Model: GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- 0005

Blocks:
- 0008

## Summary

Add a local smoke test and runbook that starts the Docker Compose TLSNotary
stack, generates a real proof, verifies the proof envelope, and then tears the
stack down.

## Rationale

The project now has real TLSNotary proof generation and verification, but the
manual path depends on the developer knowing how to start compatible local
infrastructure. A compose-backed smoke harness makes regressions visible and
keeps the documented workflow honest.

## Plan

- Add a script or Deno task that brings up the compose stack and waits for the
  verifier service to be reachable.
- Generate a proof against a stable public HTTPS endpoint using the local
  verifier address.
- Verify the resulting proof with `tlsn-verify` and assert the server name and
  revealed body marker.
- Tear down the compose stack even when proof generation fails.
- Update README or a runbook with the exact local-only workflow and expected
  outputs.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `scripts/e2e-tlsn.sh`
- `scripts/test-all.sh`
- `deno.json`
- `package.json`
- `scripts/package-entrypoints.test.ts`
- `README.md`
- `docs/development.md`

Verified with:

- `deno task test:scripts`
- `deno task lint:paths`
- `deno task test:e2e:tlsn`

Harness update:

- Added `deno task test:e2e:tlsn`, which starts Compose, waits for the local
  verifier TCP port, generates a Coinbase BTC/USD proof through the local
  verifier, verifies it, asserts that only the JSON price value is revealed,
  and tears Compose down.

Review residuals:

- The E2E target is a public HTTPS endpoint and still depends on network
  availability.

Follow-up:

- `docs/issues/pending/0008-add-compose-full-test-ci.md`
