# 0010 - GIF Export as a Post-Recording Preview Action

## Status

Accepted

## Context

Users want to save a recording as a GIF. The capture pipeline records video with
`wf-recorder` to a temporary `.mkv`, which the preview page already holds before
`Save`. A GIF is necessarily a re-encode of that captured video. Two placements
were viable:

- **(a) Pre-record toggle** — choose "record as GIF" before recording; on stop,
  auto-convert and present the GIF in the preview.
- **(b) Post-record action** — keep recording untouched; add a `GIF` button to
  the recording preview that converts the temp MKV on demand.

## Decision

Pick **(b)**. A `GIF` action button appears on the preview page for recordings
only (hidden for screenshots). Clicking it runs
`record::convert_recording_to_gif` and then makes the resulting GIF the **active
preview artifact** (`load_preview_gif`), replacing the recording. The GIF does
**not** auto-save; the standard `Save` / `Open` / `Reveal` lifecycle then applies
to it identically to a recording. `Save` writes it into `save_dir_recordings`.

The button does not save on its own because that diverged from every other
preview action (which are single-purpose) and left `Open` / `Reveal` pointing at
the now-gone recording — the GIF was unreachable through the normal flow.

- Conversion uses `ffmpeg` (already a dependency for the recording thumbnail)
  with the palettegen/paletteuse filter for acceptable colour quality within
  GIF's 256-colour limit.
- Frame rate and maximum width are config-driven: `gif_fps` (default 15) and
  `gif_max_width` (default 800), parsed leniently like the rest of config.
  `scale='min(<max_w>,iw)':-2` never upscales and keeps even dimensions.
- The conversion is CPU-bound, so it runs on a worker thread with a
  "Converting…" status, polled back on the GTK main loop — the same
  worker-thread + `glib::timeout_add_local` pattern used by capture/recording.

## Consequences

- Recording behaviour is unchanged; GIF is derived per-clip when the user asks.
- Accepted GIF limitations: large files (mitigated by the fps cap and width
  scale), no audio, and 256 colours per frame.
- Long recordings take noticeable time to convert; the UI stays responsive
  because the work is off the main loop.
- GIF is always a re-encode of the captured MKV. A future "record straight to
  GIF" or format-selection flow would amend this ADR.
