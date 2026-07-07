use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::process::Child;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;

use super::icons::{icon_image, icon_image_colored, icon_texture};
use super::countdown;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingAction {
    None,
    Pause,
    Restart,
}

struct ActiveRecording {
    /// `None` while paused (the segment process has exited).
    child: Option<Child>,
    /// The segment currently being written (or last written while paused).
    temp_path: PathBuf,
    /// Completed earlier segments (pause/resume, ADR-0016).
    segments: Vec<PathBuf>,
    spec: crate::capture::record::LaunchSpec,
    /// First segment's path; later segments derive their names from it.
    base_path: PathBuf,
    seg_index: u32,
    hud_window: Option<gtk::Window>,
    indicator_window: Option<gtk::Window>,
    accumulated: Duration,
    segment_started: Option<Instant>,
    pending: PendingAction,
}

impl ActiveRecording {
    fn elapsed(&self) -> Duration {
        self.accumulated
            + self
                .segment_started
                .map(|t| t.elapsed())
                .unwrap_or_default()
    }
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

    // Pre-position from the measured size so the dock maps at its final
    // spot instead of flashing at screen center; position_dock then
    // corrects with the realized size and keeps the rule fresh for
    // post-capture re-maps.
    let (_, nat_w, _, _) = dock.measure(gtk::Orientation::Horizontal, -1);
    let (_, nat_h, _, _) = dock.measure(gtk::Orientation::Vertical, nat_w);
    if let Some(mon) = crate::hyprland::focused_monitor() {
        let (x, y) = dock_position(&mon, nat_w, nat_h);
        crate::hyprland::preposition_window("Hyprscreen", x, y);
    }

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
    let format_cell = Rc::new(Cell::new(config.recording_format));
    let audio_cell = Rc::new(Cell::new(config.record_audio));

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
    pointer_button.set_active(config.show_pointer);
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
    if config.capture_delay_seconds > 0 {
        apply_delay(config.capture_delay_seconds);
    }

    // Pointer row
    let pointer_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .build();
    let pointer_text = quick_setting_text("Show pointer", "include cursor in capture");
    let pointer_switch = gtk::Switch::builder()
        .active(config.show_pointer)
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

    // ── Record-only rows: format, audio, HUD ──────────────────
    let rec_rows = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(14)
        .build();

    // Format (MP4/WEBM; GIF stays post-capture per ADR-0010/0017)
    let format_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();
    let format_header = gtk::Label::builder()
        .label("FORMAT")
        .halign(gtk::Align::Start)
        .css_classes(["hs-qlabel"])
        .build();
    let fseg = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(3)
        .homogeneous(true)
        .css_classes(["hs-qseg"])
        .build();
    let formats = [
        (crate::capture::record::RecordingFormat::Mp4, "MP4"),
        (crate::capture::record::RecordingFormat::Webm, "WEBM"),
    ];
    let format_buttons: Vec<gtk::ToggleButton> = formats
        .iter()
        .map(|(_, text)| {
            let b = gtk::ToggleButton::builder()
                .label(*text)
                .css_classes(["hs-qseg-btn"])
                .build();
            b.set_can_focus(false);
            fseg.append(&b);
            b
        })
        .collect();
    for (button, (value, _)) in format_buttons.iter().zip(formats.iter()) {
        button.set_active(*value == format_cell.get());
        let format_cell = format_cell.clone();
        let value = *value;
        let all = format_buttons.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                format_cell.set(value);
                for other in &all {
                    if other != b {
                        other.set_active(false);
                    }
                }
            } else if format_cell.get() == value {
                b.set_active(true);
            }
        });
    }
    format_row.append(&format_header);
    format_row.append(&fseg);

    // Audio toggle
    let audio_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .build();
    let audio_text = quick_setting_text("Audio", "microphone via wf-recorder");
    let audio_switch = gtk::Switch::builder()
        .active(audio_cell.get())
        .halign(gtk::Align::End)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .css_classes(["hs-switch"])
        .build();
    audio_switch.set_can_focus(false);
    let audio_cell_for_switch = audio_cell.clone();
    audio_switch.connect_active_notify(move |switch| {
        audio_cell_for_switch.set(switch.is_active());
    });
    audio_row.append(&audio_text);
    audio_row.append(&audio_switch);

    // Recording HUD row
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

    hud_toggle.connect_active_notify(glib::clone!(
        #[strong]
        show_recording_hud,
        move |switch| {
            *show_recording_hud.borrow_mut() = switch.is_active();
        }
    ));

    rec_rows.append(&format_row);
    rec_rows.append(&audio_row);
    rec_rows.append(&hud_row);
    rec_rows.set_visible(default_is_record);

    qbody.append(&delay_row);
    qbody.append(&pointer_row);
    qbody.append(&rec_rows);
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
        #[weak(rename_to = rec_rows)]
        rec_rows,
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
                rec_rows.set_visible(false);
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
        #[weak(rename_to = rec_rows)]
        rec_rows,
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
                rec_rows.set_visible(true);
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
        #[strong]
        format_cell,
        #[strong]
        audio_cell,
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
                format_cell.get(),
                &audio_cell,
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
                gtk::gdk::Key::question | gtk::gdk::Key::slash => {
                    super::cheatsheet::toggle()
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
            let (x, y) = dock_position(&mon, w, h);
            crate::hyprland::place_window_exact("Hyprscreen", x, y);
            crate::hyprland::preposition_window("Hyprscreen", x, y);
        }
    });
}

