# 0008 - Monitor Identifier Overlays

## Status

Accepted

## Context

The `Monitor` target shows a `slurp -r` rectangle per screen, but the user has no
visual cue indicating which physical screen each rectangle belongs to. Hyprland
connector names (`DP-1`, `HDMI-A-1`, `eDP-1`) are the canonical OS identifier and
are the same names the user already sees in `hyprctl` and config files.

## Decision

During `Monitor` selection (both screenshot and recording), render one transient
borderless GTK window per monitor, centered on each screen, displaying that
monitor's Hyprland connector name. Close all of them as soon as `slurp` resolves,
whether the user picked a monitor or cancelled.

The overlays use CSS classes `hs-mon-id` (window) and `hs-mon-id-label` (label).

## Consequences

- Amends ADR-0002: the recording HUD is no longer the only allowed extra window.
  Transient identification overlays are also permitted, scoped to the selection
  phase.
- `slurp` is invoked on a worker thread while the overlays are presented; the
  result is polled back to the GTK main loop. This is required because the GTK
  main thread must remain free to answer Wayland `wl_display` pings — otherwise
  the compositor flags Hyprscreen as "not responding" while the user is still
  picking a screen.
- `src/hyprland/mod.rs` exposes `enumerate_monitors()` so the UI layer can place
  overlays without going through the capture module.
