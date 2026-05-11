# 0006 - New Repeats Last Action

## Status

Accepted

## Context

In preview mode, users often want to repeat the same screenshot action immediately without returning to setup and reselecting the same target.

## Decision

`New` in `Preview` repeats the last executed screenshot action.

## Consequences

- `Back` still returns to `Setup`.
- `Discard` still clears the current result and returns to `Setup`.
- The UI stores a small `LastAction` state so preview actions can re-run the previous screenshot target.
