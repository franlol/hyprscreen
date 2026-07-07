# Recording Scope

## Responsibilities

- Recording start/stop flow, pause/resume segmentation (ADR-0016)
- HUD coordination (full control bar or compact pill — `hud_style`, ADR-0017)
- Recording handoff to the corner thumbnail card (ADR-0013)

## Current Recording Slice

- `Record -> Area`
- `Record -> Window`
- `Record -> Monitor` (identifier overlays show each screen's Hyprland connector name during selection — see ADR-0008)
- Formats: MP4 (libx264) or WEBM (libvpx-vp9 + libopus), chosen in the quick-settings popover or `recording_format` (ADR-0017)
- Audio: `record_audio` / popover switch → `wf-recorder -a` (or `--audio=<audio_device>`); the HUD mic button applies at the next pause/resume boundary
- Pause/Resume/Restart on the full HUD via segmented recording + lossless `ffmpeg -c copy` concat on stop (ADR-0016)
- `hyprscreen record <target>` as a CLI start path (opens the GUI and auto-triggers — see ADR-0009)
- `hyprscreen stop` as a CLI stop path; while paused it leaves a stop-request sentinel the GUI poll consumes
- Finished recordings land in the corner thumbnail (auto-saved by default); its `GIF` action converts post-capture per ADR-0010, tuned by `gif_fps` / `gif_max_width`, off the GTK main loop
- Thumbnail card shows duration, resolution, and file size from ffprobe
- When `show_recording_hud=false`, a small red recording indicator flashes for 300ms every 5s on the recording monitor

## State file

`$XDG_RUNTIME_DIR/hyprscreen-recording.json` holds `{ pid, temp_path }` of the
current segment; `pid` is `0` while paused. A CLI stop writes
`hyprscreen-stop-requested` alongside it.

## Invariants

- Hidden HUD requires a verified stop shortcut.
- Visible HUD can provide visual stop controls.
- Pause gaps (~100–300ms of lost footage per pause) are accepted; segments are
  never re-encoded on join.

## Current Status

Recording `Area`, `Window`, and `Monitor` are implemented.

- `Record -> Window` currently records the selected window geometry and does not follow moved windows.
- The thumbnail uses a generated poster frame rather than embedded playback.
- HUD cam and draw buttons are placeholders until the webcam bubble and
  draw-on-screen phases land.

Screenshot flow remains the reference for general hide/show timing.
