//! Embedded SVG icon rasterization shared by all UI surfaces.
//!
//! v2 icons are authored with `stroke="currentColor"`; librsvg has no CSS
//! context here, so the color is baked in before rasterization (2× render
//! + `set_pixel_size` for sharp output, per docs/scopes/ui.md).

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

pub(crate) fn icon_image(icon_key: &str, size: i32, css_class: Option<&str>) -> gtk::Image {
    icon_image_colored(icon_key, size, css_class, "#EDEEF2")
}

pub(crate) fn icon_image_colored(
    icon_key: &str,
    size: i32,
    css_class: Option<&str>,
    color: &str,
) -> gtk::Image {
    let image = gtk::Image::from_paintable(Some(&icon_texture(icon_key, size, color)));
    image.set_pixel_size(size);
    if let Some(css_class) = css_class {
        image.add_css_class(css_class);
    }
    image
}

pub(crate) fn icon_texture(icon_key: &str, size: i32, color: &str) -> gtk::gdk::Texture {
    let svg = String::from_utf8_lossy(icon_bytes(icon_key)).replace("currentColor", color);
    let bytes = glib::Bytes::from_owned(svg.into_bytes());
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    let render = size * 2;
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_stream_at_scale(
        &stream,
        render,
        render,
        true,
        gio::Cancellable::NONE,
    )
    .expect("failed to rasterize embedded SVG");
    gtk::gdk::Texture::for_pixbuf(&pixbuf)
}

fn icon_bytes(icon_key: &str) -> &'static [u8] {
    match icon_key {
        "area" => include_bytes!("../../assets/icons/area.svg"),
        "window" => include_bytes!("../../assets/icons/window.svg"),
        "monitor" => include_bytes!("../../assets/icons/monitor.svg"),
        "back" => include_bytes!("../../assets/icons/back.svg"),
        "refresh" => include_bytes!("../../assets/icons/refresh.svg"),
        "save" => include_bytes!("../../assets/icons/save.svg"),
        "copy" => include_bytes!("../../assets/icons/copy.svg"),
        "reveal" => include_bytes!("../../assets/icons/reveal.svg"),
        "open" => include_bytes!("../../assets/icons/open.svg"),
        "gif" => include_bytes!("../../assets/icons/gif.svg"),
        "shutter" => include_bytes!("../../assets/icons/shutter.svg"),
        "shot" => include_bytes!("../../assets/icons/shot.svg"),
        "rec" => include_bytes!("../../assets/icons/rec.svg"),
        "timer" => include_bytes!("../../assets/icons/timer.svg"),
        "pointer" => include_bytes!("../../assets/icons/pointer.svg"),
        "chevron" => include_bytes!("../../assets/icons/chevron.svg"),
        "close" => include_bytes!("../../assets/icons/close.svg"),
        "pen" => include_bytes!("../../assets/icons/pen.svg"),
        "share" => include_bytes!("../../assets/icons/share.svg"),
        "trash" => include_bytes!("../../assets/icons/trash.svg"),
        "play" => include_bytes!("../../assets/icons/play.svg"),
        "arrow" => include_bytes!("../../assets/icons/arrow.svg"),
        "box" => include_bytes!("../../assets/icons/box.svg"),
        "text" => include_bytes!("../../assets/icons/text.svg"),
        "blur" => include_bytes!("../../assets/icons/blur.svg"),
        "highlight" => include_bytes!("../../assets/icons/highlight.svg"),
        "step" => include_bytes!("../../assets/icons/step.svg"),
        "undo" => include_bytes!("../../assets/icons/undo.svg"),
        "pause" => include_bytes!("../../assets/icons/pause.svg"),
        "restart" => include_bytes!("../../assets/icons/restart.svg"),
        "mic" => include_bytes!("../../assets/icons/mic.svg"),
        "mic-off" => include_bytes!("../../assets/icons/mic-off.svg"),
        "cam" => include_bytes!("../../assets/icons/cam.svg"),
        "cam-off" => include_bytes!("../../assets/icons/cam-off.svg"),
        "draw" => include_bytes!("../../assets/icons/draw.svg"),
        "keyboard" => include_bytes!("../../assets/icons/keyboard.svg"),
        "alert" => include_bytes!("../../assets/icons/alert.svg"),
        "check" => include_bytes!("../../assets/icons/check.svg"),
        _ => include_bytes!("../../assets/icons/area.svg"),
    }
}
