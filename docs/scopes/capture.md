# Capture Scope

## Responsibilities

- Screenshot execution
- Recording execution
- Temporary files for preview
- Clipboard copy and save handoff from preview

## Implemented Screenshot Tools

- `grim`
- `slurp`
- `hyprctl`
- `wl-clipboard`

## Implemented Screenshot Targets

- `Area`
- `Window`
- `Monitor`

## Selection Styling

- Screenshot mode uses the teal (`#5EE6D0`) selection style for `Area`, `Window`, and `Monitor` (ADR-0011).
- Recording mode uses the coral (`#FF5D5D`) selection style for `Area` and `Monitor` (ADR-0011).
- `Window` and `Monitor` targets use `slurp -r` with predefined rectangles inside a dimmed full-screen overlay.
- `Area` and `Window` use `slurp -w 3`; `Monitor` uses `slurp -w 6` so the full-screen rectangle outlines read at scale.
- `Area` selection uses `slurp -d` to display live drag dimensions.

## Screenshot Monitor Notes

- `Screenshot -> Monitor` uses `slurp -r` with monitor rectangles.
- It follows the screenshot visual style rather than the coral recording style.
- The chosen monitor is captured with `grim -o`.
- Identifier overlays render the Hyprland connector name on each screen during selection (see `docs/scopes/ui.md` and ADR-0008).

## Notes

- Preview files are created in a temporary Hyprscreen directory.
- `Save` copies the temporary file into `~/Pictures/Screenshots`.
- `Copy` sends the PNG to `wl-copy`.
- `New` repeats the last screenshot action.
- Screenshot preview enables `Reveal` only after the file has been saved.

## Current Recording Notes

- `Record -> Area` is implemented with `wf-recorder`.
- `Record -> Monitor` is implemented with `slurp -r` monitor selection and `wf-recorder -o`. Identifier overlays appear during selection, same pattern as the screenshot flow.
- `Record -> Window` is implemented with visible-window selection and `wf-recorder -g`.
- Recording preview thumbnails are generated with `ffmpeg`; metadata such as duration, resolution, and file size is extracted from `ffprobe` and the file itself.
- Recording preview files are temporary until `Save` copies them into `~/Videos/Recordings`.
- `Open` and `Reveal` are disabled for recordings until the file has been saved.
- `hyprscreen stop` stops the current recording via a runtime state file.
- `hyprscreen screenshot <target>` and `hyprscreen record <target>` are CLI entry points into the capture flow — they open the GUI and auto-trigger the action (see ADR-0009).

## Planned Recording Targets

- None for v0 target coverage; embedded playback is next.
