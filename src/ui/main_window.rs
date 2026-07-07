use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::process::Child;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;

use super::icons::{icon_image, icon_image_colored, icon_texture};
use super::thumbnail;
use super::toast;

const INITIAL_HIDE_DELAY_MS: u64 = 60;
const MONITOR_OVERLAY_EXTRA_DELAY_MS: u64 = 220;
const SELECTION_POLL_INTERVAL_MS: u64 = 50;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Screenshot,
    Record,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Target {
    Area,
    Window,
    Monitor,
}

struct ActiveRecording {
    child: Child,
    temp_path: PathBuf,
    hud_window: Option<gtk::Window>,
    indicator_window: Option<gtk::Window>,
    started_at: Instant,
}

pub fn build(
    app: &gtk::Application,
    startup: Option<crate::cli::StartupAction>,
) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Hyprscreen")
        .resizable(false)
        .build();
    window.set_decorated(false);
    if crate::config::get().dock_style == crate::config::DockStyle::Glass {
        window.add_css_class("hs-glass");
    }
    window.connect_map(move |w| {
        crate::hyprland::make_window_glass("Hyprscreen", 18);
        position_dock(w);
    });

    let recording_state = Rc::new(RefCell::new(None::<ActiveRecording>));
    let setup_cta = Rc::new(RefCell::new(None::<gtk::Button>));
    let show_recording_hud = Rc::new(RefCell::new(crate::config::get().show_recording_hud));

    let dock = build_dock(
        &window,
        &recording_state,
        &setup_cta,
        &show_recording_hud,
        startup,
    );
    window.set_child(Some(&dock));

    window
}

