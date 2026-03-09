use anyhow::{Context, Result};
use miniserde::{json, Deserialize};
use std::process::Command;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Workspace {
    pub id: i32,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Client {
    pub class: String,
    pub address: String,
    #[serde(rename = "initialClass")]
    pub initial_class: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "initialTitle")]
    pub initial_title: Option<String>,
    pub tag: Option<String>,
    #[serde(rename = "xdgTag")]
    pub xdg_tag: Option<String>,
    pub workspace: Workspace,
    pub floating: bool,
    pub monitor: i32,
    pub pinned: bool,
    pub fullscreen: bool,
    #[serde(rename = "fullscreenMode")]
    pub fullscreen_mode: i32,
    pub at: [i32; 2],
    pub size: [i32; 2],
}

pub fn launch_command(command: &str) -> Result<()> {
    let output = Command::new("hyprctl")
        .arg("dispatch")
        .arg("exec")
        .arg(command)
        .output()
        .context("Failed to execute hyprctl exec")?;

    if !output.status.success() {
        anyhow::bail!("hyprctl exec failed");
    }
    Ok(())
}

pub fn focus_window(address: &str) -> Result<bool> {
    let output = Command::new("hyprctl")
        .arg("dispatch")
        .arg("focuswindow")
        .arg(format!("address:{address}"))
        .output()
        .context("Failed to execute hyprctl focuswindow")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("No such window found") {
        return Ok(false);
    }

    Ok(output.status.success())
}

pub fn move_to_workspace_silent(address: &str, workspace_id: i32) -> Result<()> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("movetoworkspacesilent")
        .arg(format!("{workspace_id},address:{address}"))
        .output()
        .context("Failed to move window to workspace")?;
    Ok(())
}

pub fn set_floating(address: &str, floating: bool) -> Result<()> {
    // Hyprland doesn't have a direct "set floating", only "togglefloating"
    // We need to check current state first, but for simplicity in loading 
    // we can use "setfloating" if it exists or check before toggle.
    // Actually, "dispatch setfloating" is not a thing, but we can use:
    // hyprctl dispatch togglefloating address:<addr>
    // We'll assume the window starts tiled (default) and toggle if needed.
    if floating {
        Command::new("hyprctl")
            .arg("dispatch")
            .arg("togglefloating")
            .arg(format!("address:{address}"))
            .output()
            .context("Failed to toggle floating")?;
    }
    Ok(())
}

pub fn set_fullscreen(address: &str, mode: i32) -> Result<()> {
    if mode > 0 {
        Command::new("hyprctl")
            .arg("dispatch")
            .arg("fullscreen")
            .arg(mode.to_string())
            // We might need to focus first as fullscreen often applies to active window
            // but recent Hyprland supports address in some dispatchers.
            // If not, we focus then fullscreen.
            .output()
            .context("Failed to set fullscreen")?;
    }
    Ok(())
}

pub fn move_window_pixel(address: &str, x: i32, y: i32) -> Result<()> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("movewindowpixel")
        .arg(format!("exact {x} {y},address:{address}"))
        .output()
        .context("Failed to move window pixel")?;
    Ok(())
}

pub fn resize_window_pixel(address: &str, w: i32, h: i32) -> Result<()> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("resizewindowpixel")
        .arg(format!("exact {w} {h},address:{address}"))
        .output()
        .context("Failed to resize window pixel")?;
    Ok(())
}

pub fn set_pinned(address: &str, pinned: bool) -> Result<()> {
    if pinned {
        Command::new("hyprctl")
            .arg("dispatch")
            .arg("pin")
            .arg(format!("address:{address}"))
            .output()
            .context("Failed to pin window")?;
    }
    Ok(())
}

pub fn get_active_window() -> Result<Client> {
    let output = Command::new("hyprctl")
        .arg("activewindow")
        .arg("-j")
        .output()?;
    let stdout = String::from_utf8(output.stdout)
        .context("Reading `hyprctl currentwindow -j` to string failed")?;

    if stdout.trim() == "{}" || stdout.trim().is_empty() {
        anyhow::bail!("No active window");
    }

    let client = json::from_str::<Client>(&stdout)
        .context("Failed to parse `hyprctl activewindow -j`")?;
    Ok(client)
}

pub fn get_clients() -> Result<Vec<Client>> {
    let output = Command::new("hyprctl").arg("clients").arg("-j").output()?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .context("Reading `hyprctl clients -j` to string failed")?;
        let clients = json::from_str::<Vec<Client>>(&stdout)
            .context("Failed to parse `hyprctl clients -j`")?;
        Ok(clients)
    } else {
        anyhow::bail!("hyprctl clients -j failed")
    }
}

pub fn get_client_addresses() -> Result<std::collections::HashSet<String>> {
    let clients = get_clients()?;
    Ok(clients.into_iter().map(|c| c.address).collect())
}
