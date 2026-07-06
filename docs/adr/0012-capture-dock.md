# 0012 - Horizontal Capture Dock Replaces the Vertical Setup Window

## Status

Accepted

## Context

The v1 main window was a 400px-wide vertical panel (mode toggle, target grid,
HUD switch, CTA, status line) centered on screen. The v2 design reshapes the
main surface into a horizontal command bar — the "Capture Dock" — floating
bottom-center: mode segment (Shot/Rec), target icon buttons with an
active-accent dot, a delay chip, a pointer toggle, a quick-settings popover,
and the primary Capture/Record button with a keyboard hint.

## Decision

- The main window becomes the dock. This does **not** touch ADR-0001's
  single-window rule: it is the same one main window with a new shape. The
  fixed 400px width invariant is retired; the dock sizes to content.
- The dock floats bottom-center on the **focused monitor**, 34px above the
  bottom edge, positioned via `hyprland::place_window_exact` after mapping
  (`position_dock`, re-applied on every map).
- Quick settings is a `gtk::Popover` on a `gtk::MenuButton` (not a window):
  delay presets (Now/3s/5s/10s), show-pointer switch, and — in record mode —
  the Recording HUD switch that used to live on the setup page.
- The delay chip cycles 0 → 3 → 5 → 10s and mirrors the popover presets. The
  delay runs **after** selection (the countdown overlay lands with ADR-0015).
- The pointer toggle maps to `grim -c` and is disabled in record mode
  (wf-recorder always records the cursor).
- In-app keyboard shortcuts (dock focused): Enter fire, 1/2/3 targets,
  S/R mode, D delay cycle, P pointer, Esc quit.
- Mode accenting extends the class mechanism: the window and the popover carry
  `hs-mode-shot` / `hs-mode-rec`; icon textures are re-baked on mode change
  (GTK CSS cannot recolor rasterized SVGs).
- **Transitional**: the preview page remains a second stack page inside the
  dock window (it grows the window when shown) and a slim status line renders
  under the dock bar. Both are removed by the corner-thumbnail + toast work
  (ADR-0013/0014).

## Consequences

- CLI auto-trigger (ADR-0009) is unchanged: the dock presents, the startup
  action toggles the right controls and fires after ~120ms.
- The dock hides during selection and recording exactly as the old window did.
- `docs/scopes/ui.md` drops the 400px invariant and documents the dock layout.
