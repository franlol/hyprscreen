//! Hyprland-specific integration points.

use std::process::Command;
use std::time::Duration;

use gtk::glib;
use serde::Deserialize;

#[derive(Deserialize)]
struct ClientInfo {
    address: String,
    title: String,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub focused: bool,
}

#[derive(Deserialize)]
struct MonitorInfoRaw {
    name: String,
    disabled: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: f64,
    #[serde(default)]
    focused: bool,
}

pub fn enumerate_monitors() -> Vec<Monitor> {
    let Ok(output) = Command::new("hyprctl").args(["monitors", "-j"]).output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(monitors) = serde_json::from_slice::<Vec<MonitorInfoRaw>>(&output.stdout) else {
        return Vec::new();
    };
    monitors
        .into_iter()
        .filter(|m| !m.disabled)
        .map(|m| Monitor {
            name: m.name,
            x: m.x,
            y: m.y,
            width: ((m.width as f64) / m.scale).round() as i32,
            height: ((m.height as f64) / m.scale).round() as i32,
            focused: m.focused,
        })
        .collect()
}

/// The monitor holding keyboard focus, falling back to the first enumerated one.
pub fn focused_monitor() -> Option<Monitor> {
    let monitors = enumerate_monitors();
    monitors
        .iter()
        .find(|m| m.focused)
        .cloned()
        .or_else(|| monitors.into_iter().next())
}

fn is_hyprland_session() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

fn dispatch_setprop(selector: &str, property: &str, value: &str) {
    let _ = Command::new("hyprctl")
        .arg("dispatch")
        .arg("setprop")
        .arg(selector)
        .arg(property)
        .arg(value)
        .output();
}

fn dispatch_move_exact(selector: &str, x: i32, y: i32) {
    let _ = Command::new("hyprctl")
        .arg("dispatch")
        .arg("movewindowpixel")
        .arg(format!("exact {x} {y},{selector}"))
        .output();
}

fn dispatch_setfloating(selector: &str) {
    let _ = Command::new("hyprctl")
        .arg("dispatch")
        .arg("setfloating")
        .arg(selector)
        .output();
}

thread_local! {
    static PREPOSITIONED: std::cell::RefCell<std::collections::HashMap<String, (i32, i32)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Registers a windowrule so the next window titled `window_match` maps
/// floating at (x, y) with no open animation, instead of tiling at screen
/// center until the post-map `place_window_exact` move lands. Dynamic rules
/// cannot be unset (only a config reload clears them) but later rules win,
/// so the last coordinates are cached and a rule is only added when they
/// change — the freshest rule supersedes stale ones.
pub fn preposition_window(window_match: &str, x: i32, y: i32) {
    if !is_hyprland_session() {
        return;
    }
    let cached = PREPOSITIONED.with_borrow(|map| map.get(window_match) == Some(&(x, y)));
    if cached {
        return;
    }
    let rule = format!("match:title ^({window_match})$, float 1, no_anim 1, move {x} {y}");
    let output = Command::new("hyprctl")
        .args(["keyword", "windowrule", &rule])
        .output();
    let ok = matches!(&output, Ok(o) if o.status.success() && o.stdout.starts_with(b"ok"));
    if ok {
        PREPOSITIONED
            .with_borrow_mut(|map| map.insert(window_match.to_string(), (x, y)));
    }
}

pub fn place_window_exact(window_match: &str, x: i32, y: i32) {
    if !is_hyprland_session() {
        return;
    }

    for delay in [40_u64, 90_u64] {
        let title = window_match.to_string();
        glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            if let Some(selector) = selector_for_title(&title) {
                dispatch_setfloating(&selector);
                dispatch_move_exact(&selector, x, y);
            }
        });
    }
}

/// Moves a floating window by a relative delta (used for drag-to-move).
#[cfg_attr(not(feature = "webcam"), allow(dead_code))]
pub fn move_window_relative(window_match: &str, dx: i32, dy: i32) {
    if !is_hyprland_session() {
        return;
    }
    if let Some(selector) = selector_for_title(window_match) {
        let _ = Command::new("hyprctl")
            .arg("dispatch")
            .arg("movewindowpixel")
            .arg(format!("{dx} {dy},{selector}"))
            .output();
    }
}

/// Pins a floating window so it stays above the stack (best effort).
pub fn pin_window(window_match: &str) {
    if !is_hyprland_session() {
        return;
    }
    if let Some(selector) = selector_for_title(window_match) {
        let _ = Command::new("hyprctl")
            .args(["dispatch", "pin", &selector])
            .output();
    }
}

pub fn make_window_plain(window_match: &str) {
    if !is_hyprland_session() {
        return;
    }

    for delay in [40_u64, 90_u64, 160_u64] {
        let title = window_match.to_string();
        glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            if let Some(selector) = selector_for_title(&title) {
                dispatch_setprop(&selector, "decorate", "0");
                dispatch_setprop(&selector, "border_size", "0");
                dispatch_setprop(&selector, "no_blur", "1");
                dispatch_setprop(&selector, "no_shadow", "1");
                dispatch_setprop(&selector, "rounding", "0");
                dispatch_setprop(&selector, "no_anim", "1");
            }
        });
    }
}

/// Like `make_window_plain`, but leaves Hyprland's blur and shadow on and asks
/// for rounded corners, so a translucent window renders as a glass panel
/// (ADR-0011). Requires `decoration:blur` enabled in Hyprland; without it the
/// window degrades to a flat translucent panel.
pub fn make_window_glass(window_match: &str, rounding: u32) {
    if !is_hyprland_session() {
        return;
    }

    for delay in [40_u64, 90_u64, 160_u64] {
        let title = window_match.to_string();
        glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            if let Some(selector) = selector_for_title(&title) {
                dispatch_setprop(&selector, "decorate", "0");
                dispatch_setprop(&selector, "border_size", "0");
                dispatch_setprop(&selector, "no_blur", "0");
                dispatch_setprop(&selector, "no_shadow", "0");
                dispatch_setprop(&selector, "rounding", &rounding.to_string());
                dispatch_setprop(&selector, "no_anim", "1");
            }
        });
    }
}

fn selector_for_title(title: &str) -> Option<String> {
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let clients: Vec<ClientInfo> = serde_json::from_slice(&output.stdout).ok()?;
    clients
        .into_iter()
        .find(|client| client.title == title)
        .map(|client| format!("address:{}", client.address))
}
