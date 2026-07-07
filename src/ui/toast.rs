//! Transient feedback toasts (ADR-0014).
//!
//! One borderless window bottom-center above the dock, single-slot: a new
//! toast replaces the current one. Success toasts auto-dismiss after 4s,
//! error toasts after 6s.

use std::cell::RefCell;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use super::icons::icon_image_colored;

const TITLE: &str = "Hyprscreen Toast";

thread_local! {
    static CURRENT: RefCell<Option<gtk::Window>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ToastKind {
    Ok,
    Err,
}

pub fn success(title: &str, subtitle: &str) {
    show(ToastKind::Ok, title, subtitle, None);
}

pub fn success_with(title: &str, subtitle: &str, action: &str, on_click: impl Fn() + 'static) {
    show(ToastKind::Ok, title, subtitle, Some((action, Box::new(on_click))));
}

pub fn error(title: &str, subtitle: &str) {
    show(ToastKind::Err, title, subtitle, None);
}

pub fn error_with(title: &str, subtitle: &str, action: &str, on_click: impl Fn() + 'static) {
    show(
        ToastKind::Err,
        title,
        subtitle,
        Some((action, Box::new(on_click))),
    );
}

pub fn dismiss() {
    if let Some(window) = CURRENT.take() {
        window.close();
    }
}

fn show(kind: ToastKind, title: &str, subtitle: &str, action: Option<(&str, Box<dyn Fn()>)>) {
    dismiss();

    let window = gtk::Window::builder()
        .title(TITLE)
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("hs-toast-window");
    if crate::config::get().dock_style == crate::config::DockStyle::Solid {
        window.remove_css_class("hs-toast-window");
        window.add_css_class("hs-toast-window-solid");
    }

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .css_classes(["hs-toast"])
        .build();

    let (icon_key, icon_color, icon_class) = match kind {
        ToastKind::Ok => ("check", "#6FD79E", "ok"),
        ToastKind::Err => ("alert", "#FF5D5D", "err"),
    };
    let icon_chip = gtk::Box::builder()
        .width_request(30)
        .height_request(30)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["hs-toast-ico", icon_class])
        .build();
    let icon = icon_image_colored(icon_key, 17, None, icon_color);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    icon.set_hexpand(true);
    icon_chip.append(&icon);
    row.append(&icon_chip);

    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(3)
        .valign(gtk::Align::Center)
        .build();
    let title_label = gtk::Label::builder()
        .label(title)
        .halign(gtk::Align::Start)
        .css_classes(["hs-toast-title"])
        .build();
    body.append(&title_label);
    if !subtitle.is_empty() {
        let sub_label = gtk::Label::builder()
            .label(subtitle)
            .halign(gtk::Align::Start)
            .max_width_chars(42)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .css_classes(["hs-toast-sub"])
            .build();
        body.append(&sub_label);
    }
    row.append(&body);

    if let Some((label, on_click)) = action {
        let button = gtk::Button::builder()
            .label(label)
            .valign(gtk::Align::Center)
            .css_classes(["hs-toast-action", icon_class])
            .build();
        button.connect_clicked(move |_| {
            on_click();
            dismiss();
        });
        row.append(&button);
    }

    window.set_child(Some(&row));
    // Pre-position from the measured size so the toast maps at its final
    // spot instead of flashing at screen center; position() then corrects
    // with the realized size if the measurement was off.
    let (_, nat_w, _, _) = row.measure(gtk::Orientation::Horizontal, -1);
    let (_, nat_h, _, _) = row.measure(gtk::Orientation::Vertical, nat_w);
    if let Some(mon) = crate::hyprland::focused_monitor() {
        let (x, y) = bottom_center(&mon, nat_w, nat_h);
        crate::hyprland::preposition_window(TITLE, x, y);
    }
    window.present();
    crate::hyprland::make_window_glass(TITLE, 12);
    position(&window);

    let lifetime = match kind {
        ToastKind::Ok => Duration::from_secs(4),
        ToastKind::Err => Duration::from_secs(6),
    };
    let this = window.clone();
    glib::timeout_add_local_once(lifetime, move || {
        let is_current = CURRENT.with_borrow(|current| current.as_ref() == Some(&this));
        if is_current {
            dismiss();
        }
    });

    CURRENT.set(Some(window));
}

fn position(window: &gtk::Window) {
    let window = window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        let (w, h) = (window.width(), window.height());
        if w <= 1 {
            return;
        }
        if let Some(mon) = crate::hyprland::focused_monitor() {
            let (x, y) = bottom_center(&mon, w, h);
            crate::hyprland::place_window_exact(TITLE, x, y);
        }
    });
}

fn bottom_center(mon: &crate::hyprland::Monitor, w: i32, h: i32) -> (i32, i32) {
    let x = mon.x + ((mon.width - w) / 2).max(0);
    // Sits above the dock (dock height ~76 + 34 margin + 12 gap).
    let y = mon.y + (mon.height - h - 130).max(0);
    (x, y)
}
