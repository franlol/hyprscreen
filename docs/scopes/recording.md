# Recording Scope

## Responsibilities

- Recording start/stop flow
- HUD coordination
- Recording preview handoff

## Current Recording Slice

- `Record -> Area`
- `Record -> Window`
- `Record -> Monitor` (identifier overlays show each screen's Hyprland connector name during selection — see ADR-0008)
- optional HUD window with a stop button
- `hyprscreen record <target>` as a CLI start path (opens the GUI and auto-triggers — see ADR-0009)
- `hyprscreen stop` as a CLI stop path
- temporary video file before `Save` or `Back`
- preview actions: `Open` and `Reveal` only after `Save`
- `Open` follows `open_video_command`, then common video players
- Recording preview generates a thumbnail and metadata such as duration, resolution, and file size before save
- When `show_recording_hud=false`, a small red recording indicator flashes for 300ms every 5s on the recording monitor

## Invariants

- Hidden HUD requires a verified stop shortcut.
- Visible HUD can provide visual stop controls.

## Current Status

Recording `Area`, `Window`, and `Monitor` are implemented.

- `Record -> Window` currently records the selected window geometry and does not follow moved windows.
- Video preview currently uses a generated thumbnail and metadata rather than embedded playback.

Screenshot flow remains the reference for general hide/show timing and preview handoff.
