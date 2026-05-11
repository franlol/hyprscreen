# 0004 - Validate Stop Shortcut

## Status

Accepted

## Context

If the HUD is hidden, the user still needs a guaranteed way to stop recording.

## Decision

Do not allow hidden-HUD recording unless Hyprscreen verifies the official stop shortcut path.

## Consequences

Flexible manual bind detection is less important than predictable safety in v0.1.
