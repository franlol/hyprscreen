---
name: verify
description: How to drive hyprscreen's GTK UI end-to-end on this machine (real input injection, window screenshots) to verify changes.
---

# Verifying hyprscreen UI changes

Runs on the live Hyprland session (no headless setup needed).

## Real input injection (mouse + keyboard)

`/dev/uinput` is user-writable (ACL) and `python-evdev` is installed, so real
clicks and keystrokes can be injected at the compositor level:

1. Position the cursor exactly: `hyprctl dispatch movecursor <x> <y>` (global
   logical coordinates).
2. Inject button/key events with a small python-evdev `UInput` script
   (create the virtual device, **sleep ~1s** so Hyprland picks it up, then
   write EV_KEY events). Chain click/type/key steps in one invocation.

Two hazards learned the hard way:

- **A fresh virtual keyboard drops its first event** (keymap negotiation
  race). Tap a harmless key (bare Shift) as a warmup before the real
  sequence, or the first shortcut silently vanishes.
- **Keystrokes go to whatever window is focused.** franlol may be actively
  using the machine; injected typing once landed in the Claude Code prompt
  and self-submitted as a message. Always `hyprctl dispatch focuswindow
  address:<a>` + confirm via `hyprctl -j activewindow` immediately before a
  typing batch, and if the user is mid-session, ask for a hands-off window
  (or let them test manually) instead of injecting blind.

This supersedes older notes saying pointer automation is impossible here
(no ydotool/wtype — still true, but uinput works directly).

## Window geometry and screenshots

- Find the window: `hyprctl -j clients | jq '.[] | select(.title=="…")'` —
  `at`/`size` are global logical px.
- Screenshot it: `grim -g "<x>,<y> <w>x<h>" out.png`. Output is physical px;
  the ratio `image_width / logical_width` (≈1.666 here) converts pixel
  positions found in the screenshot back to logical click coordinates
  (divide by ratio, add window origin).

## Reaching UI surfaces with no CLI entry

Most windows (annotate editor, thumbnail, HUD) only open mid-flow. Add a
temporary CLI arm in `src/cli/mod.rs` that opens the target window directly,
e.g. for the annotate editor: `gtk::init()`, build a `gtk::Application` with
a debug app id, in `connect_activate` call `crate::app::load_css()` (needs a
temporary `pub`), `std::mem::forget(app.hold())` so the app survives the
window closing, then `crate::ui::annotate::open(&image, |_| {})`.
**Remove the hook before committing.**

Gotcha: `app.hold()` keeps the first process alive, and GApplication
uniqueness routes a second launch to it — with the *old* closure arguments.
`pkill -x hyprscreen` between runs (`pkill -f` kills your own shell).
