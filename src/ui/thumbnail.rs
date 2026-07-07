//! Post-capture corner thumbnail card (ADR-0013).
//!
//! One card at the top-right of the focused monitor, single-slot: a new
//! capture replaces the current card. With `autosave = true` (default) the
//! artifact is saved before the card appears and the card self-dismisses
//! after `thumbnail_timeout_seconds`; with `autosave = false` the artifact
//! stays a temp file ("pinned") until Save, Discard, or Close.

use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::Duration;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use super::icons::icon_image_colored;
use super::toast;

const TITLE: &str = "Hyprscreen Thumbnail";
const CARD_WIDTH: i32 = 268;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThumbKind {
    Screenshot,
    Recording,
}

pub struct ThumbInfo {
    pub kind: ThumbKind,
    /// The artifact (temp file when pinned, saved file when auto-saved).
    pub file_path: PathBuf,
    /// Image rendered in the preview area (video thumbnail for recordings).
    pub display_path: Option<PathBuf>,
    /// `display_path` is a temp file the card must clean up on close.
    pub display_is_temp: bool,
    /// "PNG · 1920×1200 · 284 KB" — without the saved/pinned tail.
    pub meta: String,
    pub saved: bool,
}

struct CardData {
    info: ThumbInfo,
}

thread_local! {
    #[allow(clippy::type_complexity)]
    static CURRENT: RefCell<Option<(gtk::Window, Rc<RefCell<CardData>>)>> =
        const { RefCell::new(None) };
}

/// Builds a `ThumbInfo` for a fresh screenshot, applying auto-save semantics.
pub fn for_screenshot(temp_path: PathBuf) -> anyhow::Result<ThumbInfo> {
    let (file_path, saved) = apply_autosave(temp_path, true)?;
    let meta = screenshot_meta(&file_path);
    Ok(ThumbInfo {
        kind: ThumbKind::Screenshot,
        display_path: None,
        display_is_temp: false,
        meta,
        file_path,
        saved,
    })
}

/// Builds a `ThumbInfo` for a finished recording, applying auto-save semantics.
pub fn for_recording(temp_path: PathBuf) -> anyhow::Result<ThumbInfo> {
    let preview = crate::capture::record::build_video_preview_info(&temp_path).ok();
    let (file_path, saved) = apply_autosave(temp_path, false)?;
    Ok(ThumbInfo {
        kind: ThumbKind::Recording,
        display_path: preview.as_ref().and_then(|p| p.thumbnail_path.clone()),
        display_is_temp: true,
        meta: preview
            .map(|p| p.metadata_summary)
            .unwrap_or_else(|| file_name_of(&file_path)),
        file_path,
        saved,
    })
}

fn apply_autosave(temp_path: PathBuf, is_screenshot: bool) -> anyhow::Result<(PathBuf, bool)> {
    let config = crate::config::get();
    if !config.autosave {
        return Ok((temp_path, false));
    }
    let destination = save_artifact(&temp_path, is_screenshot)?;
    let _ = std::fs::remove_file(&temp_path);
    Ok((destination, true))
}

fn save_artifact(source: &Path, is_screenshot: bool) -> anyhow::Result<PathBuf> {
    let save_dir = if is_screenshot {
        crate::config::get().save_dir_screenshots.clone()
    } else {
        crate::config::get().save_dir_recordings.clone()
    };
    std::fs::create_dir_all(&save_dir)?;
    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("temporary file path has no file name"))?;
    let destination = save_dir.join(file_name);
    if *source != destination {
        std::fs::copy(source, &destination)?;
    }
    Ok(destination)
}