fn dock_position(mon: &crate::hyprland::Monitor, w: i32, h: i32) -> (i32, i32) {
    let x = mon.x + ((mon.width - w) / 2).max(0);
    let y = mon.y + (mon.height - h - 34).max(0);
    (x, y)
}

/// Re-presents the dock after it was hidden for a capture. Re-mapping is a
/// fresh map to the compositor, so the preposition rule must still exist —
/// refresh it first in case a config reload dropped it.
fn present_dock(window: &gtk::ApplicationWindow) {
    crate::hyprland::refresh_preposition("Hyprscreen");
    window.present();
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

            let countdown_region = match target {
                Target::Monitor => monitor_region_by_name(&selection),
                _ => parse_region(&selection),
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

                            present_dock(&window2);
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
            let cancel_window = window.clone();
            let cancel_fire = fire_button.clone();
            countdown::run(delay_secs, countdown_region, false, proceed, move || {
                present_dock(&cancel_window);
                cancel_fire.set_sensitive(true);
            });

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
    format: crate::capture::record::RecordingFormat,
    audio_cell: &Rc<Cell<bool>>,
    fire_button: &gtk::Button,
) {
    let window = window.clone();
    let recording_state = recording_state.clone();
    let setup_cta = setup_cta.clone();
    let fire_button = fire_button.clone();
    let audio_cell = audio_cell.clone();

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

            let placement = match &selection {
                crate::capture::record::RecordingSelection::Geometry { monitor, .. } => *monitor,
                crate::capture::record::RecordingSelection::OutputName { placement, .. } => {
                    *placement
                }
            };
            let countdown_region =
                Some((placement.x, placement.y, placement.width, placement.height));

            // Phase 2: wait for one compositor frame, then launch wf-recorder.
            // launch_recording is fast (spawn + file write) so runs on the GTK thread.
            let window2 = window.clone();
            let recording_state2 = recording_state.clone();
            let setup_cta2 = setup_cta.clone();
            let fire_button2 = fire_button.clone();
            let audio_cell2 = audio_cell.clone();

            let proceed = move || {
                wait_compositor_frame(move || {
                    let audio = audio_cell2.get();
                    match crate::capture::record::launch_recording(selection, format, audio) {
                        Err(error) => {
                            report_capture_error(
                                "Recording failed",
                                &error,
                                &window2,
                                &fire_button2,
                            );
                        }
                        Ok(session) => {
                            let monitor = session.monitor;
                            let hud_window = if show_hud {
                                Some(create_recording_hud(
                                    &recording_state2,
                                    &audio_cell2,
                                    monitor,
                                ))
                            } else {
                                None
                            };

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
                                child: Some(session.child),
                                base_path: session.temp_path.clone(),
                                temp_path: session.temp_path,
                                segments: Vec::new(),
                                spec: session.spec,
                                seg_index: 0,
                                hud_window,
                                indicator_window,
                                accumulated: Duration::ZERO,
                                segment_started: Some(Instant::now()),
                                pending: PendingAction::None,
                            });

                            #[cfg(feature = "webcam")]
                            if crate::config::get().webcam_enabled {
                                super::webcam::toggle(monitor);
                            }

                            start_recording_poll(&window2, &recording_state2, &setup_cta2);
                        }
                    }
                })
            };
            let cancel_window = window.clone();
            let cancel_fire = fire_button.clone();
            countdown::run(delay_secs, countdown_region, true, proceed, move || {
                present_dock(&cancel_window);
                cancel_fire.set_sensitive(true);
            });

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

        // `hyprscreen stop` while paused: there is no live segment process,
        // so the CLI leaves a stop request for us instead (ADR-0016).
        if active.child.is_none() {
            if crate::capture::record::take_stop_request() {
                let finished = borrowed.take().expect("active recording disappeared");
                drop(borrowed);
                finish_recording(finished, &window, &setup_cta);
                return glib::ControlFlow::Break;
            }
            return glib::ControlFlow::Continue;
        }

        let wait_result = active.child.as_mut().expect("checked above").try_wait();
        match wait_result {
            Ok(Some(status)) => match active.pending {
                PendingAction::Pause => {
                    active.child = None;
                    active.pending = PendingAction::None;
                    active.segments.push(active.temp_path.clone());
                    crate::capture::record::mark_state_paused();
                    glib::ControlFlow::Continue
                }
                PendingAction::Restart => {
                    active.pending = PendingAction::None;
                    let _ = std::fs::remove_file(&active.temp_path);
                    for segment in active.segments.drain(..) {
                        let _ = std::fs::remove_file(segment);
                    }
                    active.seg_index = 0;
                    active.accumulated = Duration::ZERO;
                    match crate::capture::record::spawn_segment(&active.spec, &active.base_path) {
                        Ok(child) => {
                            crate::capture::record::mark_state_resumed(
                                child.id(),
                                &active.base_path,
                            );
                            active.temp_path = active.base_path.clone();
                            active.child = Some(child);
                            active.segment_started = Some(Instant::now());
                            glib::ControlFlow::Continue
                        }
                        Err(error) => {
                            let finished =
                                borrowed.take().expect("active recording disappeared");
                            drop(borrowed);
                            close_recording_windows(&finished);
                            crate::capture::record::clear_state_file();
                            present_dock(&window);
                            enable_setup_cta(&setup_cta);
                            toast::error("Restart failed", &error.to_string());
                            glib::ControlFlow::Break
                        }
                    }
                }
                PendingAction::None => {
                    let mut finished = borrowed.take().expect("active recording disappeared");
                    drop(borrowed);
                    finished.segments.push(finished.temp_path.clone());

                    let has_output = finished.segments.iter().any(|p| p.exists());
                    if !has_output && !status.success() {
                        close_recording_windows(&finished);
                        crate::capture::record::clear_state_file();
                        present_dock(&window);
                        enable_setup_cta(&setup_cta);
                        toast::error(
                            "Recording failed",
                            "the recording was cancelled or crashed",
                        );
                        return glib::ControlFlow::Break;
                    }

                    finish_recording(finished, &window, &setup_cta);
                    glib::ControlFlow::Break
                }
            },
            Ok(None) => glib::ControlFlow::Continue,
            Err(error) => {
                drop(borrowed);
                crate::capture::record::clear_state_file();
                present_dock(&window);
                enable_setup_cta(&setup_cta);
                toast::error("Recording poll failed", &error.to_string());
                glib::ControlFlow::Break
            }
        }
    });
}

