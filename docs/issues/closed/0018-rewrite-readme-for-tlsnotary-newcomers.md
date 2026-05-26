# Rewrite README for TLSNotary newcomers

Created: 2026-05-16
Model: GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

Rewrite the README so readers without TLSNotary background can quickly
understand what `tlsn-fetch` can do, why it is useful, and how to run it.

The README should lead with the user-facing outcome before implementation
details: fetch HTTPS data, produce a verifiable proof, selectively reveal
response data, and verify the proof later without trusting the fetcher.

## Rationale

The current README starts by naming TLSNotary, the Deno CLI, and Rust sidecars.
That is accurate for contributors, but it does not first answer the onboarding
questions a new user has:

- What can I prove with this tool?
- Why is this different from copying an API response or screenshot?
- What commands do I run to create and verify a proof?

The docs should preserve the existing command reference while adding a clearer
mental model and a guided first-run path.

## Plan

- Add a short opening section that explains `tlsn-fetch` in product terms.
- Add a "why this exists" section describing the trust problem it solves.
- Rework Quick Start so each command states what it produces and what success
  looks like.
- Keep sidecar, package-manager, direct executable, and fetch option details
  available after the newcomer-oriented flow.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `README.md`

Verified with:

- `deno task lint:paths`

Harness update:

- None - documentation-only onboarding rewrite.

Review residuals:

- None

Follow-up:

- None
