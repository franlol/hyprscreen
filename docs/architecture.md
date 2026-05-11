# Architecture

## High-Level Modules

- `app`: application bootstrap
- `ui`: main window, setup state, preview state, HUD
- `capture`: screenshots and recordings
- `hyprland`: Hyprland integration and validation
- `config`: config parsing and defaults
- `cli`: app commands and command routing

## Core Flow

1. User selects `Screenshot` or `Record`.
2. User selects `Area`, `Window`, or `Monitor`.
3. Main window hides before capture starts.
4. Capture is deferred briefly so Hyprscreen disappears from the scene before tools run.
5. Main window returns to the `Preview` state.

## Current Screenshot Flow

1. `Area` uses `slurp -d` (live dimensions) followed by `grim -g`.
2. `Monitor` uses `slurp -r` with one selectable rectangle per monitor, then captures the chosen monitor with `grim -o`. Identifier overlays show each screen's Hyprland connector name during selection. `slurp` runs on a worker thread so the GTK main loop can keep answering Wayland pings while the overlays are visible.
3. `Window` builds predefined selectable rectangles from Hyprland client geometry and selects among visible windows using `slurp -r`.
4. Preview actions operate on temporary files before the user decides to save or return to setup.

## Selection Styling

- Screenshot selection uses a dimmed overlay with a cold white border and subtle white fill.
- Recording selection uses a dimmed overlay with a red border and subtle red fill.
- `Window` and `Monitor` selection use `slurp -r` with predefined rectangles, so the whole screen is dimmed while selectable regions remain visually emphasized.
- `Area` and `Window` use `slurp -w 3`; `Monitor` uses `slurp -w 6`. The wider stroke is deliberate — full-screen rectangles need a heavier outline to read at scale.

## Current Recording Flow

1. `Area` uses `slurp -d` with a red recording-themed selection border.
2. `Monitor` uses `slurp -r` with one selectable rectangle per monitor, then records the chosen monitor by output name. Identifier overlays show each screen's Hyprland connector name during selection, and `slurp` runs on a worker thread to keep the GTK main loop responsive.
3. `Window` currently reuses visible-window filtering and records the selected geometry with `wf-recorder -g`.
4. `wf-recorder` writes into a temporary file.
5. An optional HUD shows a timer and a `Stop` button.
6. `hyprscreen stop` can stop the current recording from CLI or external binds. `hyprscreen record <target>` (and `hyprscreen screenshot <target>`) start a capture from CLI by opening the GUI and programmatically firing the CTA after a short delay — see ADR-0009.
7. Recording preview treats `Reveal` as unavailable until the file has been saved.
8. Recording `Open` uses config-first command resolution similar to `Reveal`, but targets video players instead of file managers.
9. Recording preview generates a thumbnail and metadata such as duration, resolution, and file size using `ffmpeg`, `ffprobe`, and file metadata.
10. When the HUD is hidden, a small red recording indicator flashes for 300ms every 5s on the target monitor.

## Recording Backend Direction

- `Area` and `Monitor` continue to use the direct `wf-recorder` backend.
- `Window` currently uses the stable geometry-based recording path.

## Preview Behavior

- `Back` clears the temporary result and returns to `Setup`.
- `New` repeats the last screenshot or recording action instead of returning to `Setup`.
- `Save` persists the current temporary file into a user-facing directory.
- Preview exposes `Reveal` for both screenshots and recordings after `Save`.
- Preview uses `Copy` for screenshots and `Open` for recordings as the contextual action.
- Preview width is kept stable even when status messages change length.

## Important Constraint

The main app flow stays in one window. Only the recording HUD may use a separate window.

## Runtime Config

- The app reads `~/.config/hyprscreen.conf`.
- Config controls UI defaults, save directories, naming, recording reminder cadence, and integration commands.
- Invalid config values fall back silently to internal defaults.
