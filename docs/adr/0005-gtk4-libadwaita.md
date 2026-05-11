# 0005 - GTK4 UI Stack

## Status

Accepted

## Context

Hyprscreen needs a modern Linux-native UI for a small but polished desktop app.

## Decision

Use GTK4 for the main app UI and avoid libadwaita-specific styling constraints.

## Consequences

System GTK4 development packages are required to build the app.
