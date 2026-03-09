use anyhow::{Context, Result};
use miniserde::{json, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct State {
    pub windows: HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowProperties {
    pub workspace_id: i32,
    pub floating: bool,
    pub monitor: i32,
    pub pinned: bool,
    pub fullscreen_mode: i32,
    pub at: [i32; 2],
    pub size: [i32; 2],
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Session {
    /// Mapping of command to properties of windows it owned.
    pub entries: Vec<(String, WindowProperties)>,
}

impl State {
    fn get_path() -> Result<PathBuf> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR not set")?;
        Ok(PathBuf::from(runtime_dir).join("hyprrun.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;
        if !path.exists() {
            return Ok(<State as Default>::default());
        }

        let content = fs::read_to_string(&path).context("Failed to read state file")?;

        // Handle empty file case
        if content.trim().is_empty() {
            return Ok(<State as Default>::default());
        }

        json::from_str(&content).context("Failed to parse state file")
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;
        let tmp_path = path.with_extension("json.tmp");
        let content = json::to_string(self);
        fs::write(&tmp_path, content).context("Failed to write temporary state file")?;
        fs::rename(tmp_path, path).context("Failed to finalize state file save")
    }

    /// Removes an address from the tracked windows for a given command.
    pub fn remove_window(&mut self, command: &str, address: &str) {
        if let Some(addresses) = self.windows.get_mut(command) {
            addresses.retain(|a| a != address);
            if addresses.is_empty() {
                self.windows.remove(command);
            }
        }
    }

    /// Removes an address from ALL commands.
    pub fn remove_window_by_address(&mut self, address: &str) {
        let mut commands_to_remove = Vec::new();
        for (cmd, addrs) in self.windows.iter_mut() {
            addrs.retain(|a| a != address);
            if addrs.is_empty() {
                commands_to_remove.push(cmd.clone());
            }
        }
        for cmd in commands_to_remove {
            self.windows.remove(&cmd);
        }
    }

    pub fn add_window(&mut self, command: &str, address: String) {
        let windows = self
            .windows
            .entry(command.to_string())
            .or_insert_with(Vec::new);

        if !windows.contains(&address) {
            windows.push(address);
        }
    }

    pub fn get_next_window(&self, command: &str, current_focus: Option<&str>) -> Option<String> {
        let addresses = self.windows.get(command)?;
        if addresses.is_empty() {
            return None;
        }

        if let Some(focus) = current_focus {
            if let Some(pos) = addresses.iter().position(|a| a == focus) {
                // Return next window, cycling to start
                return Some(addresses[(pos + 1) % addresses.len()].clone());
            }
        }

        // Default to first window
        Some(addresses[0].clone())
    }
}

impl Session {
    fn get_path() -> Result<PathBuf> {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME").map(|h| PathBuf::from(h).join(".config"))
            })
            .context("Could not determine config directory")?;

        let dir = config_dir.join("hyprrun");
        if !dir.exists() {
            fs::create_dir_all(&dir).context("Failed to create config directory")?;
        }
        Ok(dir.join("session.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;
        if !path.exists() {
            return Ok(<Session as Default>::default());
        }
        let content = fs::read_to_string(&path).context("Failed to read session file")?;
        if content.trim().is_empty() {
            return Ok(<Session as Default>::default());
        }
        json::from_str(&content).context("Failed to parse session file")
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;
        let content = json::to_string(self);
        fs::write(path, content).context("Failed to write session file")
    }
}
