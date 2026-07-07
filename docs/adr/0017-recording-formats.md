# 0017 - Recording Formats at Capture Time (MP4/WEBM); GIF Stays Post-Capture

## Status

Accepted (reaffirms 0010)

## Context

The v2 quick-settings popover offers a recording format choice. wf-recorder
finalizes whatever container the output path implies on SIGINT, so recording
directly into the target container removes the old always-MKV intermediate.

## Decision

- Recordings write directly to the chosen container: **MP4** (wf-recorder's
  default libx264) or **WEBM** (`-c libvpx-vp9`, `-C libopus` when audio is
  on). Selected in the popover (record mode) or via `recording_format`.
- Pause/resume segments share one container; concat stays within it.
- **GIF is intentionally absent from the format picker** (the mock shows it):
  ADR-0010 stands — GIF remains a post-capture conversion on the thumbnail
  card, where it replaces the Share action for recordings.
- Audio: `record_audio` + optional `audio_device` map to
  `-a` / `--audio=<device>`.
- The mock's "Encoder unavailable → Use CPU" toast is not implemented:
  hyprscreen never requests a hardware encoder (libx264/libvpx are CPU
  codecs), so the failure mode it addresses cannot occur here.

## Consequences

- The `.mkv` intermediate is gone; temp files carry their final extension.
- `hud_style = full|compact` selects the extended HUD (pause/restart/mic/
  stop, plus placeholders for webcam and draw) or the v1-style pill.
