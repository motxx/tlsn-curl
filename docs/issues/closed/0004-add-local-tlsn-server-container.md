# Add local TLSN server container

Created: 2026-05-16
Model: GPT-5

## Priority

feature

## Dependencies

Depends on:
- None

Blocks:
- 0005
- 0006

## Summary

Add a local Docker image for a TLSNotary-compatible verifier server so
`tlsn-fetch` can run against a fully local notary/verifier endpoint during
development.

## Rationale

The current repository has prover and verifier CLI sidecars, but no local
long-running verifier server container. The `motxx/anchr` repository provides a
working reference for a TLSN server Dockerfile and service shape that can be
adapted here without requiring the rest of the Anchr stack.

## Plan

- Add or adapt a minimal `tlsn-server` crate or equivalent server component
  needed by Docker Compose.
- Add a Dockerfile that builds the server with locked Rust dependencies and
  runs it as a non-root user.
- Expose TCP and WebSocket ports with defaults compatible with the prover
  options.
- Keep the image scoped to TLSNotary infrastructure only; do not pull in
  unrelated Anchr services.
- Verify the container starts and advertises usable local ports.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `crates/tlsn-server/Cargo.toml`
- `crates/tlsn-server/Cargo.lock`
- `crates/tlsn-server/src/main.rs`
- `Dockerfile.tlsn-server`
- `.dockerignore`
- `deno.json`
- `package.json`
- `docs/development.md`

Verified with:

- `cargo check --manifest-path crates/tlsn-server/Cargo.toml`
- `docker build -f Dockerfile.tlsn-server -t tlsn-fetch-server:local .`
- `docker run --rm -d --name tlsn-fetch-server-test -p 127.0.0.1:17047:7047 tlsn-fetch-server:local`
- `docker ps --filter name=tlsn-fetch-server-test --format '{{.Names}} {{.Status}} {{.Ports}}'`
- `docker logs --tail 20 tlsn-fetch-server-test`

Harness update:

- Added the server crate and image entrypoint needed by the Compose harness.

Review residuals:

- The container exposes the TCP verifier protocol used by `--verifier
  localhost:7047`; WebSocket verifier support is not included in this local
  image.

Follow-up:

- `docs/issues/pending/0005-add-local-docker-compose-stack.md`
