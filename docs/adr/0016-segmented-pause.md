# 0016 - Pause/Resume via Segmented Recording and Lossless Concat

## Status

Accepted

## Context

The v2 recording HUD has pause/resume. The installed wf-recorder (0.6.0) has
no pause capability — no signal handling for it, nothing in `--help` or the
binary's strings. Waiting for upstream support would block the feature.

## Decision

- **Pause** SIGINTs the current wf-recorder segment (which finalizes its
  file), remembers it in a segment list, freezes the accumulated timer, and
  marks the runtime state file with pid 0.
- **Resume** spawns a new wf-recorder with identical parameters
  (`LaunchSpec`: target geometry/output, format, audio) writing to
  `<base>-segN.<ext>`.
- **Stop** finalizes: a single segment passes through; multiple segments are
  joined with `ffmpeg -f concat -safe 0 -c copy` (identical codec parameters
  ⇒ lossless, no re-encode; verified: 1.56s + 1.44s → 3.00s) and the segment
  files are removed.
- **Restart** SIGINTs the current segment, deletes all segments, and respawns
  segment 0 with a fresh timer.
- **CLI `hyprscreen stop` while paused**: there is no live pid, so the CLI
  writes a stop-request sentinel (`$XDG_RUNTIME_DIR/hyprscreen-stop-requested`)
  that the GUI poll consumes and finalizes from.
- A ~100–300ms gap per pause (process teardown/startup) is accepted.
- The HUD mic button toggles the shared audio flag, which takes effect **at
  the next segment boundary** — wf-recorder cannot re-route audio
  mid-segment. The tooltip says so.

## Consequences

- The recording state machine lives in the GUI poll: pending Pause/Restart
  actions are resolved when the segment process is reaped.
- If the GUI process dies mid-recording, orphan segment files may remain in
  the temp dir.
