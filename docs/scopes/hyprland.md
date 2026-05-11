# Hyprland Scope

## Responsibilities

- Query windows and monitors
- Validate stop shortcut support
- Integrate with Hyprland-specific behavior
- Filter window selection to what is actually visible on screen
- Expose monitor enumeration to the UI layer via `enumerate_monitors()` for identifier overlay placement

## Invariants

- Hyprscreen is Hyprland-only.
- Robust stop validation matters more than flexible manual binds in v0.1.

## Current Behavior

- `Monitor` capture builds a `slurp -r` rectangle per monitor from `hyprctl monitors -j` and uses visual selection.
- `Window` capture uses clients from `hyprctl clients -j`.
- Window candidates are filtered to windows that are visible on monitors whose workspaces are currently active.
- Hyprscreen excludes its own window from selectable window targets.
- The main window is hidden before capture, with a short delay before tools are executed, to avoid self-capture.
- `place_window_exact` and `make_window_plain` are used to position the monitor identifier overlays during `Monitor` selection.
