//! Draw-on-screen overlay while recording (ADR-0020).
//!
//! A monitor-sized transparent floating window with freehand cairo strokes.
//! wf-recorder captures whatever is on screen, so strokes appear in the
//! video. Drawing needs pointer input, so the overlay is modal on its
//! monitor while active — Esc or the HUD draw button ends it (strokes are
//! cleared on exit).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

const TITLE: &str = "Hyprscreen Draw";
const STROKE_WIDTH: f64 = 3.5;

thread_local! {
    static CURRENT: RefCell<Option<gtk::Window>> = const { RefCell::new(None) };
}

pub fn is_active() -> bool {
    CURRENT.with_borrow(|current| current.is_some())
}

pub fn stop() {
    if let Some(window) = CURRENT.take() {
        window.close();
    }
}

/// Toggles the draw overlay on the given monitor region.
pub fn toggle(monitor: crate::capture::record::MonitorPlacement) {
    if is_active() {
        stop();
        return;
    }

    let window = gtk::Window::builder()
        .title(TITLE)
        .decorated(false)
        .resizable(false)
        .default_width(monitor.width)
        .default_height(monitor.height)
        .build();
    window.add_css_class("hs-draw-window");

    let strokes: Rc<RefCell<Vec<Vec<(f64, f64)>>>> = Rc::new(RefCell::new(Vec::new()));

    let canvas = gtk::DrawingArea::builder()
        .content_width(monitor.width)
        .content_height(monitor.height)
        .build();

    let strokes_for_draw = strokes.clone();
    canvas.set_draw_func(move |_, cr, _, _| {
        // Coral ink — drawing only exists while recording.
        cr.set_source_rgb(1.0, 0.365, 0.365);
        cr.set_line_width(STROKE_WIDTH);
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        cr.set_line_join(gtk::cairo::LineJoin::Round);
        for stroke in strokes_for_draw.borrow().iter() {
            let mut points = stroke.iter();
            if let Some((x, y)) = points.next() {
                cr.move_to(*x, *y);
                for (x, y) in points {
                    cr.line_to(*x, *y);
                }
                let _ = cr.stroke();
            }
        }
    });

    let drag = gtk::GestureDrag::new();
    {
        let strokes = strokes.clone();
        let canvas = canvas.clone();
        drag.connect_drag_begin(move |_, x, y| {
            strokes.borrow_mut().push(vec![(x, y)]);
            canvas.queue_draw();
        });
    }
    {
        let strokes = strokes.clone();
        let canvas = canvas.clone();
        drag.connect_drag_update(move |gesture, dx, dy| {
            let Some((sx, sy)) = gesture.start_point() else {
                return;
            };
            if let Some(stroke) = strokes.borrow_mut().last_mut() {
                stroke.push((sx + dx, sy + dy));
            }
            canvas.queue_draw();
        });
    }
    canvas.add_controller(drag);

    let keys = gtk::EventControllerKey::new();
    {
        let strokes = strokes.clone();
        let canvas = canvas.clone();
        keys.connect_key_pressed(move |_, key, _, modifier| {
            if key == gtk::gdk::Key::Escape {
                stop();
                return glib::Propagation::Stop;
            }
            if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK)
                && (key == gtk::gdk::Key::z || key == gtk::gdk::Key::Z)
            {
                strokes.borrow_mut().pop();
                canvas.queue_draw();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    window.add_controller(keys);

    window.set_child(Some(&canvas));
    window.present();
    crate::hyprland::make_window_plain(TITLE);
    crate::hyprland::place_window_exact(TITLE, monitor.x, monitor.y);
    // Best effort: keep the overlay from drifting behind other windows.
    glib::timeout_add_local_once(Duration::from_millis(120), || {
        crate::hyprland::pin_window(TITLE);
    });

    CURRENT.set(Some(window));
}
