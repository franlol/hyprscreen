# 0013 - Corner Thumbnail Replaces the Preview Page; Auto-Save Semantics

## Status

Accepted (amends 0001 and 0002; supersedes 0006 and 0007)

## Context

The v1 flow kept every capture inside the main window: a preview page with
Back / New / Save / Copy·Open / Reveal / GIF. The v2 design replaces this with
a 536px corner thumbnail card at the top-right of the focused monitor and
default auto-saving, so the dock never changes shape and the result is
non-blocking.

## Decision

- **New window type**: `Hyprscreen Thumbnail` joins the extra-window whitelist
  (amending ADR-0002). Single-slot: a new capture replaces the current card.
- **Auto-save (default `autosave = true`)**: the artifact is copied to the
  save directory before the card appears; the temp is deleted immediately.
  The meta line shows green `auto-saved`; the card self-dismisses after
  `thumbnail_timeout_seconds` (default 8; `0` = never).
- **Pinned (`autosave = false`)**: the artifact stays a temp file, meta shows
  `pinned`, the card never self-dismisses, and Save copies it out (meta flips
  to `saved`).
- **Card actions**: Annotate (wired later) · Copy (accent; raw PNG for
  screenshots, `text/uri-list` for videos) · Save · Share (screenshots,
  `text/uri-list` — pastes as an attachable file) / GIF (recordings, replaces
  Share; conversion stays post-capture per ADR-0010 and the converted GIF
  becomes the card's artifact) · Discard (deletes the file, saved or not).
  Recordings get a center play affordance that opens the video.
- **Close (×) and replacement** delete a still-pinned temp (the old preview
  `Back` semantics); saved files are never deleted implicitly.
- **Back and New are gone** (superseding ADR-0006/0007): the dock is always
  on screen — repeat is pressing Enter again, and temp lifecycle is handled
  by auto-save/discard/close as above.
- `Preview` is no longer a main-window state; ADR-0001's single-window rule
  now covers just the dock.

## Consequences

- Feedback for card actions arrives via toasts (ADR-0014), not a status line.
- `Reveal` moved from a dedicated button into the Save toast's action.
- The recording thumbnail PNG is a temp owned by the card and cleaned up on
  close.