fn build_dock(
    window: &gtk::ApplicationWindow,
    recording_state: &Rc<RefCell<Option<ActiveRecording>>>,
    setup_cta: &Rc<RefCell<Option<gtk::Button>>>,
    show_recording_hud: &Rc<RefCell<bool>>,
    startup: Option<crate::cli::StartupAction>,
) -> gtk::Widget {
    let config = crate::config::get();
    let default_is_record = config.default_mode == crate::config::DefaultMode::Record;

    let delay_secs = Rc::new(Cell::new(0_u64));

    window.add_css_class(if default_is_record {
        "hs-mode-rec"
    } else {
        "hs-mode-shot"
    });

    // ── Dock bar ───────────────────────────────────────────────
    let dock = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .css_classes(["hs-dock"])
        .build();

    // Mode segment: Shot / Rec
    let seg = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(3)
        .valign(gtk::Align::Center)
        .css_classes(["hs-dseg"])
        .build();

    let (screenshot_button, shot_seg_icon) = make_seg_button("Shot", "shot", "hs-seg-shot");
    screenshot_button.set_active(!default_is_record);
    let (record_button, rec_seg_icon) = make_seg_button("Rec", "rec", "hs-seg-rec");
    record_button.set_active(default_is_record);
    seg.append(&screenshot_button);
    seg.append(&record_button);
    if default_is_record {
        rec_seg_icon.set_paintable(Some(&icon_texture("rec", 15, "#FF5D5D")));
    } else {
        shot_seg_icon.set_paintable(Some(&icon_texture("shot", 15, "#5EE6D0")));
    }

    // Targets
    let area_button = make_dock_target("area", "Area · drag a region");
    area_button.set_active(config.default_target == crate::config::DefaultTarget::Area);
    let window_button = make_dock_target("window", "Window · pick a window");
    window_button.set_active(config.default_target == crate::config::DefaultTarget::Window);
    let monitor_button = make_dock_target("monitor", "Monitor · whole display");
    monitor_button.set_active(config.default_target == crate::config::DefaultTarget::Monitor);

    let target_group = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .valign(gtk::Align::Center)
        .build();
    target_group.append(&area_button);
    target_group.append(&window_button);
    target_group.append(&monitor_button);

    // Delay chip — click cycles presets, popover mirrors it
    let delay_chip = gtk::Button::builder()
        .css_classes(["hs-dchip"])
        .valign(gtk::Align::Center)
        .build();
    delay_chip.set_can_focus(false);
    let chip_inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(7)
        .build();
    let chip_icon = icon_image_colored("timer", 16, None, "#9A9CA6");
    let chip_label = gtk::Label::builder()
        .label("No delay")
        .css_classes(["hs-dchip-label"])
        .build();
    chip_inner.append(&chip_icon);
    chip_inner.append(&chip_label);
    delay_chip.set_child(Some(&chip_inner));

    // Pointer toggle (screenshots only — wf-recorder always records the cursor)
    let pointer_button = make_dock_target("pointer", "Pointer: on");
    pointer_button.set_active(true);
    pointer_button.connect_toggled(|btn| {
        btn.set_tooltip_text(Some(if btn.is_active() {
            "Pointer: on"
        } else {
            "Pointer: off"
        }));
    });

    // Quick settings popover
    let more = gtk::MenuButton::builder()
        .css_classes(["hs-dmore"])
        .direction(gtk::ArrowType::Up)
        .valign(gtk::Align::Center)
        .build();
    more.set_can_focus(false);
    more.set_child(Some(&icon_image("chevron", 18, Some("hs-dico-icon"))));

    let qpop = gtk::Popover::builder().css_classes(["hs-qpop"]).build();
    qpop.add_css_class(if default_is_record {
        "hs-mode-rec"
    } else {
        "hs-mode-shot"
    });

    let qbody = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(14)
        .width_request(252)
        .build();

    // Delay presets row
    let delay_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();
    let delay_header = gtk::Label::builder()
        .label("DELAY")
        .halign(gtk::Align::Start)
        .css_classes(["hs-qlabel"])
        .build();
    let qseg = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(3)
        .homogeneous(true)
        .css_classes(["hs-qseg"])
        .build();
    let delay_presets: [(u64, &str); 4] = [(0, "Now"), (3, "3s"), (5, "5s"), (10, "10s")];
    let delay_buttons: Vec<gtk::ToggleButton> = delay_presets
        .iter()
        .map(|(_, text)| {
            let b = gtk::ToggleButton::builder()
                .label(*text)
                .css_classes(["hs-qseg-btn"])
                .build();
            b.set_can_focus(false);
            qseg.append(&b);
            b
        })
        .collect();
    delay_buttons[0].set_active(true);
    delay_row.append(&delay_header);
    delay_row.append(&qseg);

    // Applies a delay value everywhere: shared state, chip, preset buttons.
    let apply_delay = {
        let delay_secs = delay_secs.clone();
        let chip_label = chip_label.clone();
        let delay_chip = delay_chip.clone();
        let delay_buttons = delay_buttons.clone();
        Rc::new(move |value: u64| {
            delay_secs.set(value);
            if value == 0 {
                chip_label.set_label("No delay");
                delay_chip.remove_css_class("on");
            } else {
                chip_label.set_label(&format!("{value}s"));
                delay_chip.add_css_class("on");
            }
            for (button, (preset, _)) in delay_buttons.iter().zip(delay_presets.iter()) {
                button.set_active(*preset == value);
            }
        })
    };

    for (button, (preset, _)) in delay_buttons.iter().zip(delay_presets.iter()) {
        let apply_delay = apply_delay.clone();
        let preset = *preset;
        let delay_secs_for_toggle = delay_secs.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                if delay_secs_for_toggle.get() != preset {
                    apply_delay(preset);
                }
            } else if delay_secs_for_toggle.get() == preset {
                // Never leave the preset row empty.
                b.set_active(true);
            }
        });
    }

    delay_chip.connect_clicked({
        let apply_delay = apply_delay.clone();
        let delay_secs = delay_secs.clone();
        move |_| {
            let next = match delay_secs.get() {
                0 => 3,
                3 => 5,
                5 => 10,
                _ => 0,
            };
            apply_delay(next);
        }
    });

    // Pointer row
    let pointer_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .build();
    let pointer_text = quick_setting_text("Show pointer", "include cursor in capture");
    let pointer_switch = gtk::Switch::builder()
        .active(true)
        .halign(gtk::Align::End)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .css_classes(["hs-switch"])
        .build();
    pointer_row.append(&pointer_text);
    pointer_row.append(&pointer_switch);

    // Keep the dock toggle and the popover switch in lockstep.
    pointer_switch.connect_active_notify(glib::clone!(
        #[weak]
        pointer_button,
        move |switch| {
            if pointer_button.is_active() != switch.is_active() {
                pointer_button.set_active(switch.is_active());
            }
        }
    ));
    pointer_button.connect_toggled(glib::clone!(
        #[weak]
        pointer_switch,
        move |btn| {
            if pointer_switch.is_active() != btn.is_active() {
                pointer_switch.set_active(btn.is_active());
            }
        }
    ));

    // Recording HUD row (record mode only)
    let hud_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .build();
    let hud_text = quick_setting_text("Recording HUD", "timer + stop while recording");
    let hud_toggle = gtk::Switch::builder()
        .active(*show_recording_hud.borrow())
        .halign(gtk::Align::End)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .css_classes(["hs-switch"])
        .build();
    hud_toggle.set_can_focus(false);
    hud_row.append(&hud_text);
    hud_row.append(&hud_toggle);
    hud_row.set_visible(default_is_record);

    hud_toggle.connect_active_notify(glib::clone!(
        #[strong]
        show_recording_hud,
        move |switch| {
            *show_recording_hud.borrow_mut() = switch.is_active();
        }
    ));

    qbody.append(&delay_row);
    qbody.append(&pointer_row);
    qbody.append(&hud_row);
    qpop.set_child(Some(&qbody));
    more.set_popover(Some(&qpop));

    // ── Primary fire button ────────────────────────────────────
    let cta_button = gtk::Button::builder()
        .css_classes(["hs-primary", "hs-dfire"])
        .valign(gtk::Align::Center)
        .build();
    cta_button.set_can_focus(false);
    set_primary_button_content(
        &cta_button,
        if default_is_record {
            Mode::Record
        } else {
            Mode::Screenshot
        },
    );
    if default_is_record {
        cta_button.add_css_class("mode-rec");
    } else {
        cta_button.add_css_class("mode-shot");
    }
    *setup_cta.borrow_mut() = Some(cta_button.clone());

    // ── Mode toggle handlers ───────────────────────────────────
    let target_buttons = [
        area_button.clone(),
        window_button.clone(),
        monitor_button.clone(),
    ];

    screenshot_button.connect_toggled(glib::clone!(
        #[weak]
        window,
        #[weak]
        screenshot_button,
        #[weak]
        record_button,
        #[weak]
        cta_button,
        #[weak]
        hud_row,
        #[weak]
        pointer_button,
        #[weak]
        qpop,
        #[weak]
        shot_seg_icon,
        #[weak]
        rec_seg_icon,
        move |_| {
            if !screenshot_button.is_active() && !record_button.is_active() {
                screenshot_button.set_active(true);
            }
            if screenshot_button.is_active() {
                record_button.set_active(false);
                set_primary_button_content(&cta_button, Mode::Screenshot);
                cta_button.remove_css_class("mode-rec");
                cta_button.add_css_class("mode-shot");
                for widget in [window.upcast_ref::<gtk::Widget>(), qpop.upcast_ref()] {
                    widget.remove_css_class("hs-mode-rec");
                    widget.add_css_class("hs-mode-shot");
                }
                shot_seg_icon.set_paintable(Some(&icon_texture("shot", 15, "#5EE6D0")));
                rec_seg_icon.set_paintable(Some(&icon_texture("rec", 15, "#9A9CA6")));
                hud_row.set_visible(false);
                pointer_button.set_sensitive(true);
                pointer_button.set_tooltip_text(Some(if pointer_button.is_active() {
                    "Pointer: on"
                } else {
                    "Pointer: off"
                }));
            }
        }
    ));

    record_button.connect_toggled(glib::clone!(
        #[weak]
        window,
        #[weak]
        screenshot_button,
        #[weak]
        record_button,
        #[weak]
        cta_button,
        #[weak]
        hud_row,
        #[weak]
        pointer_button,
        #[weak]
        qpop,
        #[weak]
        shot_seg_icon,
        #[weak]
        rec_seg_icon,
        move |_| {
            if !screenshot_button.is_active() && !record_button.is_active() {
                record_button.set_active(true);
            }
            if record_button.is_active() {
                screenshot_button.set_active(false);
                set_primary_button_content(&cta_button, Mode::Record);
                cta_button.remove_css_class("mode-shot");
                cta_button.add_css_class("mode-rec");
                for widget in [window.upcast_ref::<gtk::Widget>(), qpop.upcast_ref()] {
                    widget.remove_css_class("hs-mode-shot");
                    widget.add_css_class("hs-mode-rec");
                }
                shot_seg_icon.set_paintable(Some(&icon_texture("shot", 15, "#9A9CA6")));
                rec_seg_icon.set_paintable(Some(&icon_texture("rec", 15, "#FF5D5D")));
                hud_row.set_visible(true);
                pointer_button.set_sensitive(false);
                pointer_button.set_tooltip_text(Some("wf-recorder always records the pointer"));
            }
        }
    ));

    // ── Target mutual-exclusion ────────────────────────────────
    for current in &target_buttons {
        let all = target_buttons.clone();
        current.connect_toggled(move |button| {
            if button.is_active() {
                for other in &all {
                    if other != button {
                        other.set_active(false);
                    }
                }
            } else if !all.iter().any(|b| b.is_active()) {
                button.set_active(true);
            }
        });
    }

    // ── Fire ───────────────────────────────────────────────────
    cta_button.connect_clicked(glib::clone!(
        #[weak]
        screenshot_button,
        #[weak]
        area_button,
        #[weak]
        window_button,
        #[weak]
        pointer_button,
        #[weak]
        cta_button,
        #[weak]
        window,
        #[strong]
        recording_state,
        #[strong]
        setup_cta,
        #[strong]
        show_recording_hud,
        #[strong]
        delay_secs,
        move |_| {
            let target = active_target(&area_button, &window_button);
            let delay = delay_secs.get();

            if screenshot_button.is_active() {
                run_capture_action(
                    &window,
                    target,
                    delay,
                    pointer_button.is_active(),
                    &cta_button,
                );
                return;
            }

            let show_hud = *show_recording_hud.borrow();
            start_recording_action(
                &window,
                &recording_state,
                &setup_cta,
                target,
                show_hud,
                delay,
                &cta_button,
            );
        }
    ));

    // ── Keyboard shortcuts ─────────────────────────────────────
    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(glib::clone!(
        #[weak(rename_to = win)]
        window,
        #[weak]
        cta_button,
        #[weak]
        screenshot_button,
        #[weak]
        record_button,
        #[weak]
        area_button,
        #[weak]
        window_button,
        #[weak]
        monitor_button,
        #[weak]
        delay_chip,
        #[weak]
        pointer_button,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, _| {
            match key {
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                    if cta_button.is_sensitive() {
                        cta_button.emit_clicked();
                    }
                }
                gtk::gdk::Key::_1 | gtk::gdk::Key::KP_1 => area_button.set_active(true),
                gtk::gdk::Key::_2 | gtk::gdk::Key::KP_2 => window_button.set_active(true),
                gtk::gdk::Key::_3 | gtk::gdk::Key::KP_3 => monitor_button.set_active(true),
                gtk::gdk::Key::s | gtk::gdk::Key::S => screenshot_button.set_active(true),
                gtk::gdk::Key::r | gtk::gdk::Key::R => record_button.set_active(true),
                gtk::gdk::Key::d | gtk::gdk::Key::D => delay_chip.emit_clicked(),
                gtk::gdk::Key::p | gtk::gdk::Key::P => {
                    if pointer_button.is_sensitive() {
                        pointer_button.set_active(!pointer_button.is_active());
                    }
                }
                gtk::gdk::Key::Escape => win.close(),
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        }
    ));
    window.add_controller(key_controller);

    // ── Assemble ───────────────────────────────────────────────
    dock.append(&seg);
    dock.append(&make_dock_divider());
    dock.append(&target_group);
    dock.append(&make_dock_divider());
    dock.append(&delay_chip);
    dock.append(&pointer_button);
    dock.append(&more);
    dock.append(&cta_button);

    if let Some(action) = startup {
        let screenshot_btn = screenshot_button.clone();
        let record_btn = record_button.clone();
        let area_btn = area_button.clone();
        let window_btn = window_button.clone();
        let monitor_btn = monitor_button.clone();
        let cta = cta_button.clone();
        glib::timeout_add_local_once(Duration::from_millis(120), move || {
            match action {
                crate::cli::StartupAction::Screenshot(target) => {
                    if !screenshot_btn.is_active() {
                        screenshot_btn.set_active(true);
                    }
                    apply_startup_target(target, &area_btn, &window_btn, &monitor_btn);
                }
                crate::cli::StartupAction::Record(target) => {
                    if !record_btn.is_active() {
                        record_btn.set_active(true);
                    }
                    apply_startup_target(target, &area_btn, &window_btn, &monitor_btn);
                }
            }
            cta.emit_clicked();
        });
    }

    dock.upcast()
}