pub fn show(info: ThumbInfo) {
    dismiss(true);

    let window = gtk::Window::builder()
        .title(TITLE)
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("hs-thumb-window");

    let data = Rc::new(RefCell::new(CardData { info }));

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .width_request(CARD_WIDTH)
        .css_classes(["hs-thumb"])
        .build();

    let (kind, saved) = {
        let d = data.borrow();
        (d.info.kind, d.info.saved)
    };

    // ── Preview area ───────────────────────────────────────────
    let overlay = gtk::Overlay::new();
    let picture = gtk::Picture::builder()
        .width_request(CARD_WIDTH)
        .height_request(CARD_WIDTH * 10 / 16)
        .content_fit(gtk::ContentFit::Cover)
        .css_classes(["hs-thumb-preview"])
        .build();
    {
        let d = data.borrow();
        let shown = d.info.display_path.as_ref().unwrap_or(&d.info.file_path);
        picture.set_file(Some(&gio::File::for_path(shown)));
    }
    overlay.set_child(Some(&picture));

    let badge = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .margin_top(9)
        .margin_start(9)
        .css_classes(["hs-thumb-badge"])
        .build();
    let badge_dot = gtk::Box::builder()
        .width_request(6)
        .height_request(6)
        .valign(gtk::Align::Center)
        .css_classes(["hs-thumb-bdot"])
        .build();
    let badge_label = gtk::Label::builder()
        .label(match kind {
            ThumbKind::Screenshot => "SCREENSHOT",
            ThumbKind::Recording => "RECORDING",
        })
        .css_classes(["hs-thumb-badge-label"])
        .build();
    if kind == ThumbKind::Recording {
        badge.add_css_class("rec");
    }
    badge.append(&badge_dot);
    badge.append(&badge_label);
    overlay.add_overlay(&badge);

    let close_button = gtk::Button::builder()
        .width_request(22)
        .height_request(22)
        .halign(gtk::Align::End)
        .valign(gtk::Align::Start)
        .margin_top(8)
        .margin_end(8)
        .css_classes(["hs-thumb-close"])
        .build();
    close_button.set_child(Some(&icon_image_colored("close", 12, None, "#9A9CA6")));
    overlay.add_overlay(&close_button);

    if kind == ThumbKind::Recording {
        let play_button = gtk::Button::builder()
            .width_request(40)
            .height_request(40)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(["hs-thumb-play"])
            .build();
        let play_icon = icon_image_colored("play", 14, None, "#FFFFFF");
        play_icon.set_margin_start(2);
        play_button.set_child(Some(&play_icon));
        let data_for_play = data.clone();
        play_button.connect_clicked(move |_| {
            let path = data_for_play.borrow().info.file_path.clone();
            match crate::capture::record::open_video_file(&path) {
                Ok(method) => toast::success("Opening recording", &method.feedback_message()),
                Err(error) => toast::error("Open failed", &error.to_string()),
            }
        });
        overlay.add_overlay(&play_button);
    }

    root.append(&overlay);

    // ── Meta line ──────────────────────────────────────────────
    let meta_label = gtk::Label::builder()
        .halign(gtk::Align::Start)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(11)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["hs-thumb-meta"])
        .build();
    update_meta(&meta_label, &data.borrow().info);

    // ── Action row ─────────────────────────────────────────────
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(2)
        .homogeneous(true)
        .margin_start(8)
        .margin_end(8)
        .margin_top(2)
        .margin_bottom(2)
        .css_classes(["hs-thumb-bar"])
        .build();

    let annotate_button = thumb_button("pen", "Annotate", false);
    annotate_button.set_sensitive(kind == ThumbKind::Screenshot);
    if kind == ThumbKind::Recording {
        annotate_button.set_tooltip_text(Some("Annotation is for screenshots"));
    }
    let data_for_annotate = data.clone();
    let picture_for_annotate = picture.clone();
    let meta_for_annotate = meta_label.clone();
    annotate_button.connect_clicked(move |_| {
        let path = data_for_annotate.borrow().info.file_path.clone();
        let picture = picture_for_annotate.clone();
        let meta_label = meta_for_annotate.clone();
        let data = data_for_annotate.clone();
        super::annotate::open(&path, move |new_path| {
            // Refresh the card with the annotated result.
            picture.set_file(Some(&gio::File::for_path(new_path)));
            {
                let mut d = data.borrow_mut();
                d.info.meta = screenshot_meta(new_path);
            }
            update_meta(&meta_label, &data.borrow().info);
        });
    });
    bar.append(&annotate_button);

    let copy_button = thumb_button("copy", "Copy", true);
    if kind == ThumbKind::Recording {
        copy_button.add_css_class("rec");
    }
    let data_for_copy = data.clone();
    copy_button.connect_clicked(move |_| {
        let (path, kind, meta) = {
            let d = data_for_copy.borrow();
            (d.info.file_path.clone(), d.info.kind, d.info.meta.clone())
        };
        let result = match kind {
            ThumbKind::Screenshot => copy_image_file_to_clipboard(&path),
            ThumbKind::Recording => copy_uri_to_clipboard(&path),
        };
        match result {
            Ok(()) => toast::success("Copied to clipboard", &meta),
            Err(error) => toast::error("Copy failed", &error.to_string()),
        }
    });
    bar.append(&copy_button);

    let save_button = thumb_button("save", "Save", false);
    save_button.set_sensitive(!saved);
    let data_for_save = data.clone();
    let meta_for_save = meta_label.clone();
    let save_button_ref = save_button.clone();
    save_button.connect_clicked(move |_| {
        let (source, kind) = {
            let d = data_for_save.borrow();
            (d.info.file_path.clone(), d.info.kind)
        };
        match save_artifact(&source, kind == ThumbKind::Screenshot) {
            Ok(destination) => {
                let _ = std::fs::remove_file(&source);
                {
                    let mut d = data_for_save.borrow_mut();
                    d.info.file_path = destination.clone();
                    d.info.saved = true;
                }
                update_meta(&meta_for_save, &data_for_save.borrow().info);
                save_button_ref.set_sensitive(false);
                let reveal_path = destination.clone();
                toast::success_with(
                    "Saved",
                    &destination.display().to_string(),
                    "Reveal",
                    move || match crate::capture::record::reveal_in_file_manager(&reveal_path) {
                        Ok(method) => toast::success("Revealing", &method.feedback_message()),
                        Err(error) => toast::error("Reveal failed", &error.to_string()),
                    },
                );
            }
            Err(error) => toast::error("Save failed", &error.to_string()),
        }
    });
    bar.append(&save_button);

    match kind {
        ThumbKind::Screenshot => {
            let share_button = thumb_button("share", "Share · copies a pasteable file", false);
            let data_for_share = data.clone();
            share_button.connect_clicked(move |_| {
                let path = data_for_share.borrow().info.file_path.clone();
                match copy_uri_to_clipboard(&path) {
                    Ok(()) => toast::success("File ready to paste", &file_name_of(&path)),
                    Err(error) => toast::error("Share failed", &error.to_string()),
                }
            });
            bar.append(&share_button);
        }
        ThumbKind::Recording => {
            // GIF conversion replaces Share on video cards (ADR-0010: post-capture only).
            let gif_button = thumb_button("gif", "Convert to GIF", false);
            let data_for_gif = data.clone();
            gif_button.connect_clicked(move |button| {
                button.set_sensitive(false);
                let source = data_for_gif.borrow().info.file_path.clone();
                let (tx, rx) = std::sync::mpsc::channel::<anyhow::Result<PathBuf>>();
                std::thread::spawn(move || {
                    let _ = tx.send(crate::capture::record::convert_recording_to_gif(&source));
                });
                let button = button.clone();
                glib::timeout_add_local(Duration::from_millis(50), move || {
                    let result = match rx.try_recv() {
                        Ok(r) => r,
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            return glib::ControlFlow::Continue;
                        }
                        Err(_) => return glib::ControlFlow::Break,
                    };
                    button.set_sensitive(true);
                    match result.and_then(for_gif) {
                        Ok(info) => show(info),
                        Err(error) => toast::error("GIF failed", &error.to_string()),
                    }
                    glib::ControlFlow::Break
                });
            });
            bar.append(&gif_button);
        }
    }

    let discard_button = thumb_button("trash", "Discard", false);
    let data_for_discard = data.clone();
    discard_button.connect_clicked(move |_| {
        let path = data_for_discard.borrow().info.file_path.clone();
        let _ = std::fs::remove_file(&path);
        // The file is gone either way; don't let dismiss re-delete.
        data_for_discard.borrow_mut().info.saved = true;
        dismiss(true);
        toast::success("Discarded", &file_name_of(&path));
    });
    bar.append(&discard_button);

    root.append(&bar);
    root.append(&meta_label);

    close_button.connect_clicked(|_| dismiss(true));

    window.set_child(Some(&root));
    // Pre-position from the measured size so the card maps at its final
    // spot instead of flashing at screen center; position() then corrects
    // with the realized size if the measurement was off.
    let (_, nat_w, _, _) = root.measure(gtk::Orientation::Horizontal, -1);
    let (_, nat_h, _, _) = root.measure(gtk::Orientation::Vertical, nat_w);
    if let Some(mon) = crate::hyprland::focused_monitor() {
        let (x, y) = top_right(&mon, nat_w, nat_h);
        crate::hyprland::preposition_window(TITLE, x, y);
    }
    window.present();
    crate::hyprland::make_window_glass(TITLE, 16);
    position(&window);

    // Auto-saved cards self-dismiss; pinned cards wait for the user.
    let timeout = crate::config::get().thumbnail_timeout_seconds;
    if saved && timeout > 0 {
        let this = window.clone();
        glib::timeout_add_local_once(Duration::from_secs(timeout), move || {
            let is_current =
                CURRENT.with_borrow(|current| current.as_ref().map(|(w, _)| w) == Some(&this));
            if is_current {
                dismiss(false);
            }
        });
    }

    CURRENT.set(Some((window, data)));
}

