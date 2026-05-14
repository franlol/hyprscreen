use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

#[derive(Deserialize)]
struct MonitorInfo {
    id: i32,
    name: String,
    disabled: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: f64,
    #[serde(rename = "activeWorkspace")]
    active_workspace: WorkspaceInfo,
}

#[derive(Clone)]
struct MonitorTarget {
    name: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Deserialize)]
struct WorkspaceInfo {
    id: i32,
}

#[derive(Deserialize)]
struct ClientInfo {
    mapped: bool,
    hidden: bool,
    class: String,
    title: String,
    monitor: i32,
    workspace: WorkspaceInfo,
    at: [i32; 2],
    size: [i32; 2],
}

fn temp_file_path() -> Result<PathBuf> {
    Ok(super::hyprscreen_temp_dir()?.join(crate::capture::generated_filename("png")))
}

pub fn select_area() -> Result<String> {
    let guard = super::CompositorRepaintGuard::arm();
    let output = Command::new("slurp")
        .args([
            "-b", "#00000088",
            "-c", "#e8eefcff",
            "-s", "#00000000",
            "-w", "3",
            "-d",
        ])
        .output()
        .context("failed to launch slurp")?;

    if !output.status.success() {
        bail!("area selection was cancelled")
    }

    guard.wait();

    let geometry = String::from_utf8(output.stdout)
        .context("slurp returned non-utf8 geometry")?
        .trim()
        .to_owned();

    if geometry.is_empty() {
        bail!("area selection returned no geometry")
    }

    Ok(geometry)
}

pub fn select_window() -> Result<String> {
    let monitor_output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .context("failed to query Hyprland monitors")?;

    if !monitor_output.status.success() {
        bail!("hyprctl monitors failed")
    }

    let monitors: Vec<MonitorInfo> = serde_json::from_slice(&monitor_output.stdout)
        .context("failed to parse Hyprland monitors JSON")?;
    let active_workspaces_by_monitor = monitors
        .into_iter()
        .filter(|m| !m.disabled)
        .map(|m| (m.id, m.active_workspace.id))
        .collect::<HashMap<_, _>>();

    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .context("failed to query Hyprland clients")?;

    if !output.status.success() {
        bail!("hyprctl clients failed")
    }

    let clients: Vec<ClientInfo> =
        serde_json::from_slice(&output.stdout).context("failed to parse Hyprland clients JSON")?;

    let choices = clients
        .into_iter()
        .filter(|c| c.mapped && !c.hidden && c.class != super::SELF_APP_CLASS)
        .filter(|c| c.size[0] > 0 && c.size[1] > 0)
        .filter(|c| {
            active_workspaces_by_monitor
                .get(&c.monitor)
                .is_some_and(|ws_id| c.workspace.id == *ws_id)
        })
        .map(|c| {
            let title = if c.title.is_empty() {
                c.class.clone()
            } else {
                format!("{} - {}", c.class, c.title.replace('\n', " "))
            };
            format!("{},{} {}x{} {}", c.at[0], c.at[1], c.size[0], c.size[1], title)
        })
        .collect::<Vec<_>>();

    if choices.is_empty() {
        bail!("no eligible windows found")
    }

    let guard = super::CompositorRepaintGuard::arm();
    let mut child = Command::new("slurp")
        .args([
            "-r",
            "-b", "#00000088",
            "-c", "#e8eefcff",
            "-s", "#00000000",
            "-w", "3",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to launch slurp for window selection")?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to open slurp stdin"))?;
    stdin.write_all(choices.join("\n").as_bytes())?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .context("failed while waiting for slurp window selection")?;

    if !output.status.success() {
        bail!("window selection was cancelled")
    }

    guard.wait();

    let geometry = String::from_utf8(output.stdout)
        .context("slurp returned non-utf8 geometry")?
        .trim()
        .to_owned();

    if geometry.is_empty() {
        bail!("window selection returned no geometry")
    }

    Ok(geometry)
}

