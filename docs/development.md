# Development

## Quality Gates

```sh
./scripts/test-all.sh --local
./scripts/test-all.sh --docker
gitleaks detect --source . --config .gitleaks.toml --redact --no-banner
```

`./scripts/test-all.sh --local` is the local quality gate: local-path checks,
README command checks, Rust formatting, Rust tests, CLI smoke tests, and
TLSNotary sidecar builds. `./scripts/test-all.sh --docker` is the Docker-backed
Compose proof/verify smoke test.

The Gitleaks command runs with the repository `.gitleaks.toml` policy. Install
`gitleaks` locally before running it. The policy extends the built-in rule set
and only allowlists documented placeholders plus generated local artifact
paths.

Install the repository git hooks with:

```sh
git config core.hooksPath scripts/git-hooks
```

Set `GSTACK_PRE_COMMIT=0` to bypass the gstack pre-commit step for one commit.

## Implementation Map

- `crates/tlsn-cli`: Rust `tlsn-curl` and `tlsn-verify` CLI implementation,
  proof-envelope types, sidecar invocation, and claim checks.
- `crates/tlsn-prover`: `tlsn-prove` sidecar for TLSNotary proving.
- `crates/tlsn-verifier`: `tlsn-verifier` sidecar for presentation
  verification.
- `crates/tlsn-server`: local TCP verifier server for Docker Compose runs.

## Harness

- `scripts/test-all.sh`: single local and Docker-backed test runner.
- `scripts/docker-compose-env.sh`: isolated Docker Compose project and port
  environment.
- `scripts/local-up.sh`, `scripts/local-down.sh`, `scripts/local-status.sh`:
  Docker Compose wrappers for the local TLSN verifier server.
- `scripts/e2e-tlsn.sh`: Compose-backed proof generation and verification
  smoke test.
- `scripts/check-no-local-paths.sh`: repository-specific local path leak check.
- `scripts/check-docs.sh`: README command-surface drift check.
- `docs/review-harness.md`: where recurring review findings should be routed.

## README And Entrypoint Changes

- Keep README command examples aligned with `cargo build --release`,
  `cargo install --path .`, and the installed `tlsn-curl`/`tlsn-verify`
  commands.
- Run `./scripts/check-docs.sh` after changing README command examples.
- Run `./scripts/smoke-built-cli.sh` after changing the direct executable build.
- Run `./scripts/test-all.sh --local` before sending changes to CI.

## License

The repository `LICENSE` uses MIT. Direct dependencies are from TLSNotary Git
dependencies, RustCrypto, Tokio, Hyper, Serde, Clap, Chrono, Base64, Tempfile,
Anyhow, URL, futures, tungstenite, SOCKS, and small utility crates that publish
under MIT, Apache-2.0, or dual MIT/Apache-2.0 terms.