fn make_seg_button(
    label_text: &str,
    icon_key: &str,
    class: &str,
) -> (gtk::ToggleButton, gtk::Image) {
    let btn = gtk::ToggleButton::new();
    btn.add_css_class("hs-dseg-btn");
    btn.add_css_class(class);
    btn.set_can_focus(false);

    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();
    let icon = icon_image_colored(icon_key, 15, None, "#9A9CA6");
    let label = gtk::Label::builder()
        .label(label_text)
        .css_classes(["hs-dseg-label"])
        .build();
    inner.append(&icon);
    inner.append(&label);
    btn.set_child(Some(&inner));
    (btn, icon)
}

fn make_dock_target(icon_name: &str, tip: &str) -> gtk::ToggleButton {
    let btn = gtk::ToggleButton::builder()
        .width_request(40)
        .height_request(40)
        .valign(gtk::Align::Center)
        .css_classes(["hs-dico"])
        .build();
    btn.set_can_focus(false);
    btn.set_tooltip_text(Some(tip));

    let overlay = gtk::Overlay::new();
    let icon = icon_image(icon_name, 19, Some("hs-dico-icon"));
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    overlay.set_child(Some(&icon));

    // The active-target dot: the design uses ::after, which GTK4 CSS lacks.
    let dot = gtk::Box::builder()
        .width_request(4)
        .height_request(4)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::End)
        .margin_bottom(5)
        .css_classes(["hs-dico-dot"])
        .build();
    dot.set_visible(false);
    overlay.add_overlay(&dot);
    btn.set_child(Some(&overlay));

    btn.connect_toggled(glib::clone!(
        #[weak]
        dot,
        move |b| dot.set_visible(b.is_active())
    ));
    btn
}

