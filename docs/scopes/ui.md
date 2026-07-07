# UI Scope

## Responsibilities

- Main window shell — the Capture Dock (ADR-0012)
- Quick-settings popover (delay, pointer, recording HUD)
- Corner thumbnail card after capture (ADR-0013)
- Toast feedback windows (ADR-0014)
- Annotation editor for screenshots (ADR-0018)
- Recording HUD
- Monitor identifier overlays during `Monitor` selection

## Invariants

- Main flow uses one window: the horizontal Capture Dock, floating
  bottom-center on the focused monitor (34px above the edge), sized to content.
- Extra-window whitelist: recording HUD, monitor identification overlays,
  corner thumbnail, toast, countdown, annotation editor
  (ADR-0002 as amended by 0008/0013/0014/0015/0018).
- Post-capture results appear in the corner thumbnail card; with
  `autosave=true` they are already saved, otherwise pinned until Save.
- All transient feedback (success/error) is a toast; the dock has no status line.
- Use GTK4 widgets directly and keep the UI compact and custom.
- Dock keyboard shortcuts: Enter fires, 1/2/3 pick targets, S/R switch mode,
  D cycles delay, P toggles pointer, Esc quits.
- The pointer toggle is disabled in record mode (wf-recorder always records the cursor).
- Thumbnail actions: Annotate · Copy (accent) · Save · Share (screenshots) or
  GIF (recordings) · Discard; close/replace clears a still-pinned temp.
- HUD-off recording still provides a minimal visual reminder through a 300ms red flash every 5s on the target monitor.
- The recording HUD should keep the red dot semantics visible without turning the whole HUD red.
- The recording HUD should read as a compact control bar: red dot accent, neutral `REC` label, readable timer, and clear `Stop` affordance.
- Keep selection visuals semantically distinct (ADR-0011 v2 accents):
  - screenshots use teal `#5EE6D0`
  - recordings use coral `#FF5D5D`
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

### Window background must be explicit

On Hyprland without an explicit background color, GTK4 windows default to transparent — the
compositor renders through them. Always set an explicit `window { background: …; }` in app CSS:
either the solid surface `#14151B` (token `--d-bg-solid`) or the deliberate glass translucency
`rgba(19,20,26,0.72)` via the `hs-glass` window class (ADR-0011). Glass windows additionally call
`hyprland::make_window_glass()` so Hyprland provides blur, shadow, and corner rounding; with
`dock_style = solid` the app stays fully opaque and skips those setprops.

### Monitor identifier overlays

During `Monitor` selection, the UI shows one transient borderless GTK window per monitor,
centered on each screen, displaying that monitor's Hyprland connector name (`DP-1`,
`HDMI-A-1`, …). Window CSS class is `hs-mon-id`; label CSS class is `hs-mon-id-label`. The
connector name renders in a monospaced font on a solid `#0E1116` background with a 2 px
border for visual weight. The capture flow collects the overlay handles and closes each
one as soon as `slurp` resolves (success or cancel). `slurp` runs on a worker thread while
the overlays are presented so the GTK main loop stays responsive — without this, the
compositor flags Hyprscreen as "not responding" mid-selection.
