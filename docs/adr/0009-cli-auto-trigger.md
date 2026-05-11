# 0009 - CLI Subcommands Open the GUI and Auto-Trigger

## Status

Accepted

## Context

`hyprscreen screenshot <target>` and `hyprscreen record <target>` exist so
users can bind specific captures to keys without clicking through the GUI.
Two implementations were viable:

- **(a) Headless CLI** — the subcommands bypass the GUI entirely, calling
  `screenshot::capture_*` or `record::start_*` directly and exiting (or
  detaching, for recordings).
- **(b) GUI auto-trigger** — the subcommands open the existing GUI, preset
  the Mode/Target toggles, and programmatically fire the `Capture` button.

## Decision

Pick **(b)**. CLI subcommands open the GUI exactly as `hyprscreen` would.
A short `glib::timeout_add_local_once` (≈120 ms) runs in `build_setup_page`
after the window is presented; the closure flips the appropriate Mode and
Target toggles, then calls `cta_button.emit_clicked()`. The existing CTA
click handler runs unchanged.

## Consequences

- One code path for capture and recording. Anything that ships from the GUI
  ships from the CLI for free (preview page, HUD, monitor identifier
  overlays, status messages).
- The main window is briefly visible during a CLI-driven capture before the
  existing pre-capture hide step takes effect. Acceptable for v0.1.
- Headless captures (no GUI at all) are foreclosed by this ADR. If a future
  use case requires it — e.g. CI screenshotting or scripting without any
  visible window — a new ADR must amend this one.
- The 120 ms delay is the gate between window present and trigger fire.
  Tunable in `src/ui/main_window.rs::build_setup_page` if Hyprland window
  mapping ever races the trigger.
