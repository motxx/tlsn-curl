# Issues

This repository tracks issues in `docs/issues` instead of GitHub Issues. The
layout follows the file-based issue style used by
<https://github.com/shiguredo/http3-rs/tree/develop/issues>.

## Layout

- `pending/`: open issues.
- `closed/`: completed issues.
- `SEQUENCE`: the last allocated issue number.

Issue files use a zero-padded sequence number and a short kebab-case title:

```text
docs/issues/pending/0001-short-title.md
```

Numbers are never reused, even when an issue is closed or abandoned.

## Creating an Issue

1. Read `docs/issues/SEQUENCE` and the existing issue filenames.
2. Allocate the next number as `max(SEQUENCE, highest existing issue number) + 1`.
3. Update `SEQUENCE` to the last number created.
4. Add a Markdown file under `pending/` using the new number.
5. Keep the issue concise.
6. Keep the issue scoped enough to close in one change when possible.
7. If the issue cannot be resolved until another issue is closed, record that
   prerequisite under `Depends on`.
8. If later work should wait for this issue, record those issue numbers under
   `Blocks`.
9. Run `./scripts/check-no-local-paths.sh` before finishing so local filesystem
   paths are not captured in issue text.

Use this structure:

```text
# Short title

Created: YYYY-MM-DD
Model: <model name, if created with an agent>

## Priority

<bug | feature | design | maintenance | investigation>

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

What needs to change, and why.

## Rationale

Links, code references, logs, threat-model notes, or compatibility constraints.

## Plan

- Concrete next step
- Another concrete next step
```

Priority must be one of:

- `bug`: incorrect behavior or regression.
- `feature`: new user-visible or developer-facing capability.
- `design`: architecture, API, UX, protocol, or product decision work.
- `maintenance`: cleanup, refactor, docs, tooling, dependency, or test debt.
- `investigation`: unknown root cause or research needed before implementation.

## Closing an Issue

Move the file from `pending/` to `closed/`, keep the same filename, and add:

```text
Completed: YYYY-MM-DD

## Resolution

Implemented by updating:

- `path/to/file`

Verified with:

- `command that was run`

Harness update:

- <which test, lint, spec, or documentation update absorbs this class of
  finding>; or
- None - <one-line rationale for why no harness update was needed>.

Review residuals:

- None; or
- <human decision left after verification, with the owning document or pending
  issue number>.

Follow-up:

- None
```

Do not renumber files when moving them. If the work reveals a separate task,
create a new issue with a new number.

If the issue has unresolved `Depends on` entries, do not close it unless the
maintainer explicitly accepts the remaining dependency risk. When closing an
issue, update other pending issues whose `Depends on` or `Blocks` lists should
change as a result.

## Security

Do not put private keys, proofs containing personal data, credentials,
fund-bearing details, or unpatched vulnerability details in `docs/issues`.
Coordinate privately with maintainers before filing security-sensitive work.
Do not include developer-local absolute paths. Use repo-relative paths, named
external repository references, or placeholders such as `<repo-root>`.
