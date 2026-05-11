# 0007 - Preview Back Clears Temporary Result

## Status

Accepted

## Context

`Back` and `Discard` created overlapping meanings in preview, and users could not clearly tell the difference between them.

## Decision

Remove `Discard` from preview. `Back` now clears the temporary preview result and returns to `Setup`.

## Consequences

- Preview actions are reduced to `Back`, `New`, `Save`, and one contextual action.
- Saved files remain on disk when leaving preview.
- Temporary files are removed when the user leaves preview with `Back`.
