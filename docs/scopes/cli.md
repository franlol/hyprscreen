# CLI Scope

## Current Commands

- `hyprscreen`
- `hyprscreen stop`
- `hyprscreen screenshot area|window|monitor`
- `hyprscreen record area|window|monitor`
- `hyprscreen --version` / `-V`
- `hyprscreen --help` / `-h`

## Current Behavior

- `hyprscreen` opens the GUI in setup state.
- `hyprscreen screenshot <target>` opens the GUI and immediately runs the capture flow for the target, exactly as if the user pressed `Capture`.
- `hyprscreen record <target>` opens the GUI and immediately starts the recording flow for the target, with HUD or indicator according to config.
- `hyprscreen stop` stops the active recording without opening the GUI.
- `hyprscreen --version` / `-V` prints the version string.
- `hyprscreen --help` / `-h` prints the usage block.
- Invalid subcommands or invalid targets print help and exit with code 2.

## Planned Commands

- Headless capture flags (`--output`, `--clipboard-only`, `--no-clipboard`, `--preview`)
- `hyprscreen status` — print whether a recording is active (PID, duration, output path)
- `hyprscreen toggle <target>` — start a recording if idle, stop it otherwise
- `hyprscreen list monitors`
- `hyprscreen open <path>`
