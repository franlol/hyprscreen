# 0011 - v2 Visual Language and Glass Strategy

## Status

Accepted

## Context

The v2 "Capture Dock" design replaces the v1 palette (cold-white screenshot
accent `#E5ECF5`, red record accent `#E5484D`, opaque `#16181E` surfaces) with
a token-driven dark theme: teal (`#5EE6D0`) for screenshot mode, coral
(`#FF5D5D`) for record mode, and translucent glass surfaces
(`rgba(19,20,26,0.72)` with heavy background blur). The mock relies on CSS
`backdrop-filter`, which GTK4 does not implement, and on `::after`
pseudo-elements, which GTK4 CSS does not support either.

## Decision

- **Tokens are literal values** in the single inline stylesheet
  (`src/app/mod.rs`). A comment block at the top of the stylesheet maps the
  design token names (`--d-*`) to their values; GTK CSS has no usable `var()`,
  so selectors use the literals directly.
- **Mode accents switch via CSS classes**, extending the existing
  `mode-shot` / `mode-rec` class-toggling mechanism. Accent-dependent rules are
  duplicated per mode class.
- **Glass comes from Hyprland, not GTK**: windows that want the glass look get
  a translucent GTK background (`window.hs-glass`,
  `rgba(19,20,26,0.72)`) and `hyprland::make_window_glass()`, which uses
  `hyprctl setprop` to re-enable Hyprland's blur and shadow and set corner
  rounding. If the user's Hyprland has `decoration:blur` disabled, the window
  degrades to a flat translucent panel — still legible.
- **Solid fallback**: `dock_style = solid` in `hyprscreen.conf` renders opaque
  `#14151B` surfaces and skips the blur setprops entirely.
- The former UI rule "window background must be a solid color" becomes
  "window background must be **explicit**": either solid `#14151B` or the
  deliberate glass rgba — never unset/transparent by accident.
- **Selection retint**: `slurp` runs with the mode accent
  (`#5ee6d0ff` / `#ff5d5dff` border, `…1a` soft fill, `#06080c6b` dim), so the
  selection overlay carries the same semantics as the rest of the UI. The
  former white-vs-red distinction is now teal-vs-coral.
- **Icons are single-source SVGs** authored with `stroke="currentColor"`;
  `icon_image_colored()` bakes a concrete color in before rasterization
  (librsvg has no CSS context for currentColor), defaulting to `#EDEEF2`.

## Consequences

- Widgets that need pseudo-element affordances from the mock (active dots,
  tooltips, popover arrows) use real child widgets or native GTK nodes instead.
- Accent changes touch exactly one file (the stylesheet) plus any icon call
  sites that bake accent foregrounds.
- Glass correctness depends on Hyprland decoration settings; the app must not
  assume blur is active.