fn close_recording_windows(finished: &ActiveRecording) {
    if let Some(hud) = &finished.hud_window {
        hud.close();
    }
    if let Some(indicator) = &finished.indicator_window {
        indicator.close();
    }
    super::draw_overlay::stop();
    #[cfg(feature = "webcam")]
    super::webcam::stop();
}

/// Joins the recorded segments, then hands the result to the thumbnail card.
fn finish_recording(
    finished: ActiveRecording,
    window: &gtk::ApplicationWindow,
    setup_cta: &Rc<RefCell<Option<gtk::Button>>>,
) {
    close_recording_windows(&finished);
    crate::capture::record::clear_state_file();
    present_dock(&window);
    enable_setup_cta(setup_cta);

    let format = finished.spec.format;
    match crate::capture::record::finalize_segments(finished.segments, format)
        .and_then(thumbnail::for_recording)
    {
        Ok(info) => thumbnail::show(info),
        Err(error) => toast::error("Recording preview failed", &error.to_string()),
    }
}

/// Parses a slurp-style "x,y wxh" geometry into a region tuple.
fn parse_region(geometry: &str) -> Option<(i32, i32, i32, i32)> {
    let (pos, size) = geometry.trim().split_once(' ')?;
    let (x, y) = pos.split_once(',')?;
    let (w, h) = size.split_once('x')?;
    Some((
        x.trim().parse().ok()?,
        y.trim().parse().ok()?,
        w.trim().parse().ok()?,
        h.trim().parse().ok()?,
    ))
}

