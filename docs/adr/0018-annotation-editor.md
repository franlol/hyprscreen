# 0018 - Annotation Editor Window

## Status

Accepted (amends 0002)

## Context

The v2 design includes a markup editor opened from the thumbnail card:
arrow, box, text, step counter, highlight, and blur tools with a small ink
palette, over the captured screenshot.

## Decision

- New whitelisted window `Hyprscreen Annotate` (single-slot), opened from the
  thumbnail's pen button — **screenshots only**; recordings keep the button
  disabled.
- **Architecture**: a `gtk::DrawingArea` renders the base pixbuf plus a
  `Vec<Shape>` (`Arrow`, `Rect`, `Highlight`, `Text`, `Step{n}`, `Blur`) via
  cairo. Geometry lives in **image coordinates**; the view scales down to fit
  ≤960×600 and export replays the identical shape list at native resolution.
  Stroke width is 3.5 view-px expressed in image space, so annotations keep
  their on-screen proportions in the exported file.
- **Blur is pixelation**, not gaussian: the covered region is scaled ÷8 and
  back up with nearest-neighbour (cached per rect). Deterministic, cheap,
  no new dependencies.
- **Text** places a floating `gtk::Entry` at the click point; Enter commits
  the string as a cairo-rendered shape (with a soft shadow), Esc aborts.
- **Step** auto-numbers (count of existing steps + 1). **Select (V)** moves
  the topmost shape under the cursor. **Undo** pops the shape list (Ctrl+Z).
  Tool keys: V A B T N H L, per the mock's tooltips.
- **Copy** exports to a temp PNG and pipes it to `wl-copy`; **Done** exports
  over the current artifact (saved or pinned temp) and refreshes the
  thumbnail card, then closes. Export uses a cairo `ImageSurface` +
  `write_to_png` (cairo-rs `png` feature).
- Five inks: `#5EE6D0 #FF5D5D #FFD23F #7CA8FF #FFFFFF`;
  `annotate_default_color` seeds the selection.

## Consequences

- Annotating an auto-saved file edits the saved file in place — Done is the
  commit point; there is no separate "annotated copy".
- Redo, shape deletion, and per-shape resize are out of scope (undo covers
  the common path; AGENTS.md prefers the minimal slice).
