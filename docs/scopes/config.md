# Config Scope

## Format

- File: `~/.config/hyprscreen.conf`
- Syntax: `key=value`
- Grouped by commented scopes

## Current Keys

- `default_mode`
- `default_target`
- `show_recording_hud`
- `recording_indicator_enabled`
- `recording_indicator_interval_seconds`
- `recording_indicator_duration_ms`
- `save_dir_screenshots`
- `save_dir_recordings`
- `open_video_command`
- `reveal_folder_command`
- `filename_prefix`
- `timestamp_format`

## Current Behavior

- Missing config falls back to internal defaults.
- `default_mode` and `default_target` define the initial UI selection.
- `show_recording_hud` defines the default state of the recording HUD switch.
- The recording indicator cadence is controlled by `recording_indicator_enabled`, `recording_indicator_interval_seconds`, and `recording_indicator_duration_ms`.
- `save_dir_screenshots` and `save_dir_recordings` define where saved files are written.
- `open_video_command` is used by recording preview when the user presses `Open`.
- `reveal_folder_command` is used when the user presses `Reveal`.
- `filename_prefix` and `timestamp_format` define generated file names.
- Invalid values fall back silently to defaults.

## Example

```ini
# General
default_mode=screenshot
default_target=area

# Recording
show_recording_hud=true
recording_indicator_enabled=true
recording_indicator_interval_seconds=5
recording_indicator_duration_ms=300

# Storage
save_dir_screenshots=~/Pictures/Screenshots
save_dir_recordings=~/Videos/Recordings

# Integration
open_video_command=mpv
reveal_folder_command=thunar

# Naming
filename_prefix=hyprscreen
timestamp_format=%H%M%S%d%m%Y
```

## Timestamp Format Notes

- `%H` = hour (00-23)
- `%M` = minute (00-59)
- `%S` = second (00-59)
- `%d` = day (01-31)
- `%m` = month (01-12)
- `%Y` = full year (2026)

Examples:

- `%H%M%S%d%m%Y` -> `23595905042026`
- `%Y%m%d-%H%M%S` -> `20260405-235959`

## Planned Keys

- None required for v1 config coverage.