fn monitor_region_by_name(name: &str) -> Option<(i32, i32, i32, i32)> {
    crate::hyprland::enumerate_monitors()
        .into_iter()
        .find(|m| m.name == name)
        .map(|m| (m.x, m.y, m.width, m.height))
}

fn report_capture_error(
    prefix: &str,
    error: &anyhow::Error,
    window: &gtk::ApplicationWindow,
    fire_button: &gtk::Button,
) {
    present_dock(window);
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

fn create_recording_hud(
    recording_state: &Rc<RefCell<Option<ActiveRecording>>>,
    audio_cell: &Rc<Cell<bool>>,
    monitor: crate::capture::record::MonitorPlacement,
) -> gtk::Window {
    let full = crate::config::get().hud_style == crate::config::HudStyle::Full;

    let hud = gtk::Window::builder()
        .title("Hyprscreen HUD")
        .decorated(false)
        .resizable(false)
        .build();
    hud.add_css_class("hs-hud-window");

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(if full { 4 } else { 12 })
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["hs-hud"])
        .build();

    let rec_dot = gtk::Box::builder()
        .width_request(9)
        .height_request(9)
        .valign(gtk::Align::Center)
        .css_classes(["hs-hud-dot"])
        .build();
    let rec_label = gtk::Label::builder()
        .label("REC")
        .css_classes(["hs-hud-rec"])
        .build();
    let counter = gtk::Label::builder()
        .label("00:00")
        .css_classes(["hs-hud-timer"])
        .build();

    let status_cluster = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(9)
        .margin_end(if full { 6 } else { 0 })
        .valign(gtk::Align::Center)
        .build();
    status_cluster.append(&rec_dot);
    status_cluster.append(&rec_label);
    status_cluster.append(&counter);
    content.append(&status_cluster);
    content.append(&make_dock_divider());

    if full {
        // Pause / resume (segmented recording, ADR-0016)
        let pause_button = hud_button("pause", "Pause");
        let pause_state = recording_state.clone();
        let audio_for_resume = audio_cell.clone();
        let dot_for_pause = rec_dot.clone();
        let label_for_pause = rec_label.clone();
        pause_button.connect_clicked(move |button| {
            let mut borrowed = pause_state.borrow_mut();
            let Some(active) = borrowed.as_mut() else {
                return;
            };
            if let Some(child) = &active.child {
                if active.pending != PendingAction::None {
                    return;
                }
                // Pause: end the current segment; the poll reaps it.
                active.pending = PendingAction::Pause;
                if let Some(started) = active.segment_started.take() {
                    active.accumulated += started.elapsed();
                }
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGINT);
                }
                dot_for_pause.add_css_class("paused");
                label_for_pause.set_label("PAUSED");
                set_hud_button_icon(button, "play");
                button.set_tooltip_text(Some("Resume"));
            } else {
                // Resume: spawn the next segment with identical parameters
                // (audio may have been toggled meanwhile).
                active.spec.audio = audio_for_resume.get();
                active.seg_index += 1;
                let path =
                    crate::capture::record::segment_path(&active.base_path, active.seg_index);
                match crate::capture::record::spawn_segment(&active.spec, &path) {
                    Ok(child) => {
                        crate::capture::record::mark_state_resumed(child.id(), &path);
                        active.temp_path = path;
                        active.child = Some(child);
                        active.segment_started = Some(Instant::now());
                        dot_for_pause.remove_css_class("paused");
                        label_for_pause.set_label("REC");
                        set_hud_button_icon(button, "pause");
                        button.set_tooltip_text(Some("Pause"));
                    }
                    Err(error) => {
                        drop(borrowed);
                        toast::error("Resume failed", &error.to_string());
                    }
                }
            }
        });
        content.append(&pause_button);

        // Restart: drop everything recorded so far, start over.
        let restart_button = hud_button("restart", "Restart recording");
        let restart_state = recording_state.clone();
        let dot_for_restart = rec_dot.clone();
        let label_for_restart = rec_label.clone();
        let pause_for_restart = pause_button.clone();
        restart_button.connect_clicked(move |_| {
            let mut borrowed = restart_state.borrow_mut();
            let Some(active) = borrowed.as_mut() else {
                return;
            };
            active.accumulated = Duration::ZERO;
            if let Some(child) = &active.child {
                active.pending = PendingAction::Restart;
                active.segment_started = Some(Instant::now());
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGINT);
                }
            } else {
                // Paused: restart synchronously.
                let _ = std::fs::remove_file(&active.temp_path);
                for segment in active.segments.drain(..) {
                    let _ = std::fs::remove_file(segment);
                }
                active.seg_index = 0;
                match crate::capture::record::spawn_segment(&active.spec, &active.base_path) {
                    Ok(child) => {
                        crate::capture::record::mark_state_resumed(child.id(), &active.base_path);
                        active.temp_path = active.base_path.clone();
                        active.child = Some(child);
                        active.segment_started = Some(Instant::now());
                        dot_for_restart.remove_css_class("paused");
                        label_for_restart.set_label("REC");
                        set_hud_button_icon(&pause_for_restart, "pause");
                        pause_for_restart.set_tooltip_text(Some("Pause"));
                    }
                    Err(error) => {
                        drop(borrowed);
                        toast::error("Restart failed", &error.to_string());
                    }
                }
            }
        });
        content.append(&restart_button);

        // Mic reflects the shared audio flag; wf-recorder cannot re-route
        // audio mid-segment, so changes apply from the next pause/resume.
        let mic_button = hud_button(
            if audio_cell.get() { "mic" } else { "mic-off" },
            "Microphone · applies after pause/resume",
        );
        if !audio_cell.get() {
            mic_button.add_css_class("off");
        }
        let audio_for_mic = audio_cell.clone();
        mic_button.connect_clicked(move |button| {
            let enabled = !audio_for_mic.get();
            audio_for_mic.set(enabled);
            set_hud_button_icon(button, if enabled { "mic" } else { "mic-off" });
            if enabled {
                button.remove_css_class("off");
            } else {
                button.add_css_class("off");
            }
        });
        content.append(&mic_button);

        #[cfg(feature = "webcam")]
        {
            let cam_button = hud_button("cam", "Webcam bubble");
            if crate::config::get().webcam_enabled {
                cam_button.add_css_class("on");
            }
            cam_button.connect_clicked(move |button| {
                super::webcam::toggle(monitor);
                if super::webcam::is_active() {
                    button.add_css_class("on");
                    set_hud_button_icon(button, "cam");
                } else {
                    button.remove_css_class("on");
                    set_hud_button_icon(button, "cam-off");
                }
            });
            content.append(&cam_button);
        }
        #[cfg(not(feature = "webcam"))]
        {
            let cam_button = hud_button("cam", "Webcam — built without the webcam feature");
            cam_button.set_sensitive(false);
            content.append(&cam_button);
        }
        let draw_button = hud_button("draw", "Draw on screen · Esc exits");
        draw_button.connect_clicked(move |button| {
            super::draw_overlay::toggle(monitor);
            if super::draw_overlay::is_active() {
                button.add_css_class("on");
            } else {
                button.remove_css_class("on");
            }
        });
        content.append(&draw_button);

        content.append(&make_dock_divider());
    }

    let stop = gtk::Button::builder().css_classes(["hs-hud-stop"]).build();
    if full {
        stop.set_margin_start(4);
    }
    set_button_text(&stop, "STOP");
    stop.connect_clicked(move |_| {
        let _ = crate::capture::record::stop_active_recording();
    });
    content.append(&stop);
    hud.set_child(Some(&content));

    let recording_state_for_timer = recording_state.clone();
    glib::timeout_add_local(Duration::from_millis(500), move || {
        let borrowed = recording_state_for_timer.borrow();
        let Some(active) = borrowed.as_ref() else {
            return glib::ControlFlow::Break;
        };
        let elapsed = active.elapsed().as_secs();
        let m = elapsed / 60;
        let s = elapsed % 60;
        counter.set_label(&format!("{m:02}:{s:02}"));
        glib::ControlFlow::Continue
    });

    // Pre-position from the measured size so the HUD maps at the top of the
    // recorded monitor instead of flashing at screen center; position_hud
    // then corrects with the realized size.
    let (_, nat_w, _, _) = content.measure(gtk::Orientation::Horizontal, -1);
    let (x, y) = hud_position(&monitor, nat_w);
    crate::hyprland::preposition_window("Hyprscreen HUD", x, y);
    hud.present();
    crate::hyprland::make_window_plain("Hyprscreen HUD");
    position_hud(&hud, monitor);
    hud
}

