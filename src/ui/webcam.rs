//! Webcam bubble (ADR-0019) — feature `webcam`.
//!
//! A small circular talking-head window shown while recording, fed by a
//! GStreamer pipeline (`v4l2src ! videoconvert ! gtk4paintablesink`). The
//! circle comes from Hyprland's corner rounding on the window surface, so
//! the recording captures it round too. Requires the `gst-plugin-gtk4`
//! runtime package and a camera; failures degrade to an explanatory toast.

use std::cell::RefCell;
use std::sync::Once;
use std::time::Duration;

use gstreamer as gst;
use gstreamer::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use super::toast;

const TITLE: &str = "Hyprscreen Camera";

thread_local! {
    static CURRENT: RefCell<Option<(gtk::Window, gst::Pipeline)>> = const { RefCell::new(None) };
}

static GST_INIT: Once = Once::new();

pub fn is_active() -> bool {
    CURRENT.with_borrow(|current| current.is_some())
}

pub fn stop() {
    if let Some((window, pipeline)) = CURRENT.take() {
        let _ = pipeline.set_state(gst::State::Null);
        window.close();
    }
}

/// Toggles the bubble on the given monitor region.
pub fn toggle(monitor: crate::capture::record::MonitorPlacement) {
    if is_active() {
        stop();
        return;
    }
    if let Err(error) = start(monitor) {
        toast::error(
            "Webcam unavailable",
            &format!("{error} — check webcam_device and the gst-plugin-gtk4 package"),
        );
    }
}

fn start(monitor: crate::capture::record::MonitorPlacement) -> anyhow::Result<()> {
    GST_INIT.call_once(|| {
        let _ = gst::init();
    });

    let config = crate::config::get();
    let size = config.webcam_size.clamp(64, 512) as i32;

    let source = gst::ElementFactory::make("v4l2src")
        .property("device", &config.webcam_device)
        .build()
        .map_err(|_| anyhow::anyhow!("v4l2src element unavailable"))?;
    let convert = gst::ElementFactory::make("videoconvert")
        .build()
        .map_err(|_| anyhow::anyhow!("videoconvert element unavailable"))?;
    let sink = gst::ElementFactory::make("gtk4paintablesink")
        .build()
        .map_err(|_| anyhow::anyhow!("gtk4paintablesink element unavailable"))?;

    let pipeline = gst::Pipeline::new();
    pipeline.add_many([&source, &convert, &sink])?;
    gst::Element::link_many([&source, &convert, &sink])?;

    let paintable = sink.property::<gtk::gdk::Paintable>("paintable");

    let window = gtk::Window::builder()
        .title(TITLE)
        .decorated(false)
        .resizable(false)
        .default_width(size)
        .default_height(size)
        .build();
    window.add_css_class("hs-webcam-window");

    let overlay = gtk::Overlay::new();
    let picture = gtk::Picture::builder()
        .width_request(size)
        .height_request(size)
        .content_fit(gtk::ContentFit::Cover)
        .build();
    picture.set_paintable(Some(&paintable));
    overlay.set_child(Some(&picture));

    let badge = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .margin_top(10)
        .margin_start(14)
        .css_classes(["hs-webcam-live"])
        .build();
    let dot = gtk::Box::builder()
        .width_request(5)
        .height_request(5)
        .valign(gtk::Align::Center)
        .css_classes(["hs-webcam-live-dot"])
        .build();
    let label = gtk::Label::builder()
        .label("LIVE")
        .css_classes(["hs-webcam-live-label"])
        .build();
    badge.append(&dot);
    badge.append(&label);
    overlay.add_overlay(&badge);
    window.set_child(Some(&overlay));

    // Surface errors from the bus (missing camera shows up here).
    let bus = pipeline.bus().expect("pipeline has a bus");
    let _ = bus.add_watch_local(move |_, message| {
        if let gst::MessageView::Error(error) = message.view() {
            toast::error("Webcam error", &error.error().to_string());
            stop();
        }
        glib::ControlFlow::Continue
    });

    pipeline
        .set_state(gst::State::Playing)
        .map_err(|_| anyhow::anyhow!("failed to start the camera pipeline"))?;

    window.present();
    // Hyprland rounds the surface to a circle — the recording sees it round.
    crate::hyprland::make_window_glass(TITLE, (size / 2) as u32);
    position(&window, monitor, size);
    glib::timeout_add_local_once(Duration::from_millis(150), || {
        crate::hyprland::pin_window(TITLE);
    });

    // Drag anywhere on the bubble to move it.
    let drag = gtk::GestureDrag::new();
    let accumulated = std::rc::Rc::new(std::cell::Cell::new((0.0_f64, 0.0_f64)));
    {
        let accumulated = accumulated.clone();
        drag.connect_drag_begin(move |_, _, _| accumulated.set((0.0, 0.0)));
    }
    drag.connect_drag_update(move |_, dx, dy| {
        let (ax, ay) = accumulated.get();
        let (step_x, step_y) = (dx - ax, dy - ay);
        if step_x.abs() >= 1.0 || step_y.abs() >= 1.0 {
            crate::hyprland::move_window_relative(TITLE, step_x as i32, step_y as i32);
            accumulated.set((dx, dy));
        }
    });
    window.add_controller(drag);

    CURRENT.set(Some((window, pipeline)));
    Ok(())
}

fn position(window: &gtk::Window, monitor: crate::capture::record::MonitorPlacement, size: i32) {
    const MARGIN: i32 = 34;
    let config = crate::config::get();
    let (x, y) = match config.webcam_position.as_str() {
        "bottom-right" => (
            monitor.x + monitor.width - size - MARGIN,
            monitor.y + monitor.height - size - MARGIN,
        ),
        "top-left" => (monitor.x + MARGIN, monitor.y + MARGIN),
        "top-right" => (monitor.x + monitor.width - size - MARGIN, monitor.y + MARGIN),
        // default: bottom-left
        _ => (monitor.x + MARGIN, monitor.y + monitor.height - size - MARGIN),
    };
    let window = window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        if window.width() <= 1 {
            return;
        }
        crate::hyprland::place_window_exact(TITLE, x, y);
    });
}
