# Switch license to MIT

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

Change the repository license to MIT and confirm that dependency licenses are
compatible with that distribution choice.

## Rationale

The repository currently has no `LICENSE` file, and package metadata still marks
the package as `UNLICENSED` in `package.json` and `jsr.json`.

## Plan

- Add a repository-level `LICENSE` file containing the MIT License text.
- Update package metadata license fields from `UNLICENSED` to `MIT`.
- Review direct dependency licenses and record any incompatible, unknown, or
  policy-sensitive entries before publishing under MIT.
- Check documentation or publish notes for any stale references to the package
  being unlicensed.

Completed: 2026-05-16

## Resolution

Implemented by updating:

- `LICENSE`
- `package.json`
- `jsr.json`
- `docs/development.md`

Verified with:

- `deno task lint:paths`
- `deno task publish:jsr:dry-run`
- `deno task publish:npm:dry-run`

Harness update:

- Package publish dry-runs include the MIT metadata and `LICENSE` file.

Review residuals:

- None

Follow-up:

- None
