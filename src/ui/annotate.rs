//! Annotation editor (ADR-0018).
//!
//! A cairo shape-list editor over the captured image: arrow, free draw, box,
//! text, step counter, highlight, and pixelate-blur tools with five ink colors,
//! undo, Copy (clipboard) and Done (overwrite the file). Geometry is stored
//! in image coordinates, so export replays the same shapes at native
//! resolution.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gdk_pixbuf::{InterpType, Pixbuf};
use gtk::glib;
use gtk::prelude::*;

use super::icons::icon_image_colored;
use super::toast;

const TITLE: &str = "Hyprscreen Annotate";
const MAX_VIEW_W: f64 = 960.0;
const MAX_VIEW_H: f64 = 600.0;

pub const INK_COLORS: [&str; 5] = ["#5EE6D0", "#FF5D5D", "#FFD23F", "#7CA8FF", "#FFFFFF"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tool {
    Select,
    Arrow,
    Draw,
    Box,
    Text,
    Step,
    Highlight,
    Blur,
}

#[derive(Clone, Debug)]
enum Shape {
    Arrow { a: (f64, f64), b: (f64, f64), color: [f64; 3] },
    Path { points: Vec<(f64, f64)>, color: [f64; 3] },
    Rect { rect: (f64, f64, f64, f64), color: [f64; 3] },
    Highlight { rect: (f64, f64, f64, f64), color: [f64; 3] },
    Text { pos: (f64, f64), text: String, color: [f64; 3] },
    Step { pos: (f64, f64), n: u32, color: [f64; 3] },
    Blur { rect: (f64, f64, f64, f64) },
}

struct EditorState {
    base: Pixbuf,
    shapes: Vec<Shape>,
    draft: Option<Shape>,
    tool: Tool,
    color: [f64; 3],
    /// view px = image px × scale
    scale: f64,
    /// Stroke width in image coordinates (≈3.5 view px).
    stroke: f64,
    drag: Option<(usize, f64, f64)>,
    blur_cache: HashMap<(i32, i32, i32, i32), Pixbuf>,
}

thread_local! {
    static CURRENT: RefCell<Option<gtk::Window>> = const { RefCell::new(None) };
}

/// Floating text entry currently open on the canvas overlay.
/// `x`/`y` are the view coordinates of the click that spawned it.
struct ActiveText {
    view: gtk::TextView,
    x: f64,
    y: f64,
}

type ActiveTextCell = Rc<RefCell<Option<ActiveText>>>;

fn parse_hex(color: &str) -> [f64; 3] {
    let c = color.trim_start_matches('#');
    let component = |i: usize| {
        u8::from_str_radix(&c[i..i + 2], 16)
            .map(|v| v as f64 / 255.0)
            .unwrap_or(1.0)
    };
    [component(0), component(2), component(4)]
}

/// Opens the editor for `path`. `on_done` runs after Done overwrites the file.
pub fn open(path: &Path, on_done: impl Fn(&Path) + 'static) {
    if let Some(previous) = CURRENT.take() {
        previous.close();
    }

    let base = match Pixbuf::from_file(path) {
        Ok(pixbuf) => pixbuf,
        Err(error) => {
            toast::error("Annotate failed", &error.to_string());
            return;
        }
    };
    let (img_w, img_h) = (base.width() as f64, base.height() as f64);
    let scale = (MAX_VIEW_W / img_w).min(MAX_VIEW_H / img_h).min(1.0);
    let (view_w, view_h) = ((img_w * scale).round(), (img_h * scale).round());

    let state = Rc::new(RefCell::new(EditorState {
        base,
        shapes: Vec::new(),
        draft: None,
        tool: Tool::Arrow,
        color: parse_hex(crate::config::get().annotate_default_color.as_str()),
        scale,
        stroke: 3.5 / scale,
        drag: None,
        blur_cache: HashMap::new(),
    }));
    let active_text: ActiveTextCell = Rc::new(RefCell::new(None));

    let window = gtk::Window::builder()
        .title(TITLE)
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("hs-annot-window");

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    // ── Header ─────────────────────────────────────────────────
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(["hs-annot-top"])
        .build();
    let title = gtk::Label::builder().css_classes(["hs-annot-title"]).build();
    title.set_markup("Annotate · <span foreground=\"#5EE6D0\">screenshot</span>");
    let header_spacer = gtk::Box::builder().hexpand(true).build();
    let close_button = gtk::Button::builder()
        .width_request(24)
        .height_request(24)
        .valign(gtk::Align::Center)
        .css_classes(["hs-annot-close"])
        .build();
    close_button.set_child(Some(&icon_image_colored("close", 13, None, "#9A9CA6")));
    header.append(&title);
    header.append(&header_spacer);
    header.append(&close_button);
    root.append(&header);

    // ── Main: tool rail + canvas ───────────────────────────────
    let main = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();

    let rail = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(3)
        .css_classes(["hs-annot-tools"])
        .build();

    let canvas = gtk::DrawingArea::builder()
        .content_width(view_w as i32)
        .content_height(view_h as i32)
        .build();

    let tools: [(Tool, &str, &str); 8] = [
        (Tool::Select, "pointer", "Select / move · V"),
        (Tool::Arrow, "arrow", "Arrow · A"),
        (Tool::Draw, "draw", "Draw · D"),
        (Tool::Box, "box", "Box · B"),
        (Tool::Text, "text", "Text · T"),
        (Tool::Step, "step", "Step counter · N"),
        (Tool::Highlight, "highlight", "Highlight · H"),
        (Tool::Blur, "blur", "Blur · L"),
    ];
    let tool_buttons: Vec<gtk::ToggleButton> = tools
        .iter()
        .map(|(tool, icon, tip)| {
            let button = gtk::ToggleButton::builder()
                .width_request(38)
                .height_request(38)
                .css_classes(["hs-atool"])
                .build();
            button.set_can_focus(false);
            button.set_tooltip_text(Some(tip));
            let image = icon_image_colored(icon, 18, None, "#9A9CA6");
            image.set_halign(gtk::Align::Center);
            image.set_valign(gtk::Align::Center);
            button.set_child(Some(&image));
            button.set_active(*tool == Tool::Arrow);
            rail.append(&button);
            button
        })
        .collect();

    let rail_divider = gtk::Box::builder()
        .height_request(1)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(6)
        .margin_end(6)
        .css_classes(["hs-ddiv"])
        .build();
    rail.append(&rail_divider);

    let undo_button = gtk::Button::builder()
        .width_request(38)
        .height_request(38)
        .css_classes(["hs-atool"])
        .build();
    undo_button.set_can_focus(false);
    undo_button.set_tooltip_text(Some("Undo · Ctrl+Z"));
    let undo_icon = icon_image_colored("undo", 18, None, "#9A9CA6");
    undo_icon.set_halign(gtk::Align::Center);
    undo_icon.set_valign(gtk::Align::Center);
    undo_button.set_child(Some(&undo_icon));
    rail.append(&undo_button);

    main.append(&rail);

    // Overlay hosts the floating text entry.
    let canvas_overlay = gtk::Overlay::new();
    canvas_overlay.set_child(Some(&canvas));
    main.append(&canvas_overlay);
    root.append(&main);

    for (button, (tool, _, _)) in tool_buttons.iter().zip(tools.iter()) {
        let state = state.clone();
        let tool = *tool;
        let all = tool_buttons.clone();
        let active_text = active_text.clone();
        let overlay = canvas_overlay.clone();
        let canvas = canvas.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                commit_active_text(&active_text, &overlay, &canvas, &state);
                state.borrow_mut().tool = tool;
                for other in &all {
                    if other != b {
                        other.set_active(false);
                    }
                }
            } else if state.borrow().tool == tool {
                b.set_active(true);
            }
        });
    }

    // ── Footer ─────────────────────────────────────────────────
    let footer = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(["hs-annot-foot"])
        .build();
    let swatch_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .valign(gtk::Align::Center)
        .build();
    let swatch_buttons: Vec<gtk::ToggleButton> = INK_COLORS
        .iter()
        .enumerate()
        .map(|(index, hex)| {
            let button = gtk::ToggleButton::builder()
                .width_request(18)
                .height_request(18)
                .valign(gtk::Align::Center)
                .css_classes(["hs-af-sw", &format!("sw{index}")])
                .build();
            button.set_can_focus(false);
            button.set_tooltip_text(Some(*hex));
            button.set_active(
                parse_hex(hex) == parse_hex(&crate::config::get().annotate_default_color),
            );
            swatch_row.append(&button);
            button
        })
        .collect();
    for (button, hex) in swatch_buttons.iter().zip(INK_COLORS.iter()) {
        let state = state.clone();
        let color = parse_hex(hex);
        let all = swatch_buttons.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                state.borrow_mut().color = color;
                for other in &all {
                    if other != b {
                        other.set_active(false);
                    }
                }
            } else if state.borrow().color == color {
                b.set_active(true);
            }
        });
    }
    if !swatch_buttons.iter().any(|b| b.is_active()) {
        swatch_buttons[0].set_active(true);
    }

    let footer_spacer = gtk::Box::builder().hexpand(true).build();
    let copy_button = gtk::Button::builder()
        .label("Copy")
        .css_classes(["hs-af-btn", "ghost"])
        .build();
    let done_button = gtk::Button::builder()
        .label("Done")
        .css_classes(["hs-af-btn", "solid"])
        .build();
    footer.append(&swatch_row);
    footer.append(&footer_spacer);
    footer.append(&copy_button);
    footer.append(&done_button);
    root.append(&footer);

    // ── Rendering ──────────────────────────────────────────────
    let state_for_draw = state.clone();
    canvas.set_draw_func(move |_, cr, _, _| {
        let mut s = state_for_draw.borrow_mut();
        let scale = s.scale;
        cr.scale(scale, scale);
        draw_scene(cr, &mut s);
    });

    // ── Input: draw / drag ─────────────────────────────────────
    let drag = gtk::GestureDrag::new();
    {
        let state = state.clone();
        let canvas = canvas.clone();
        drag.connect_drag_begin(move |_, x, y| {
            let mut s = state.borrow_mut();
            let scale = s.scale;
            let (ix, iy) = (x / scale, y / scale);
            let color = s.color;
            match s.tool {
                Tool::Arrow => {
                    s.draft = Some(Shape::Arrow { a: (ix, iy), b: (ix, iy), color });
                }
                Tool::Draw => {
                    s.draft = Some(Shape::Path { points: vec![(ix, iy)], color });
                }
                Tool::Box => {
                    s.draft = Some(Shape::Rect { rect: (ix, iy, 0.0, 0.0), color });
                }
                Tool::Highlight => {
                    s.draft = Some(Shape::Highlight { rect: (ix, iy, 0.0, 0.0), color });
                }
                Tool::Blur => {
                    s.draft = Some(Shape::Blur { rect: (ix, iy, 0.0, 0.0) });
                }
                Tool::Select => {
                    s.drag = hit_test(&s.shapes, ix, iy).map(|index| (index, ix, iy));
                }
                Tool::Text | Tool::Step => {}
            }
            drop(s);
            canvas.queue_draw();
        });
    }
    {
        let state = state.clone();
        let canvas = canvas.clone();
        drag.connect_drag_update(move |gesture, dx, dy| {
            let Some((sx, sy)) = gesture.start_point() else {
                return;
            };
            let mut s = state.borrow_mut();
            let scale = s.scale;
            let (ix, iy) = ((sx + dx) / scale, (sy + dy) / scale);
            let (ox, oy) = (sx / scale, sy / scale);
            match &mut s.draft {
                Some(Shape::Arrow { b, .. }) => *b = (ix, iy),
                Some(Shape::Path { points, .. }) => points.push((ix, iy)),
                Some(Shape::Rect { rect, .. })
                | Some(Shape::Highlight { rect, .. })
                | Some(Shape::Blur { rect }) => {
                    *rect = normalized_rect(ox, oy, ix, iy);
                }
                _ => {}
            }
            if let Some((index, lx, ly)) = s.drag {
                let (mdx, mdy) = (ix - lx, iy - ly);
                if let Some(shape) = s.shapes.get_mut(index) {
                    move_shape(shape, mdx, mdy);
                }
                s.drag = Some((index, ix, iy));
            }
            drop(s);
            canvas.queue_draw();
        });
    }
    {
        let state = state.clone();
        let canvas = canvas.clone();
        drag.connect_drag_end(move |_, _, _| {
            let mut s = state.borrow_mut();
            if let Some(draft) = s.draft.take() {
                if draft_is_meaningful(&draft) {
                    s.shapes.push(draft);
                }
            }
            s.drag = None;
            drop(s);
            canvas.queue_draw();
        });
    }
    canvas.add_controller(drag);

    // ── Input: click for text / step ───────────────────────────
    let click = gtk::GestureClick::new();
    {
        let state = state.clone();
        let canvas = canvas.clone();
        let overlay = canvas_overlay.clone();
        let active_text = active_text.clone();
        click.connect_pressed(move |_, _, x, y| {
            let tool = state.borrow().tool;
            match tool {
                Tool::Step => {
                    let mut s = state.borrow_mut();
                    let scale = s.scale;
                    let n = s
                        .shapes
                        .iter()
                        .filter(|shape| matches!(shape, Shape::Step { .. }))
                        .count() as u32
                        + 1;
                    let color = s.color;
                    s.shapes.push(Shape::Step { pos: (x / scale, y / scale), n, color });
                    drop(s);
                    canvas.queue_draw();
                }
                Tool::Text => {
                    spawn_text_entry(&overlay, &canvas, &state, &active_text, x, y);
                }
                _ => {}
            }
        });
    }
    canvas.add_controller(click);

    // ── Undo / shortcuts ───────────────────────────────────────
    let undo = {
        let state = state.clone();
        let canvas = canvas.clone();
        Rc::new(move || {
            state.borrow_mut().shapes.pop();
            canvas.queue_draw();
        })
    };
    let undo_for_button = undo.clone();
    undo_button.connect_clicked(move |_| undo_for_button());

    let keys = gtk::EventControllerKey::new();
    {
        let undo = undo.clone();
        let tool_buttons = tool_buttons.clone();
        let window_for_keys = window.clone();
        keys.connect_key_pressed(move |_, key, _, modifier| {
            if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK)
                && (key == gtk::gdk::Key::z || key == gtk::gdk::Key::Z)
            {
                undo();
                return glib::Propagation::Stop;
            }
            let tool_index = match key {
                gtk::gdk::Key::v | gtk::gdk::Key::V => Some(0),
                gtk::gdk::Key::a | gtk::gdk::Key::A => Some(1),
                gtk::gdk::Key::d | gtk::gdk::Key::D => Some(2),
                gtk::gdk::Key::b | gtk::gdk::Key::B => Some(3),
                gtk::gdk::Key::t | gtk::gdk::Key::T => Some(4),
                gtk::gdk::Key::n | gtk::gdk::Key::N => Some(5),
                gtk::gdk::Key::h | gtk::gdk::Key::H => Some(6),
                gtk::gdk::Key::l | gtk::gdk::Key::L => Some(7),
                gtk::gdk::Key::Escape => {
                    window_for_keys.close();
                    return glib::Propagation::Stop;
                }
                _ => None,
            };
            if let Some(index) = tool_index {
                tool_buttons[index].set_active(true);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    window.add_controller(keys);

    // ── Copy / Done ────────────────────────────────────────────
    let file_path = path.to_path_buf();
    {
        let state = state.clone();
        let active_text = active_text.clone();
        let overlay = canvas_overlay.clone();
        let canvas = canvas.clone();
        copy_button.connect_clicked(move |_| {
            commit_active_text(&active_text, &overlay, &canvas, &state);
            match export_to_temp(&state) {
                Ok(temp) => {
                    let result = super::thumbnail::copy_image_file_to_clipboard(&temp);
                    let _ = std::fs::remove_file(&temp);
                    match result {
                        Ok(()) => toast::success("Copied to clipboard", "annotated image"),
                        Err(error) => toast::error("Copy failed", &error.to_string()),
                    }
                }
                Err(error) => toast::error("Copy failed", &error.to_string()),
            }
        });
    }
    {
        let state = state.clone();
        let window = window.clone();
        let on_done = Rc::new(on_done);
        let active_text = active_text.clone();
        let overlay = canvas_overlay.clone();
        let canvas = canvas.clone();
        done_button.connect_clicked(move |_| {
            commit_active_text(&active_text, &overlay, &canvas, &state);
            match export_to(&state, &file_path) {
                Ok(()) => {
                    on_done(&file_path);
                    toast::success("Annotations saved", &file_path.display().to_string());
                    window.close();
                }
                Err(error) => toast::error("Save failed", &error.to_string()),
            }
        });
    }

    {
        let window = window.clone();
        close_button.connect_clicked(move |_| window.close());
    }

    window.set_child(Some(&root));
    // Pre-position from the measured size so the editor maps centered
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

/// Commits the floating text entry, if any: pushes its text as a
/// `Shape::Text` (blank text is dropped) and removes the widget.
fn commit_active_text(
    active: &ActiveTextCell,
    overlay: &gtk::Overlay,
    canvas: &gtk::DrawingArea,
    state: &Rc<RefCell<EditorState>>,
) {
    let Some(ActiveText { view, x, y }) = active.borrow_mut().take() else {
        return;
    };
    let buffer = view.buffer();
    let text = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .trim_end()
        .to_string();
    if !text.trim().is_empty() {
        let mut s = state.borrow_mut();
        let scale = s.scale;
        let color = s.color;
        s.shapes.push(Shape::Text { pos: (x / scale, (y / scale) + 18.0 / scale), text, color });
    }
    overlay.remove_overlay(&view);
    canvas.queue_draw();
}

fn spawn_text_entry(
    overlay: &gtk::Overlay,
    canvas: &gtk::DrawingArea,
    state: &Rc<RefCell<EditorState>>,
    active: &ActiveTextCell,
    x: f64,
    y: f64,
) {
    commit_active_text(active, overlay, canvas, state);

    // A TextView (not an Entry) so the box grows with its content and
    // Shift+Enter can insert new lines.
    let view = gtk::TextView::builder()
        .css_classes(["hs-annot-entry"])
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .margin_start(x as i32)
        .margin_top(y as i32)
        .width_request(160)
        .wrap_mode(gtk::WrapMode::None)
        .accepts_tab(false)
        .left_margin(8)
        .right_margin(8)
        .top_margin(5)
        .bottom_margin(5)
        .build();
    overlay.add_overlay(&view);
    active.replace(Some(ActiveText { view: view.clone(), x, y }));

    // TextView is a scrollable: its natural size ignores content, so size it
    // from the text's pango layout and grow as the buffer changes.
    let resize = {
        let view = view.clone();
        move || {
            let buffer = view.buffer();
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            let layout = view.create_pango_layout(Some(text.as_str()));
            let (text_w, text_h) = layout.pixel_size();
            // Margins, border, and caret slack around the content.
            view.set_size_request((text_w + 22).max(160), text_h + 12);
        }
    };
    resize();
    view.buffer().connect_changed(move |_| resize());

    let view_for_focus = view.clone();
    glib::idle_add_local_once(move || {
        view_for_focus.grab_focus();
    });

    let active = active.clone();
    let overlay = overlay.clone();
    let canvas = canvas.clone();
    let state = state.clone();
    let keys = gtk::EventControllerKey::new();
    // Capture phase: intercept Enter before the TextView inserts a newline.
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, modifier| match key {
        gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter
            if !modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK) =>
        {
            commit_active_text(&active, &overlay, &canvas, &state);
            glib::Propagation::Stop
        }
        gtk::gdk::Key::Escape => {
            // Esc aborts: remove without committing.
            if let Some(ActiveText { view, .. }) = active.borrow_mut().take() {
                overlay.remove_overlay(&view);
            }
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    view.add_controller(keys);
}

fn normalized_rect(ax: f64, ay: f64, bx: f64, by: f64) -> (f64, f64, f64, f64) {
    (ax.min(bx), ay.min(by), (bx - ax).abs(), (by - ay).abs())
}

fn draft_is_meaningful(shape: &Shape) -> bool {
    match shape {
        Shape::Arrow { a, b, .. } => (b.0 - a.0).hypot(b.1 - a.1) > 4.0,
        Shape::Path { points, .. } => points.len() > 2,
        Shape::Rect { rect, .. } | Shape::Highlight { rect, .. } | Shape::Blur { rect } => {
            rect.2 > 4.0 && rect.3 > 4.0
        }
        _ => true,
    }
}

fn shape_bbox(shape: &Shape) -> (f64, f64, f64, f64) {
    match shape {
        Shape::Arrow { a, b, .. } => normalized_rect(a.0, a.1, b.0, b.1),
        Shape::Path { points, .. } => {
            let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
            let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for (x, y) in points {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
            (min_x, min_y, max_x - min_x, max_y - min_y)
        }
        Shape::Rect { rect, .. } | Shape::Highlight { rect, .. } | Shape::Blur { rect } => *rect,
        Shape::Text { pos, text, .. } => {
            let lines = text.lines().count().max(1) as f64;
            let widest = text.lines().map(str::len).max().unwrap_or(0) as f64;
            (pos.0, pos.1 - 20.0, widest * 10.0, 6.0 + lines * 20.0)
        }
        Shape::Step { pos, .. } => (pos.0 - 13.0, pos.1 - 13.0, 26.0, 26.0),
    }
}

fn hit_test(shapes: &[Shape], x: f64, y: f64) -> Option<usize> {
    const PAD: f64 = 8.0;
    shapes.iter().enumerate().rev().find_map(|(index, shape)| {
        let (bx, by, bw, bh) = shape_bbox(shape);
        (x >= bx - PAD && x <= bx + bw + PAD && y >= by - PAD && y <= by + bh + PAD)
            .then_some(index)
    })
}

fn move_shape(shape: &mut Shape, dx: f64, dy: f64) {
    match shape {
        Shape::Arrow { a, b, .. } => {
            a.0 += dx;
            a.1 += dy;
            b.0 += dx;
            b.1 += dy;
        }
        Shape::Path { points, .. } => {
            for point in points {
                point.0 += dx;
                point.1 += dy;
            }
        }
        Shape::Rect { rect, .. } | Shape::Highlight { rect, .. } | Shape::Blur { rect } => {
            rect.0 += dx;
            rect.1 += dy;
        }
        Shape::Text { pos, .. } | Shape::Step { pos, .. } => {
            pos.0 += dx;
            pos.1 += dy;
        }
    }
}

/// Paints the base image and every shape in image coordinates. The caller
/// scales the context (view scale on screen, 1.0 for export).
fn draw_scene(cr: &gtk::cairo::Context, state: &mut EditorState) {
    use gtk::gdk::prelude::GdkCairoContextExt;

    cr.set_source_pixbuf(&state.base, 0.0, 0.0);
    let _ = cr.paint();

    let stroke = state.stroke;
    let font_size = 15.0 / state.scale;
    // Blur shapes render below inks, matching the mock.
    let blurs: Vec<(f64, f64, f64, f64)> = state
        .shapes
        .iter()
        .chain(state.draft.iter())
        .filter_map(|shape| match shape {
            Shape::Blur { rect } => Some(*rect),
            _ => None,
        })
        .collect();
    for rect in blurs {
        draw_blur(cr, state, rect);
    }

    let shapes: Vec<Shape> = state.shapes.iter().chain(state.draft.iter()).cloned().collect();
    for shape in &shapes {
        match shape {
            Shape::Blur { .. } => {}
            Shape::Arrow { a, b, color } => {
                cr.set_source_rgb(color[0], color[1], color[2]);
                cr.set_line_width(stroke);
                cr.set_line_cap(gtk::cairo::LineCap::Round);
                cr.move_to(a.0, a.1);
                cr.line_to(b.0, b.1);
                let _ = cr.stroke();
                // Filled triangular head at `b`.
                let angle = (b.1 - a.1).atan2(b.0 - a.0);
                let head = stroke * 4.0;
                cr.move_to(b.0, b.1);
                cr.line_to(
                    b.0 - head * (angle - 0.42).cos(),
                    b.1 - head * (angle - 0.42).sin(),
                );
                cr.line_to(
                    b.0 - head * (angle + 0.42).cos(),
                    b.1 - head * (angle + 0.42).sin(),
                );
                cr.close_path();
                let _ = cr.fill();
            }
            Shape::Path { points, color } => {
                let mut iter = points.iter();
                let Some((x, y)) = iter.next() else {
                    continue;
                };
                cr.set_source_rgb(color[0], color[1], color[2]);
                cr.set_line_width(stroke);
                cr.set_line_cap(gtk::cairo::LineCap::Round);
                cr.set_line_join(gtk::cairo::LineJoin::Round);
                cr.move_to(*x, *y);
                for (x, y) in iter {
                    cr.line_to(*x, *y);
                }
                let _ = cr.stroke();
            }
            Shape::Rect { rect, color } => {
                cr.set_source_rgb(color[0], color[1], color[2]);
                cr.set_line_width(stroke);
                rounded_rect(cr, rect.0, rect.1, rect.2, rect.3, stroke * 1.6);
                let _ = cr.stroke();
            }
            Shape::Highlight { rect, color } => {
                cr.set_source_rgba(color[0], color[1], color[2], 0.35);
                cr.rectangle(rect.0, rect.1, rect.2, rect.3);
                let _ = cr.fill();
            }
            Shape::Text { pos, text, color } => {
                cr.select_font_face(
                    "Cantarell",
                    gtk::cairo::FontSlant::Normal,
                    gtk::cairo::FontWeight::Bold,
                );
                cr.set_font_size(font_size);
                let line_height = font_size * 1.35;
                for (index, line) in text.lines().enumerate() {
                    let line_y = pos.1 + index as f64 * line_height;
                    // Soft shadow for legibility on any background.
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.55);
                    cr.move_to(pos.0 + 1.5 / state.scale, line_y + 1.5 / state.scale);
                    let _ = cr.show_text(line);
                    cr.set_source_rgb(color[0], color[1], color[2]);
                    cr.move_to(pos.0, line_y);
                    let _ = cr.show_text(line);
                }
            }
            Shape::Step { pos, n, color } => {
                let radius = 13.0 / state.scale;
                cr.set_source_rgb(color[0], color[1], color[2]);
                cr.arc(pos.0, pos.1, radius, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
                cr.set_source_rgb(0.03, 0.09, 0.08);
                cr.select_font_face(
                    "Cantarell",
                    gtk::cairo::FontSlant::Normal,
                    gtk::cairo::FontWeight::Bold,
                );
                cr.set_font_size(radius);
                let label = n.to_string();
                if let Ok(extents) = cr.text_extents(&label) {
                    cr.move_to(
                        pos.0 - extents.width() / 2.0 - extents.x_bearing(),
                        pos.1 + extents.height() / 2.0,
                    );
                    let _ = cr.show_text(&label);
                }
            }
        }
    }
}

fn rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 1.5 * std::f64::consts::PI);
    cr.close_path();
}

/// Pixelate: shrink the covered region ÷8 and scale it back up with nearest
/// neighbour. Deterministic, cheap, dependency-free.
fn draw_blur(cr: &gtk::cairo::Context, state: &mut EditorState, rect: (f64, f64, f64, f64)) {
    use gtk::gdk::prelude::GdkCairoContextExt;

    let (img_w, img_h) = (state.base.width(), state.base.height());
    let x = (rect.0.max(0.0) as i32).min(img_w - 1);
    let y = (rect.1.max(0.0) as i32).min(img_h - 1);
    let w = (rect.2 as i32).clamp(1, img_w - x);
    let h = (rect.3 as i32).clamp(1, img_h - y);
    if w < 8 || h < 8 {
        return;
    }

    let key = (x, y, w, h);
    if !state.blur_cache.contains_key(&key) {
        let sub = state.base.new_subpixbuf(x, y, w, h);
        let small = sub.scale_simple((w / 8).max(1), (h / 8).max(1), InterpType::Bilinear);
        let pixelated =
            small.and_then(|s| s.scale_simple(w, h, InterpType::Nearest));
        if let Some(pixelated) = pixelated {
            state.blur_cache.insert(key, pixelated);
        }
    }
    if let Some(pixelated) = state.blur_cache.get(&key) {
        cr.set_source_pixbuf(pixelated, x as f64, y as f64);
        let _ = cr.paint();
    }
}

fn export_pixbuf(state: &Rc<RefCell<EditorState>>) -> anyhow::Result<gtk::cairo::ImageSurface> {
    let mut s = state.borrow_mut();
    let (w, h) = (s.base.width(), s.base.height());
    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, w, h)
        .map_err(|e| anyhow::anyhow!("failed to create export surface: {e}"))?;
    let cr = gtk::cairo::Context::new(&surface)
        .map_err(|e| anyhow::anyhow!("failed to create export context: {e}"))?;
    draw_scene(&cr, &mut s);
    drop(cr);
    Ok(surface)
}

fn export_to(state: &Rc<RefCell<EditorState>>, path: &Path) -> anyhow::Result<()> {
    let surface = export_pixbuf(state)?;
    let mut file = std::fs::File::create(path)?;
    surface
        .write_to_png(&mut file)
        .map_err(|e| anyhow::anyhow!("failed to write PNG: {e}"))?;
    Ok(())
}

fn export_to_temp(state: &Rc<RefCell<EditorState>>) -> anyhow::Result<PathBuf> {
    let temp = crate::capture::hyprscreen_temp_dir()?
        .join(crate::capture::generated_filename("png"));
    export_to(state, &temp)?;
    Ok(temp)
}

fn position(window: &gtk::Window) {
    let window = window.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
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
