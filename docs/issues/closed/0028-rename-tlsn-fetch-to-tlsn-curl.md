# Rename tlsn-fetch to tlsn-curl

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

Remove the `tlsn-fetch` product, command, package, task, image, crate, and
documentation name from the repository so `tlsn-curl` is the only user-facing
name.

## Rationale

The project is moving to a curl-like interface and should use one consistent
name. Current references include CLI usage text, package metadata, Deno tasks,
binary names, Docker image names, Cargo crate names, tests, documentation, and
proof-format examples. The implementation should decide whether proof envelope
version strings such as `tlsn-fetch/v0` need a compatibility migration or can be
renamed directly.

## Plan

- Replace user-facing `tlsn-fetch` names with `tlsn-curl` across source,
  scripts, package metadata, docs, Docker metadata, and tests.
- Update executable, Deno task, npm bin, JSR/package names, and README examples
  so new usage consistently says `tlsn-curl`.
- Review generated artifact names, temp directory prefixes, User-Agent strings,
  and Docker/Cargo crate identifiers for compatibility-sensitive changes.
- Decide and document the proof version migration path before changing
  `tlsn-fetch/v0`.
- Run the full lint and test harness after the rename.

## Acceptance Criteria

- No active implementation, package metadata, generated binary wrapper, script,
  Docker metadata, Cargo metadata, test, or active documentation path contains
  `tlsn-fetch`; accepted exceptions are historical closed issue files and this
  tracking issue until it is closed.
- `rg -n "tlsn-fetch" src crates scripts bin package.json jsr.json deno.json docker-compose.yml README.md docs/proof-format.md docs/development.md`
  returns no matches.
- No legacy `tlsn-fetch` command, npm bin, Deno task, package name, crate name,
  Docker image name, temp prefix, or User-Agent remains unless a maintainer
  explicitly splits compatibility aliasing into a separate issue.
- Proof envelopes emitted by current code use a `tlsn-curl` version string.
  If verifier compatibility with older `tlsn-fetch/v0` proofs is retained, it
  must be isolated and documented as legacy input support, not as the current
  project name.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `README.md`
- `deno.json`
- `package.json`
- `jsr.json`
- `bin/tlsn-fetch.js`
- `docker-compose.yml`
- `.github/workflows/ci.yml`
- `scripts/docker-compose-env.sh`
- `scripts/e2e-tlsn.sh`
- `scripts/local-up.sh`
- `scripts/local-status.sh`
- `scripts/test-all.sh`
- `scripts/package-entrypoints.test.ts`
- `src/types.ts`
- `src/proof.ts`
- `src/cli.ts`
- `src/cli_test.ts`
- `src/tlsn_prover.ts`
- `src/tlsn_verifier.ts`
- `src/tlsn_verifier_test.ts`
- `crates/tlsn-prover/Cargo.toml`
- `crates/tlsn-prover/Cargo.lock`
- `crates/tlsn-prover/src/main.rs`
- `crates/tlsn-verifier/Cargo.toml`
- `crates/tlsn-verifier/Cargo.lock`
- `crates/tlsn-server/Cargo.toml`
- `crates/tlsn-server/Cargo.lock`
- `.gitleaks.toml`
- `.github/workflows/ci.yml`
- `.ignore`

Verified with:

- `rg -n "tlsn-fetch" src crates scripts bin package.json jsr.json deno.json docker-compose.yml README.md docs/proof-format.md docs/development.md`
- `deno test --allow-read src/cli_test.ts src/tlsn_verifier_test.ts`
- `deno test --allow-read --allow-write --allow-run=/bin/sh scripts/package-entrypoints.test.ts`
- `deno task lint:paths`
- `deno task test:scripts`
- `deno lint`
- `deno task check`
- `deno task test`
- `deno task test:cli-binary`
- `cargo check --manifest-path crates/tlsn-prover/Cargo.toml`
- `cargo check --manifest-path crates/tlsn-verifier/Cargo.toml`
- `cargo check --manifest-path crates/tlsn-server/Cargo.toml`
- `deno task publish:jsr:dry-run`
- `deno task publish:npm:dry-run`

Harness update:

- `scripts/package-entrypoints.test.ts` now asserts only `tlsn-curl` and
  `tlsn-verify` npm bins and the renamed package metadata.

Review residuals:

- None

Follow-up:

- None