fn make_dock_divider() -> gtk::Box {
    gtk::Box::builder()
        .width_request(1)
        .height_request(26)
        .valign(gtk::Align::Center)
        .css_classes(["hs-ddiv"])
        .build()
}

fn quick_setting_text(name: &str, sub: &str) -> gtk::Box {
    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(3)
        .valign(gtk::Align::Center)
        .build();
    let name_label = gtk::Label::builder()
        .label(name)
        .halign(gtk::Align::Start)
        .css_classes(["hs-qtoggle-name"])
        .build();
    let sub_label = gtk::Label::builder()
        .label(sub)
        .halign(gtk::Align::Start)
        .css_classes(["hs-qtoggle-sub"])
        .build();
    text.append(&name_label);
    text.append(&sub_label);
    text
}

fn position_dock(window: &gtk::ApplicationWindow) {
    let window = window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        let (w, h) = (window.width(), window.height());
        if w <= 1 {
            return;
        }
        if let Some(mon) = crate::hyprland::focused_monitor() {
            let x = mon.x + ((mon.width - w) / 2).max(0);
            let y = mon.y + (mon.height - h - 34).max(0);
            crate::hyprland::place_window_exact("Hyprscreen", x, y);
        }
    });
}

fn apply_startup_target(
    target: crate::cli::StartupTarget,
    area: &gtk::ToggleButton,
    window: &gtk::ToggleButton,
    monitor: &gtk::ToggleButton,
) {
    let to_activate = match target {
        crate::cli::StartupTarget::Area => area,
        crate::cli::StartupTarget::Window => window,
        crate::cli::StartupTarget::Monitor => monitor,
    };
    if !to_activate.is_active() {
        to_activate.set_active(true);
    }
}

