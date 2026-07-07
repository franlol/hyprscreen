# 0019 - Webcam Bubble via GStreamer (Feature-Gated)

## Status

Accepted (amends 0002)

## Context

The v2 design includes a circular talking-head overlay during recordings.
Rendering a live camera feed needs a media pipeline; GStreamer with the
`gtk4paintablesink` element is the canonical GTK4 path. This is the heaviest
dependency decision in the project, so it must be strippable.

## Decision

- Cargo feature **`webcam`** (default on, strip with
  `--no-default-features`) pulls the single `gstreamer` crate. The pipeline
  is `v4l2src device=<webcam_device> ! videoconvert ! gtk4paintablesink`;
  the sink's `paintable` property feeds a `gtk::Picture`.
  - v4l2 over pipewiresrc: no portal negotiation, one config key; PipeWire
    is possible future work.
  - `gst-plugin-gtk4` is a **runtime** requirement (packaging note); if it
    or the camera is missing, the bubble degrades to an explanatory error
    toast — the recording continues unaffected.
- Window `Hyprscreen Camera` (whitelisted): `webcam_size` px square
  (default 132), placed at `webcam_position` (default bottom-left, 34px
  margins) on the recorded monitor, pinned above the stack, with a LIVE
  badge. **Hyprland's corner rounding (size/2) makes the surface circular,**
  so wf-recorder captures a round bubble — GTK-side clipping is unnecessary.
- Drag anywhere on the bubble moves it (relative `movewindowpixel`).
- Starts automatically with a recording when `webcam_enabled = true`, or on
  the HUD cam button; recording stop closes it.

## Consequences

- Packaging gains an optional runtime dependency on `gst-plugin-gtk4`
  (and gstreamer core) for the default build.
- The bubble is part of the recorded image by design; disable it per
  recording via the HUD cam button.
