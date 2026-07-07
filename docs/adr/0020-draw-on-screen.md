# 0020 - Draw-on-Screen: Modal Monitor Overlay, No Layer-Shell

## Status

Accepted (amends 0002)

## Context

The v2 full HUD has a "draw on screen" control: freehand strokes visible in
the recording. wf-recorder captures the screen, so anything rendered in an
on-screen window lands in the video — no encoder integration needed. The
open question was the window mechanism: gtk4-layer-shell (a new dependency
the app has deliberately avoided) versus a plain floating window.

## Decision

- A plain **monitor-sized transparent floating GTK window**
  (`Hyprscreen Draw`, whitelisted), positioned at the recorded monitor's
  origin and **pinned** (`hyprctl dispatch pin`) so it stays above the stack.
  No gtk4-layer-shell: layer-shell would not change the fundamental
  trade-off anyway — drawing requires pointer input, so the overlay is
  modal on its monitor while active by nature.
- Strokes are freehand polylines rendered by cairo: 3.5px round-cap coral
  (recording accent). Ctrl+Z removes the last stroke.
- The HUD draw button toggles the overlay; Esc (on the overlay) also exits.
  **Strokes clear on exit** — the overlay is a live telestrator, not an
  annotation layer. Recording stop force-closes it.
- The HUD stays reachable because it is presented after the overlay and
  Hyprland keeps recent floats above; this is best-effort stacking, and the
  overlay's Esc always works.

## Consequences

- While drawing, clicks do not reach the applications being recorded.
- The overlay itself appears in the recording — that is the point — so it
  must never render anything except strokes (fully transparent otherwise).
