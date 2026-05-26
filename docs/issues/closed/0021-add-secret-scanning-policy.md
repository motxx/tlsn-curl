# Add secret scanning policy

Created: 2026-05-16
Model: GPT-5 Codex

## Priority

maintenance

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Add a versioned secret-scanning policy so local and CI checks use the same
rules for credentials, tokens, and ignored placeholders.

## Rationale

The security audit did not find active secrets, but the repository does not
currently track a `.gitleaks.toml` or `.secretlintrc`. A committed policy makes
future scans repeatable and reduces guesswork around examples, fixtures, and
false positives.

## Plan

- Choose the scanner format for this repo, preferably `.gitleaks.toml` unless
  project tooling points elsewhere.
- Add allowlist rules for obvious placeholders and generated local artifacts.
- Add a documented local command for running the scan.
- Consider adding the scan to CI after local behavior is stable.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `.gitleaks.toml`
- `deno.json`
- `package.json`
- `docs/development.md`

Verified with:

- `deno fmt --check`
- `deno lint`
- `deno task lint:paths`
- placeholder secret-pattern scan with `rg`

Harness update:

- `deno task secrets:scan` and `npm run secrets:scan` now run Gitleaks with the
  versioned repository policy when Gitleaks is installed locally.

Review residuals:

- Gitleaks is not installed in this environment, so the actual scanner command
  was documented but not executed here.

Follow-up:

- None
