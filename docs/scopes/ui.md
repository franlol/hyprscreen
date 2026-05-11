# UI Scope

## Responsibilities

- Main window shell
- Setup page
- Preview page
- Recording HUD
- Monitor identifier overlays during `Monitor` selection

## Invariants

- Main flow uses one window.
- Extra windows are limited to the recording HUD and transient identification overlays during selection.
- `Preview` is an internal state.
- Use GTK4 widgets directly and keep the UI compact and custom.
- `Preview` uses `Back`, `New`, `Save`, one contextual action (`Copy` or `Open`), and `Reveal`.
- `Back` must clear temporary preview data before returning to `Setup`.
- The setup page should avoid persistent explanatory copy in normal states; use the status line only for validation, feedback, and errors.
- HUD-off recording still provides a minimal visual reminder through a 300ms red flash every 5s on the target monitor.
- The recording HUD should keep the red dot semantics visible without turning the whole HUD red.
- The recording HUD should read as a compact control bar: red dot accent, neutral `REC` label, readable timer, and clear `Stop` affordance.
- Keep selection visuals semantically distinct:
  - screenshots use cold white
  - recordings use red
- `Window` and `Monitor` selection should preserve the same visual language as the active mode.

## GTK4 Implementation Notes

### Icon sizing — use `gtk::Image`, not `gtk::Picture`

`gtk::Picture::for_file()` with `can_shrink(false)` renders an SVG at its *natural* size.
librsvg derives natural size from the `viewBox` attribute (e.g. `0 0 24 24`), so the SVG
`width`/`height` attributes and `set_size_request()` are both ignored.

Use `gtk::Image::from_gicon(&gio::FileIcon::new(&file))` + `image.set_pixel_size(n)` instead —
this is GTK4's designed path for pixel-exact icon rendering.

In `src/ui/main_window.rs` this is encapsulated in the `icon_image()` helper (target buttons 22 px,
action buttons 16 px, primary shutter 16 px).

### Label height — use `line-height: 1`

Pango adds ascent + descent line metrics on top of `font-size`, making a 12 px label ~14–15 px
tall. GTK4 4.22+ supports the `line-height` CSS property. Set `line-height: 1` on any label whose
pixel height feeds into a button-height calculation (`.hs-tbtn-label`, `.hs-abtn-label`).

### Button CSS overrides require `!important`

GTK4's embedded theme CSS sets `button { min-height: 24px; padding: 4px 9px; }` at
`STYLE_PROVIDER_PRIORITY_APPLICATION` (600). Our CSS provider runs at
`STYLE_PROVIDER_PRIORITY_USER` (800) and should win by cascade, but specificity interactions with
the theme's own class selectors can still lose. Use `!important` on `min-height` and `padding` to
guarantee the override regardless of specificity.

### Window background must be solid

On Hyprland without an explicit background color, GTK4 windows default to transparent — the
compositor renders through them. Always set `window { background: #<solid>; }` in app CSS.
Current value: `#16181E` (design token `--hs-surface-solid`).

### Monitor identifier overlays

During `Monitor` selection, the UI shows one transient borderless GTK window per monitor,
centered on each screen, displaying that monitor's Hyprland connector name (`DP-1`,
`HDMI-A-1`, …). Window CSS class is `hs-mon-id`; label CSS class is `hs-mon-id-label`. The
connector name renders in a monospaced font on a solid `#0E1116` background with a 2 px
border for visual weight. The capture flow collects the overlay handles and closes each
one as soon as `slurp` resolves (success or cancel). `slurp` runs on a worker thread while
the overlays are presented so the GTK main loop stays responsive — without this, the
compositor flags Hyprscreen as "not responding" mid-selection.
