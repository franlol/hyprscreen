//! Keyboard shortcuts cheatsheet — `?` on the dock.

use std::cell::RefCell;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use super::icons::icon_image_colored;

const TITLE: &str = "Hyprscreen Shortcuts";

thread_local! {
    static CURRENT: RefCell<Option<gtk::Window>> = const { RefCell::new(None) };
}

const DOCK_ROWS: [(&str, &[&str]); 8] = [
    ("Fire capture / record", &["⏎"]),
    ("Target: area · window · monitor", &["1", "2", "3"]),
    ("Screenshot / record mode", &["S", "R"]),
    ("Cycle delay", &["D"]),
    ("Toggle pointer", &["P"]),
    ("Shortcuts", &["?"]),
    ("Quit", &["Esc"]),
    ("Cancel selection / countdown", &["Esc"]),
];

const HYPRLAND_ROWS: [(&str, &str); 5] = [
    ("Capture area", "SUPER SHIFT + 4"),
    ("Capture window", "SUPER SHIFT + 5"),
    ("Capture monitor", "SUPER SHIFT + 3"),
    ("Record area", "SUPER SHIFT + R"),
    ("Stop recording", "SUPER SHIFT + X"),
];

pub fn toggle() {
    let already_open = CURRENT.with_borrow(|current| current.is_some());
    if already_open {
        dismiss();
        return;
    }

    let window = gtk::Window::builder()
        .title(TITLE)
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("hs-cheat-window");

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(14)
        .width_request(420)
        .css_classes(["hs-cheat"])
        .build();

    let heading = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(9)
        .build();
    heading.append(&icon_image_colored("keyboard", 17, None, "#5EE6D0"));
    let heading_label = gtk::Label::builder()
        .label("Keyboard shortcuts")
        .css_classes(["hs-cheat-title"])
        .build();
    heading.append(&heading_label);
    root.append(&heading);

    let grid = gtk::Grid::builder()
        .row_spacing(9)
        .column_spacing(22)
        .column_homogeneous(true)
        .build();
    for (index, (label, keys)) in DOCK_ROWS.iter().enumerate() {
        let row = cheat_row(label, keys);
        grid.attach(&row, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    root.append(&grid);

    let hypr_header = gtk::Label::builder()
        .label("SUGGESTED HYPRLAND BINDS")
        .halign(gtk::Align::Start)
        .css_classes(["hs-qlabel"])
        .build();
    root.append(&hypr_header);

    let hypr_grid = gtk::Grid::builder()
        .row_spacing(9)
        .column_spacing(22)
        .column_homogeneous(true)
        .build();
    for (index, (label, bind)) in HYPRLAND_ROWS.iter().enumerate() {
        let row = cheat_row(label, &bind.split(" + ").collect::<Vec<_>>());
        hypr_grid.attach(&row, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    root.append(&hypr_grid);

    let hint = gtk::Label::builder()
        .label("bind = SUPER SHIFT, 4, exec, hyprscreen screenshot area")
        .halign(gtk::Align::Start)
        .css_classes(["hs-cheat-hint"])
        .build();
    root.append(&hint);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(|_, key, _, _| {
        if key == gtk::gdk::Key::Escape || key == gtk::gdk::Key::question {
            dismiss();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(keys);

    window.set_child(Some(&root));
    // Pre-position from the measured size so the sheet maps centered
    // instead of flashing at the compositor's default spot; position()
    // then corrects with the realized size.
    let (_, nat_w, _, _) = root.measure(gtk::Orientation::Horizontal, -1);
    let (_, nat_h, _, _) = root.measure(gtk::Orientation::Vertical, nat_w);
    if let Some(mon) = crate::hyprland::focused_monitor() {
        let x = mon.x + ((mon.width - nat_w) / 2).max(0);
        let y = mon.y + ((mon.height - nat_h) / 2).max(0);
        crate::hyprland::preposition_window(TITLE, x, y);
    }
    window.present();
    crate::hyprland::make_window_glass(TITLE, 16);
    position(&window);
    CURRENT.set(Some(window));
}

pub fn dismiss() {
    if let Some(window) = CURRENT.take() {
        window.close();
    }
}

fn cheat_row(label: &str, keys: &[&str]) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    let name = gtk::Label::builder()
        .label(label)
        .halign(gtk::Align::Start)
        .hexpand(true)
        .wrap(false)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["hs-cheat-label"])
        .build();
    row.append(&name);
    let key_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .build();
    for key in keys {
        let kbd = gtk::Label::builder()
            .label(*key)
            .css_classes(["hs-kbd"])
            .build();
        key_box.append(&kbd);
    }
    row.append(&key_box);
    row
}

fn position(window: &gtk::Window) {
    let window = window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        let (w, h) = (window.width(), window.height());
        if w <= 1 {
            return;
        }
        if let Some(mon) = crate::hyprland::focused_monitor() {
            let x = mon.x + ((mon.width - w) / 2).max(0);
            let y = mon.y + ((mon.height - h) / 2).max(0);
            crate::hyprland::place_window_exact(TITLE, x, y);
        }
    });
}