fn hud_button(icon_key: &str, tip: &str) -> gtk::Button {
    let button = gtk::Button::builder()
        .width_request(36)
        .height_request(36)
        .valign(gtk::Align::Center)
        .css_classes(["hs-hb"])
        .build();
    button.set_tooltip_text(Some(tip));
    let icon = icon_image(icon_key, 17, None);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    button.set_child(Some(&icon));
    button
}

fn set_hud_button_icon(button: &gtk::Button, icon_key: &str) {
    if let Some(image) = button
        .child()
        .and_then(|child| child.downcast::<gtk::Image>().ok())
    {
        image.set_paintable(Some(&icon_texture(icon_key, 17, "#EDEEF2")));
    }
}

fn position_hud(window: &gtk::Window, monitor: crate::capture::record::MonitorPlacement) {
    let window = window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        let (w, _h) = (window.width(), window.height());
        if w <= 1 {
            return;
        }
        let (x, y) = hud_position(&monitor, w);
        crate::hyprland::place_window_exact("Hyprscreen HUD", x, y);
    });
}

fn hud_position(monitor: &crate::capture::record::MonitorPlacement, w: i32) -> (i32, i32) {
    let x = monitor.x + ((monitor.width - w) / 2).max(0);
    let y = monitor.y + 18;
    (x, y)
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

            let x = monitor.x + (monitor.width - 180) / 2;
            let y = monitor.y + (monitor.height - 96) / 2;
            crate::hyprland::preposition_window(&title, x, y);
            window.present();
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

    let x = monitor.x + ((monitor.width - 16) / 2).max(0);
    let y = monitor.y + monitor.height - 16 - 20;
    crate::hyprland::preposition_window("Hyprscreen Recording Indicator", x, y);
    indicator.present();
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