fn run_capture_action(
    window: &gtk::ApplicationWindow,
    target: Target,
    delay_secs: u64,
    include_pointer: bool,
    fire_button: &gtk::Button,
) {
    let window = window.clone();
    let fire_button = fire_button.clone();

    fire_button.set_sensitive(false);
    window.set_visible(false);

    let overlays = if target == Target::Monitor {
        show_monitor_identifiers(&crate::hyprland::enumerate_monitors())
    } else {
        Vec::new()
    };
    let delay_ms = if target == Target::Monitor {
        MONITOR_OVERLAY_EXTRA_DELAY_MS
    } else {
        INITIAL_HIDE_DELAY_MS
    };

    glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
        // Phase 1: run slurp on a worker thread.
        // Area/Window return a geometry string; Monitor returns the output name.
        let (sel_tx, sel_rx) = std::sync::mpsc::channel::<anyhow::Result<String>>();
        std::thread::spawn(move || {
            let result = match target {
                Target::Area => crate::capture::screenshot::select_area(),
                Target::Window => crate::capture::screenshot::select_window(),
                Target::Monitor => crate::capture::screenshot::select_monitor(),
            };
            let _ = sel_tx.send(result);
        });

        let overlays_cell = Rc::new(RefCell::new(Some(overlays)));
        glib::timeout_add_local(Duration::from_millis(SELECTION_POLL_INTERVAL_MS), move || {
            let sel_result = match sel_rx.try_recv() {
                Ok(r) => r,
                Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(_) => return glib::ControlFlow::Break,
            };

            if let Some(ov) = overlays_cell.borrow_mut().take() {
                close_monitor_identifiers(ov);
            }

            let selection = match sel_result {
                Err(error) => {
                    report_capture_error("Capture failed", &error, &window, &fire_button);
                    return glib::ControlFlow::Break;
                }
                Ok(s) => s,
            };

            // Phase 2: CompositorRepaintGuard on the worker thread already waited for
            // closelayer + one frame. This idle hands back to the GTK main loop before
            // spawning the grim thread.
            let window2 = window.clone();
            let fire_button2 = fire_button.clone();

            let proceed = move || {
                wait_compositor_frame(move || {
                    let (cap_tx, cap_rx) =
                        std::sync::mpsc::channel::<anyhow::Result<std::path::PathBuf>>();
                    std::thread::spawn(move || {
                        let result = match target {
                            Target::Area => crate::capture::screenshot::capture_geometry(
                                &selection,
                                include_pointer,
                            ),
                            Target::Window => {
                                crate::capture::screenshot::capture_window_geometry(
                                    &selection,
                                    include_pointer,
                                )
                            }
                            Target::Monitor => {
                                crate::capture::screenshot::capture_by_monitor_name(
                                    &selection,
                                    include_pointer,
                                )
                            }
                        };
                        let _ = cap_tx.send(result);
                    });

                    glib::timeout_add_local(
                        Duration::from_millis(SELECTION_POLL_INTERVAL_MS),
                        move || {
                            let result = match cap_rx.try_recv() {
                                Ok(r) => r,
                                Err(std::sync::mpsc::TryRecvError::Empty) => {
                                    return glib::ControlFlow::Continue;
                                }
                                Err(_) => return glib::ControlFlow::Break,
                            };

                            window2.present();
                            fire_button2.set_sensitive(true);
                            match result.and_then(thumbnail::for_screenshot) {
                                Ok(info) => thumbnail::show(info),
                                Err(error) => {
                                    report_capture_error(
                                        "Capture failed",
                                        &error,
                                        &window2,
                                        &fire_button2,
                                    );
                                }
                            }
                            glib::ControlFlow::Break
                        },
                    );
                })
            };
            // The delay counts against the already-chosen geometry (plan: selection first).
            if delay_secs > 0 {
                glib::timeout_add_local_once(Duration::from_secs(delay_secs), proceed);
            } else {
                proceed();
            }

            glib::ControlFlow::Break
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn start_recording_action(
    window: &gtk::ApplicationWindow,
    recording_state: &Rc<RefCell<Option<ActiveRecording>>>,
    setup_cta: &Rc<RefCell<Option<gtk::Button>>>,
    target: Target,
    show_hud: bool,
    delay_secs: u64,
    fire_button: &gtk::Button,
) {
    let window = window.clone();
    let recording_state = recording_state.clone();
    let setup_cta = setup_cta.clone();
    let fire_button = fire_button.clone();

    fire_button.set_sensitive(false);
    window.set_visible(false);

    let overlays = if target == Target::Monitor {
        show_monitor_identifiers(&crate::hyprland::enumerate_monitors())
    } else {
        Vec::new()
    };
    let delay_ms = if target == Target::Monitor {
        MONITOR_OVERLAY_EXTRA_DELAY_MS
    } else {
        INITIAL_HIDE_DELAY_MS
    };

    glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
        // Phase 1: run slurp on a worker thread.
        let (sel_tx, sel_rx) = std::sync::mpsc::channel::<
            anyhow::Result<crate::capture::record::RecordingSelection>,
        >();
        std::thread::spawn(move || {
            let result = match target {
                Target::Area => crate::capture::record::select_area(),
                Target::Monitor => crate::capture::record::select_monitor(),
                Target::Window => crate::capture::record::select_window(),
            };
            let _ = sel_tx.send(result);
        });

        let overlays_cell = Rc::new(RefCell::new(Some(overlays)));
        glib::timeout_add_local(Duration::from_millis(SELECTION_POLL_INTERVAL_MS), move || {
            let sel_result = match sel_rx.try_recv() {
                Ok(r) => r,
                Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(_) => return glib::ControlFlow::Break,
            };

            if let Some(ov) = overlays_cell.borrow_mut().take() {
                close_monitor_identifiers(ov);
            }

            let selection = match sel_result {
                Err(error) => {
                    report_capture_error("Recording failed", &error, &window, &fire_button);
                    return glib::ControlFlow::Break;
                }
                Ok(s) => s,
            };

            // Phase 2: wait for one compositor frame, then launch wf-recorder.
            // launch_recording is fast (spawn + file write) so runs on the GTK thread.
            let window2 = window.clone();
            let recording_state2 = recording_state.clone();
            let setup_cta2 = setup_cta.clone();
            let fire_button2 = fire_button.clone();

            let proceed = move || {
                wait_compositor_frame(move || {
                    match crate::capture::record::launch_recording(selection) {
                        Err(error) => {
                            report_capture_error(
                                "Recording failed",
                                &error,
                                &window2,
                                &fire_button2,
                            );
                        }
                        Ok(session) => {
                            let hud_window = if show_hud {
                                Some(create_recording_hud(&recording_state2))
                            } else {
                                None
                            };

                            let monitor = session.monitor;
                            let indicator_window = if show_hud {
                                None
                            } else if crate::config::get().recording_indicator_enabled {
                                let (w, _dot) =
                                    create_recording_indicator(monitor, &recording_state2);
                                Some(w)
                            } else {
                                None
                            };

                            *recording_state2.borrow_mut() = Some(ActiveRecording {
                                child: session.child,
                                temp_path: session.temp_path,
                                hud_window,
                                indicator_window,
                                started_at: Instant::now(),
                            });

                            start_recording_poll(&window2, &recording_state2, &setup_cta2);
                        }
                    }
                })
            };
            if delay_secs > 0 {
                glib::timeout_add_local_once(Duration::from_secs(delay_secs), proceed);
            } else {
                proceed();
            }

            glib::ControlFlow::Break
        });
    });
}

fn start_recording_poll(
    window: &gtk::ApplicationWindow,
    recording_state: &Rc<RefCell<Option<ActiveRecording>>>,
    setup_cta: &Rc<RefCell<Option<gtk::Button>>>,
) {
    let window = window.clone();
    let recording_state = recording_state.clone();
    let setup_cta = setup_cta.clone();

    glib::timeout_add_local(Duration::from_millis(250), move || {
        let mut borrowed = recording_state.borrow_mut();
        let Some(active) = borrowed.as_mut() else {
            return glib::ControlFlow::Break;
        };

        match active.child.try_wait() {
            Ok(Some(status)) => {
                let finished = borrowed.take().expect("active recording disappeared");
                drop(borrowed);

                if let Some(hud) = finished.hud_window {
                    hud.close();
                }
                if let Some(indicator) = finished.indicator_window {
                    indicator.close();
                }
                crate::capture::record::clear_state_file();
                window.present();
                enable_setup_cta(&setup_cta);

                if !status.success() || !finished.temp_path.exists() {
                    toast::error("Recording failed", "the recording was cancelled or crashed");
                    return glib::ControlFlow::Break;
                }

                match thumbnail::for_recording(finished.temp_path) {
                    Ok(info) => thumbnail::show(info),
                    Err(error) => toast::error("Recording preview failed", &error.to_string()),
                }
                glib::ControlFlow::Break
            }
            Ok(None) => glib::ControlFlow::Continue,
            Err(error) => {
                drop(borrowed);
                crate::capture::record::clear_state_file();
                window.present();
                enable_setup_cta(&setup_cta);
                toast::error("Recording poll failed", &error.to_string());
                glib::ControlFlow::Break
            }
        }
    });
}

fn report_capture_error(
    prefix: &str,
    error: &anyhow::Error,
    window: &gtk::ApplicationWindow,
    fire_button: &gtk::Button,
) {
    window.present();
    fire_button.set_sensitive(true);
    let retry_button = fire_button.clone();
    toast::error_with(prefix, &error.to_string(), "Retry", move || {
        if retry_button.is_sensitive() {
            retry_button.emit_clicked();
        }
    });
}

fn wait_compositor_frame<F: FnOnce() + 'static>(callback: F) {
    // The worker thread already waited for Hyprland's closelayer event + one
    // frame via CompositorRepaintGuard. This idle just hands control back to
    // the GTK main loop before spawning the capture thread.
    glib::idle_add_local_once(callback);
}

