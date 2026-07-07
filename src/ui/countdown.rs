//! Delay countdown overlay (ADR-0015).
//!
//! A centered circle with a large tick number and a "Cancel · Esc" pill on
//! the target monitor. Deliberately *not* a fullscreen dim: a fullscreen
//! window would grab input across the monitor and risk leaking into the
//! capture. The window closes one compositor-settle delay before the capture
//! fires so it never appears in the result.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

const TITLE: &str = "Hyprscreen Countdown";
/// Time between closing the overlay and firing the capture, so the
/// compositor has repainted the area underneath.
const SETTLE_MS: u64 = 120;

/// Runs a countdown centered in `region` (falls back to the focused
/// monitor), then calls `on_done`. Esc or the Cancel pill calls `on_cancel`.
pub fn run(
    seconds: u64,
    region: Option<(i32, i32, i32, i32)>,
    is_rec: bool,
    on_done: impl FnOnce() + 'static,
    on_cancel: impl FnOnce() + 'static,
) {
    if seconds == 0 {
        on_done();
        return;
    }

    let window = gtk::Window::builder()
        .title(TITLE)
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("hs-countdown-window");
    if is_rec {
        window.add_css_class("hs-mode-rec");
    }

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();

    let circle = gtk::Box::builder()
        .width_request(260)
        .height_request(260)
        .halign(gtk::Align::Center)
        .css_classes(["hs-cd-circle"])
        .build();
    let number = gtk::Label::builder()
        .label(seconds.to_string())
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .hexpand(true)
        .vexpand(true)
        .css_classes(["hs-cd-num"])
        .build();
    circle.append(&number);

    let cancel = gtk::Button::builder()
        .label("Cancel · Esc")
        .halign(gtk::Align::Center)
        .css_classes(["hs-cd-cancel"])
        .build();

    root.append(&circle);
    root.append(&cancel);
    window.set_child(Some(&root));
    window.present();
    crate::hyprland::make_window_plain(TITLE);
    position(&window, region);

    // FnOnce callbacks fire exactly once across tick/cancel paths.
    let done_slot: Rc<RefCell<Option<Box<dyn FnOnce()>>>> =
        Rc::new(RefCell::new(Some(Box::new(on_done))));
    let cancel_slot: Rc<RefCell<Option<Box<dyn FnOnce()>>>> =
        Rc::new(RefCell::new(Some(Box::new(on_cancel))));
    let remaining = Rc::new(Cell::new(seconds));

    let fire_cancel = {
        let window = window.clone();
        let cancel_slot = cancel_slot.clone();
        let done_slot = done_slot.clone();
        Rc::new(move || {
            // Disarm the tick by emptying the done slot.
            done_slot.borrow_mut().take();
            if let Some(cb) = cancel_slot.borrow_mut().take() {
                window.close();
                cb();
            }
        })
    };

    let fire_cancel_for_button = fire_cancel.clone();
    cancel.connect_clicked(move |_| fire_cancel_for_button());

    let key_controller = gtk::EventControllerKey::new();
    let fire_cancel_for_key = fire_cancel.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            fire_cancel_for_key();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    let tick_window = window.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        if done_slot.borrow().is_none() {
            // Cancelled.
            return glib::ControlFlow::Break;
        }
        let left = remaining.get().saturating_sub(1);
        remaining.set(left);
        if left > 0 {
            number.set_label(&left.to_string());
            return glib::ControlFlow::Continue;
        }

        tick_window.close();
        if let Some(display) = gtk::gdk::Display::default() {
            display.sync();
        }
        let done_slot = done_slot.clone();
        glib::timeout_add_local_once(Duration::from_millis(SETTLE_MS), move || {
            if let Some(cb) = done_slot.borrow_mut().take() {
                cb();
            }
        });
        glib::ControlFlow::Break
    });
}

fn position(window: &gtk::Window, region: Option<(i32, i32, i32, i32)>) {
    let window = window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        let (w, h) = (window.width(), window.height());
        if w <= 1 {
            return;
        }
        let Some((rx, ry, rw, rh)) = region.or_else(|| {
            crate::hyprland::focused_monitor().map(|m| (m.x, m.y, m.width, m.height))
        }) else {
            return;
        };
        let x = rx + ((rw - w) / 2).max(0);
        let y = ry + ((rh - h) / 2).max(0);
        crate::hyprland::place_window_exact(TITLE, x, y);
    });
}