/// Closes the current card. `delete_unsaved` removes a still-pinned temp file
/// (Close/replace act like the old preview `Back`); the auto-dismiss timer
/// passes `false` — although saved cards have nothing to delete anyway.
fn dismiss(delete_unsaved: bool) {
    if let Some((window, data)) = CURRENT.take() {
        {
            let d = data.borrow();
            if delete_unsaved && !d.info.saved {
                let _ = std::fs::remove_file(&d.info.file_path);
            }
            if d.info.display_is_temp {
                if let Some(display) = &d.info.display_path {
                    let _ = std::fs::remove_file(display);
                }
            }
        }
        window.close();
    }
}

fn for_gif(gif_path: PathBuf) -> anyhow::Result<ThumbInfo> {
    let (file_path, saved) = apply_autosave(gif_path, false)?;
    let meta = screenshot_meta(&file_path);
    Ok(ThumbInfo {
        kind: ThumbKind::Recording,
        display_path: Some(file_path.clone()),
        display_is_temp: false,
        meta,
        file_path,
        saved,
    })
}

fn thumb_button(icon_key: &str, tip: &str, accent: bool) -> gtk::Button {
    let button = gtk::Button::builder()
        .height_request(38)
        .css_classes(["hs-tbtn2"])
        .build();
    if accent {
        button.add_css_class("accent");
    }
    button.set_tooltip_text(Some(tip));
    let color = if accent { "#5EE6D0" } else { "#9A9CA6" };
    let icon = icon_image_colored(icon_key, 17, None, color);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    button.set_child(Some(&icon));
    button
}