fn enable_setup_cta(setup_cta: &Rc<RefCell<Option<gtk::Button>>>) {
    if let Some(button) = setup_cta.borrow().as_ref() {
        button.set_sensitive(true);
    }
}

fn create_recording_hud(recording_state: &Rc<RefCell<Option<ActiveRecording>>>) -> gtk::Window {
    let hud = gtk::Window::builder()
        .title("Hyprscreen HUD")
        .decorated(false)
        .resizable(false)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["hs-hud"])
        .build();

    // Red pulse dot
    let rec_dot = gtk::Box::builder()
        .width_request(9)
        .height_request(9)
        .valign(gtk::Align::Center)
        .css_classes(["hs-hud-dot"])
        .build();

    // REC label
    let rec_label = gtk::Label::builder()
        .label("REC")
        .css_classes(["hs-hud-rec"])
        .build();

    // Timer
    let counter = gtk::Label::builder()
        .label("00:00")
        .css_classes(["hs-hud-timer"])
        .build();

    // Separator
    let sep = gtk::Box::builder()
        .width_request(1)
        .height_request(16)
        .valign(gtk::Align::Center)
        .css_classes(["hs-hud-sep"])
        .build();

    // Stop button
    let stop = gtk::Button::builder().css_classes(["hs-hud-stop"]).build();
    set_button_text(&stop, "STOP");

    content.append(&rec_dot);
    content.append(&rec_label);
    content.append(&counter);
    content.append(&sep);
    content.append(&stop);
    hud.set_child(Some(&content));

    stop.connect_clicked(move |_| {
        let _ = crate::capture::record::stop_active_recording();
    });

    let recording_state_for_timer = recording_state.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        let borrowed = recording_state_for_timer.borrow();
        let Some(active) = borrowed.as_ref() else {
            return glib::ControlFlow::Break;
        };
        let elapsed = active.started_at.elapsed().as_secs();
        let m = elapsed / 60;
        let s = elapsed % 60;
        counter.set_label(&format!("{m:02}:{s:02}"));
        glib::ControlFlow::Continue
    });

    hud.present();
    hud
}

