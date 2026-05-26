# Review Harness

This document maps recurring review concerns to the automated harness that
should catch them next time. When a finding repeats, prefer extending a script,
test, or issue over relying on human memory.

## Commands

| Command | Owner |
| --- | --- |
| `./scripts/test-all.sh --local` | Local path leaks, README command drift, Rust formatting, Rust tests, CLI smoke tests, and TLSNotary sidecar builds. |
| `./scripts/test-all.sh --docker` | Docker-backed Compose proof/verify smoke test. |
| `./scripts/check-no-local-paths.sh` | Fast local path leak check for issue files and docs. |

## Routing

| Review concern | Harness update |
| --- | --- |
| Rust API or type-safety regression | Add or extend Rust unit tests in the owning crate. |
| Developer-local paths in tracked text | Extend `scripts/check-no-local-paths.sh`. |
| TLSNotary proof-generation regression | Add or extend `scripts/e2e-tlsn.sh` or Rust coverage in the owning crate. |
| Package-manager or CI command drift | Update `Cargo.toml`, CI workflow, and README examples in the same change. |

## Residual Review

After the harness passes, human review should focus on decisions that cannot be
reduced to deterministic checks yet: protocol scope, security risk acceptance,
public vocabulary, and dependency or infrastructure policy.
