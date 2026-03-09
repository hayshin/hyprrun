use anyhow::{Context, Result};
use miniserde::{json, Deserialize};
use std::process::{Child, Command};

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

pub fn launch_command(command: &str) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("exec")
        .arg(command)
        .spawn()
}

pub fn focus_window(address: &str) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("focuswindow")
        .arg(format!("address:{address}"))
        .spawn()
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
        // Fallback or error? The original code had a match block handling failure by just launching.
        // Here we should probably return error and let caller decide.
        // But looking at original main:
        // match json { Ok ... => { logic }, _ => { launch_command } }
        // So if get_clients fails, we treat it as "no clients found" or similar and proceed to launch.
        // I will return Result.
        anyhow::bail!("hyprctl clients -j failed")
    }
}

pub fn get_client_addresses() -> Result<std::collections::HashSet<String>> {
    let clients = get_clients()?;
    Ok(clients.into_iter().map(|c| c.address).collect())
}