fn show_monitor_identifiers(monitors: &[crate::hyprland::Monitor]) -> Vec<gtk::Window> {
    monitors
        .iter()
        .map(|monitor| {
            let title = format!("Hyprscreen Monitor ID {}", monitor.name);
            let window = gtk::Window::builder()
                .title(&title)
                .decorated(false)
                .resizable(false)
                .default_width(180)
                .default_height(96)
                .build();
            window.add_css_class("hs-mon-id");

            let label = gtk::Label::builder()
                .label(&monitor.name)
                .css_classes(["hs-mon-id-label"])
                .build();
            window.set_child(Some(&label));
            window.present();

            let x = monitor.x + (monitor.width - 180) / 2;
            let y = monitor.y + (monitor.height - 96) / 2;
            crate::hyprland::make_window_plain(&title);
            crate::hyprland::place_window_exact(&title, x, y);

            window
        })
        .collect()
}

fn close_monitor_identifiers(overlays: Vec<gtk::Window>) {
    for overlay in overlays {
        overlay.close();
    }
    if let Some(display) = gtk::gdk::Display::default() {
        display.sync();
    }
}

fn create_recording_indicator(
    monitor: crate::capture::record::MonitorPlacement,
    recording_state: &Rc<RefCell<Option<ActiveRecording>>>,
) -> (gtk::Window, gtk::Label) {
    let indicator = gtk::Window::builder()
        .title("Hyprscreen Recording Indicator")
        .decorated(false)
        .resizable(false)
        .default_width(16)
        .default_height(16)
        .build();
    indicator.add_css_class("hs-rec-indicator");

    let dot = gtk::Label::builder()
        .label("●")
        .css_classes(["hs-rec-flash"])
        .margin_top(2)
        .margin_bottom(2)
        .margin_start(2)
        .margin_end(2)
        .build();
    indicator.set_child(Some(&dot));
    indicator.present();

    let x = monitor.x + ((monitor.width - 16) / 2).max(0);
    let y = monitor.y + monitor.height - 16 - 20;
    crate::hyprland::make_window_plain("Hyprscreen Recording Indicator");
    crate::hyprland::place_window_exact("Hyprscreen Recording Indicator", x, y);
    dot.set_visible(false);

    let dot_for_first_flash = dot.clone();
    glib::timeout_add_local_once(Duration::from_millis(180), move || {
        flash_indicator(&dot_for_first_flash);
    });

    let dot_for_timer = dot.clone();
    let recording_state_for_timer = recording_state.clone();
    let interval = crate::config::get().recording_indicator_interval_seconds;
    glib::timeout_add_local(Duration::from_secs(interval), move || {
        if recording_state_for_timer.borrow().is_none() {
            return glib::ControlFlow::Break;
        }
        flash_indicator(&dot_for_timer);
        glib::ControlFlow::Continue
    });

    (indicator, dot)
}

