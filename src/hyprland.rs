use anyhow::{Context, Result};
use miniserde::{json, Deserialize};
use std::process::Command;

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