fn update_meta(label: &gtk::Label, info: &ThumbInfo) {
    let state = if info.saved {
        r##"<span foreground="#6FD79E">auto-saved</span>"##
    } else {
        r##"<span foreground="#9A9CA6">pinned</span>"##
    };
    // Manually-saved cards show "saved"; the flag flips before this runs, so
    // distinguish by whether autosave is on.
    let state = if info.saved && !crate::config::get().autosave {
        r##"<span foreground="#6FD79E">saved</span>"##
    } else {
        state
    };
    label.set_markup(&format!(
        "{} · {}",
        glib::markup_escape_text(&info.meta),
        state
    ));
}

fn screenshot_meta(path: &Path) -> String {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_uppercase())
        .unwrap_or_else(|| "FILE".into());
    let dims = gtk::gdk_pixbuf::Pixbuf::file_info(path)
        .map(|(_, w, h)| format!(" · {w}×{h}"))
        .unwrap_or_default();
    let size = std::fs::metadata(path)
        .map(|m| format!(" · {}", human_size(m.len())))
        .unwrap_or_default();
    format!("{ext}{dims}{size}")
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub(crate) fn copy_image_file_to_clipboard(path: &Path) -> anyhow::Result<()> {
    let mut child = Command::new("wl-copy")
        .arg("--type")
        .arg("image/png")
        .stdin(Stdio::piped())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed to open wl-copy stdin"))?;
    stdin.write_all(&std::fs::read(path)?)?;
    drop(stdin);
    if !child.wait()?.success() {
        anyhow::bail!("wl-copy failed")
    }
    Ok(())
}

/// Copies a `text/uri-list` reference — pastes as an attachable *file* in
/// browsers and chat apps, unlike Copy which pastes raw image data.
fn copy_uri_to_clipboard(path: &Path) -> anyhow::Result<()> {
    let absolute = path.canonicalize()?;
    let status = Command::new("wl-copy")
        .arg("--type")
        .arg("text/uri-list")
        .arg(format!("file://{}", absolute.display()))
        .status()?;
    if !status.success() {
        anyhow::bail!("wl-copy failed")
    }
    Ok(())
}

fn position(window: &gtk::Window) {
    let window = window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        let (w, h) = (window.width(), window.height());
        if w <= 1 || h <= 1 {
            return;
        }
        if let Some(mon) = crate::hyprland::focused_monitor() {
            let (x, y) = top_right(&mon, w, h);
            crate::hyprland::place_window_exact(TITLE, x, y);
        }
    });
}

fn top_right(mon: &crate::hyprland::Monitor, w: i32, _h: i32) -> (i32, i32) {
    let x = mon.x + (mon.width - w - 22).max(0);
    let y = mon.y + 54;
    (x, y)
}
