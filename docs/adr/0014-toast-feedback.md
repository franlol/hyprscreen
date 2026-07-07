# 0014 - Toast Feedback Windows

## Status

Accepted (amends 0002)

## Context

With the preview page and its status line gone (ADR-0013), success and error
feedback needs a surface that exists independently of any card — copy from a
card, save errors, capture failures, and recording errors can all fire when
nothing else is on screen.

## Decision

- One transient `Hyprscreen Toast` window, bottom-center above the dock,
  joins the extra-window whitelist. Single-slot: a new toast replaces the
  current one.
- Kinds: `ok` (green check chip, 4s lifetime) and `err` (coral alert chip,
  6s). An optional action button rides on the right (`Reveal`, `Retry`,
  later `Use CPU` / `Settings`).
- Wired today: Copy/Share/Save successes (Save carries `Reveal`), Discard
  confirmation, capture/recording failures (carry `Retry`, which re-fires the
  dock's primary button with current dock state), open/reveal errors.
- No toast fires while a recording is running with feedback that would land
  in the captured area — recording start intentionally emits nothing (the
  HUD or flash indicator covers it, honoring ADR-0004's reminder duty).

## Consequences

- The dock carries no status line; all transient feedback is a toast.
- Toasts are windows and can appear in captures if mistimed; wiring must keep
  them out of the capture path (they only fire after present()).
