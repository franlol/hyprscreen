use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
    let dir = std::env::temp_dir().join("hyprscreen");
    fs::create_dir_all(&dir).context("failed to create Hyprscreen temp directory")?;
    Ok(dir.join(crate::capture::generated_filename("png")))
}

pub fn capture_area() -> Result<PathBuf> {
    let geometry = select_area_geometry()?;
    capture_geometry(&geometry)
}

pub fn select_monitor() -> Result<String> {
    let monitors = available_monitors()?;
    if monitors.is_empty() {
        bail!("no eligible monitors found")
    }
    let geometry = select_monitor_geometry(&monitors)?;
    let (x, y, width, height) = parse_geometry(&geometry)?;
    monitors
        .into_iter()
        .find(|m| m.x == x && m.y == y && m.width == width && m.height == height)
        .map(|m| m.name)
        .ok_or_else(|| anyhow!("selected monitor could not be resolved"))
}

pub fn capture_by_monitor_name(name: &str) -> Result<PathBuf> {
    // Let the compositor repaint without slurp's overlay before grim captures.
    thread::sleep(Duration::from_millis(80));

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
        .filter(|monitor| !monitor.disabled)
        .map(|monitor| MonitorTarget {
            name: monitor.name,
            x: monitor.x,
            y: monitor.y,
            width: ((monitor.width as f64) / monitor.scale).round() as i32,
            height: ((monitor.height as f64) / monitor.scale).round() as i32,
        })
        .collect())
}

fn select_monitor_geometry(monitors: &[MonitorTarget]) -> Result<String> {
    let choices = monitors
        .iter()
        .map(|monitor| {
            format!(
                "{},{} {}x{}",
                monitor.x, monitor.y, monitor.width, monitor.height
            )
        })
        .collect::<Vec<_>>();

    let mut child = Command::new("slurp")
        .args([
            "-r",
            "-b",
            "#00000088",
            "-c",
            "#e8eefcff",
            "-s",
            "#e8eefc1a",
            "-w",
            "6",
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

fn parse_geometry(geometry: &str) -> Result<(i32, i32, i32, i32)> {
    let (origin, size) = geometry
        .split_once(' ')
        .ok_or_else(|| anyhow!("geometry did not contain a size separator"))?;
    let (x, y) = origin
        .split_once(',')
        .ok_or_else(|| anyhow!("geometry origin was invalid"))?;
    let (width, height) = size
        .split_once('x')
        .ok_or_else(|| anyhow!("geometry size was invalid"))?;

    Ok((x.parse()?, y.parse()?, width.parse()?, height.parse()?))
}

pub fn capture_window() -> Result<PathBuf> {
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
        .filter(|monitor| !monitor.disabled)
        .map(|monitor| (monitor.id, monitor.active_workspace.id))
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
        .filter(|client| client.mapped && !client.hidden && client.class != "land.hypr.Hyprscreen")
        .filter(|client| client.size[0] > 0 && client.size[1] > 0)
        .filter(|client| {
            active_workspaces_by_monitor
                .get(&client.monitor)
                .is_some_and(|workspace_id| client.workspace.id == *workspace_id)
        })
        .map(|client| {
            let title = if client.title.is_empty() {
                client.class.clone()
            } else {
                format!("{} - {}", client.class, client.title.replace('\n', " "))
            };

            format!(
                "{},{} {}x{} {}",
                client.at[0], client.at[1], client.size[0], client.size[1], title
            )
        })
        .collect::<Vec<_>>();

    if choices.is_empty() {
        bail!("no eligible windows found")
    }

    let mut child = Command::new("slurp")
        .args([
            "-r",
            "-b",
            "#00000088",
            "-c",
            "#e8eefcff",
            "-s",
            "#e8eefc1a",
            "-w",
            "3",
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

    let geometry = String::from_utf8(output.stdout)
        .context("slurp returned non-utf8 geometry")?
        .trim()
        .to_owned();

    if geometry.is_empty() {
        bail!("window selection returned no geometry")
    }

    capture_geometry(&geometry)
}

fn select_area_geometry() -> Result<String> {
    let output = Command::new("slurp")
        .args([
            "-b",
            "#00000088",
            "-c",
            "#e8eefcff",
            "-s",
            "#e8eefc1a",
            "-w",
            "3",
            "-d",
        ])
        .output()
        .context("failed to launch slurp")?;

    if !output.status.success() {
        bail!("area selection was cancelled")
    }

    let geometry = String::from_utf8(output.stdout)
        .context("slurp returned non-utf8 geometry")?
        .trim()
        .to_owned();

    if geometry.is_empty() {
        bail!("area selection returned no geometry")
    }

    Ok(geometry)
}

fn capture_geometry(geometry: &str) -> Result<PathBuf> {
    // Let the compositor repaint without slurp's overlay before grim captures,
    // otherwise the selection borders and dim layer bleed into the screenshot.
    thread::sleep(Duration::from_millis(80));

    let path = temp_file_path()?;
    let status = Command::new("grim")
        .arg("-g")
        .arg(geometry)
        .arg(&path)
        .status()
        .context("failed to launch grim")?;

    if !status.success() {
        return Err(anyhow!("grim failed to capture the selected geometry"));
    }

    Ok(path)
}
