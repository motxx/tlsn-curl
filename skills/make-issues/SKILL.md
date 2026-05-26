---
name: make-issues
description: Create repository-tracked Markdown issues under `docs/issues/pending/`. Use when the user says `make-issues`, asks to file/create/add docs issues, wants a backlog split into tracked work items, or asks to convert findings/TODOs/plans into `docs/issues` entries. This skill updates `docs/issues/SEQUENCE`, writes zero-padded issue files, and follows `docs/issues/README.md`.
argument-hint: "<issue request, findings, TODOs, or plan>"
---

# Make Issues

Create one or more file-based issues in `docs/issues/pending/` and update
`docs/issues/SEQUENCE`.

## Workflow

1. Read `docs/issues/README.md` and `docs/issues/SEQUENCE`.
2. List `docs/issues/pending/*.md` and `docs/issues/closed/*.md`.
3. Determine the next number:
   - Parse `SEQUENCE` as the last allocated number.
   - Also parse existing issue filenames.
   - Use `max(sequence, highest_existing_number) + 1`.
   - Update `SEQUENCE` to the last number created.
4. Turn the user's request into scoped issues:
   - Prefer one issue per independently closeable change.
   - Keep each issue concise.
   - If the request is too vague, inspect local context before asking.
5. Write each issue as `docs/issues/pending/NNNN-short-title.md`.
6. Run `deno task lint:paths`.
7. Report the created paths and numbers.

## Issue Format

Use this shape unless `docs/issues/README.md` changes:

```markdown
# Short title

Created: YYYY-MM-DD
Model: <current agent/model name>

## Priority

<bug | feature | design | maintenance | investigation>

## Dependencies

Depends on:
- None

Blocks:
- None

## Summary

What needs to change, in a few lines.

## Rationale

Only the necessary background.

## Plan

- Concrete next step
```

## Rules

- Priority is one of `bug`, `feature`, `design`, `maintenance`,
  `investigation`.
- Number is four digits, zero-padded.
- Slug is lowercase ASCII kebab-case.
- Do not reuse numbers.
- Do not include private keys, credentials, private proofs, personal data,
  fund-bearing details, or unpatched vulnerability details.
- Do not include developer-local absolute paths. Use repo-relative paths or
  placeholders such as `<repo-root>`.
- Do not create GitHub Issues unless the user explicitly asks.