pub fn select_monitor() -> Result<String> {
    let monitors = available_monitors()?;
    if monitors.is_empty() {
        bail!("no eligible monitors found")
    }
    let guard = super::CompositorRepaintGuard::arm();
    let geometry = select_monitor_geometry(&monitors)?;
    guard.wait();
    let (x, y, width, height) = super::parse_geometry(&geometry)?;
    monitors
        .into_iter()
        .find(|m| m.x == x && m.y == y && m.width == width && m.height == height)
        .map(|m| m.name)
        .ok_or_else(|| anyhow!("selected monitor could not be resolved"))
}

pub fn capture_geometry(geometry: &str) -> Result<PathBuf> {
    // slurp's 3px border is centered on the selection boundary, so ~1.5 logical
    // pixels of it fall inside the reported geometry. Insetting by 2px removes
    // the two artifact rows/columns that would otherwise appear in the capture.
    capture_geometry_inset(geometry, 2)
}

pub fn capture_window_geometry(geometry: &str) -> Result<PathBuf> {
    // Window captures need a larger inset: Hyprland draws its border + rounded
    // corners over the region reported by `hyprctl clients`. The inset must cover
    // border_size + rounding so that neither the straight-edge border nor the
    // corner arc bleeds into the captured image.
    capture_geometry_inset(geometry, super::hyprland_window_inset())
}

fn capture_geometry_inset(geometry: &str, inset: i32) -> Result<PathBuf> {
    let path = temp_file_path()?;
    let inset_geom = super::inset_geometry(geometry, inset).unwrap_or_else(|| geometry.to_owned());
    let status = Command::new("grim")
        .arg("-g")
        .arg(&inset_geom)
        .arg(&path)
        .status()
        .context("failed to launch grim")?;

    if !status.success() {
        return Err(anyhow!("grim failed to capture the selected geometry"));
    }

    Ok(path)
}

pub fn capture_by_monitor_name(name: &str) -> Result<PathBuf> {
    let path = temp_file_path()?;
    let status = Command::new("grim")
        .arg("-o")
        .arg(name)
        .arg(&path)
        .status()
        .context("failed to launch grim")?;
    if !status.success() {
        return Err(anyhow!("grim failed to capture the monitor"));
    }
    Ok(path)
}

fn available_monitors() -> Result<Vec<MonitorTarget>> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .context("failed to query Hyprland monitors")?;

    if !output.status.success() {
        bail!("hyprctl monitors failed")
    }

    let monitors: Vec<MonitorInfo> =
        serde_json::from_slice(&output.stdout).context("failed to parse Hyprland monitors JSON")?;

    Ok(monitors
        .into_iter()
        .filter(|m| !m.disabled)
        .map(|m| MonitorTarget {
            name: m.name,
            x: m.x,
            y: m.y,
            width: ((m.width as f64) / m.scale).round() as i32,
            height: ((m.height as f64) / m.scale).round() as i32,
        })
        .collect())
}

fn select_monitor_geometry(monitors: &[MonitorTarget]) -> Result<String> {
    let choices = monitors
        .iter()
        .map(|m| format!("{},{} {}x{}", m.x, m.y, m.width, m.height))
        .collect::<Vec<_>>();

    let mut child = Command::new("slurp")
        .args([
            "-r",
            "-b", "#00000088",
            "-c", "#e8eefcff",
            "-s", "#00000000",
            "-w", "6",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to launch slurp for monitor selection")?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to open slurp stdin"))?;
    stdin.write_all(choices.join("\n").as_bytes())?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .context("failed while waiting for slurp monitor selection")?;

    if !output.status.success() {
        bail!("monitor selection was cancelled")
    }

    let geometry = String::from_utf8(output.stdout)
        .context("slurp returned non-utf8 geometry")?
        .trim()
        .to_owned();

    if geometry.is_empty() {
        bail!("monitor selection returned no geometry")
    }

    Ok(geometry)
}

