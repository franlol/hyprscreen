# 0015 - Delay Countdown as a Centered Per-Monitor Overlay

## Status

Accepted

## Context

The v2 design shows a fullscreen dimmed countdown (large number in a circle,
"Cancel · Esc") after the user fires a delayed capture. Two properties of a
fullscreen implementation are wrong for us: a fullscreen window grabs input
across the whole monitor, and it risks appearing in the capture if the
compositor hasn't repainted when grim/wf-recorder start.

## Decision

- The countdown is a **centered ~260px transparent-background window**
  (`Hyprscreen Countdown`, whitelisted extra window) on the **target
  monitor**: the monitor containing the selection's center (screenshots), the
  recording placement (recordings), or the selected output. It shows the tick
  number in an accent-tinted circle and a "Cancel · Esc" pill.
- **Order of operations**: fire → dock hides → selection (slurp) → countdown
  → capture/record. The delay counts against the already-chosen geometry.
- On reaching zero the overlay closes, the display syncs, and the capture
  fires after a 120ms compositor-settle delay so the overlay never leaks into
  the result.
- Esc (the overlay takes focus on present) or the Cancel pill aborts: the
  overlay closes, the dock re-presents, and the fire button re-enables.
- Approximations vs the mock, accepted deliberately: no fullscreen dim, no
  per-tick pop animation (GTK CSS animation restarts are unreliable).
- Related approximations recorded here: slurp renders the selection UI, so
  the mock's dashed border, corner handles, and window-pick title chip are
  not implemented — slurp is retinted to the mode accent instead (ADR-0011).

## Consequences

- `capture_delay_seconds` (default 0) seeds the dock's delay chip;
  `show_pointer` (default true) seeds the pointer toggle.
- The countdown window must never be fullscreen or grab input beyond itself.