fn flash_indicator(dot: &gtk::Label) {
    dot.set_visible(true);
    let dot = dot.clone();
    glib::timeout_add_local_once(
        Duration::from_millis(crate::config::get().recording_indicator_duration_ms),
        move || {
            dot.set_visible(false);
        },
    );
}

fn active_target(area_button: &gtk::ToggleButton, window_button: &gtk::ToggleButton) -> Target {
    if area_button.is_active() {
        Target::Area
    } else if window_button.is_active() {
        Target::Window
    } else {
        Target::Monitor
    }
}

fn set_button_text(button: &impl IsA<gtk::Button>, text: &str) {
    let button = button.as_ref();

    if let Some(label) = button
        .child()
        .and_then(|child| child.downcast::<gtk::Label>().ok())
    {
        label.set_label(text);
        return;
    }

    let label = gtk::Label::new(Some(text));
    button.set_child(Some(&label));
}

fn set_primary_button_content(button: &gtk::Button, mode: Mode) {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();
    let icon: gtk::Widget = match mode {
        Mode::Screenshot => icon_image_colored("shutter", 18, None, "#06231F").upcast(),
        Mode::Record => gtk::Box::builder()
            .width_request(13)
            .height_request(13)
            .css_classes(["hs-rec-ring"])
            .valign(gtk::Align::Center)
            .build()
            .upcast(),
    };
    let label = gtk::Label::builder()
        .label(match mode {
            Mode::Screenshot => "Capture",
            Mode::Record => "Record",
        })
        .css_classes(["hs-primary-label"])
        .build();
    let kbd = gtk::Label::builder()
        .label(match mode {
            Mode::Screenshot => "\u{23ce}",
            Mode::Record => "\u{23f5}",
        })
        .css_classes(["hs-fire-kbd"])
        .build();
    content.append(&icon);
    content.append(&label);
    content.append(&kbd);
    button.set_child(Some(&content));
}
