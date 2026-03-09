use anyhow::{Context, Result};
use miniserde::{json, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct State {
    pub windows: HashMap<String, Vec<String>>,
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
        let content = json::to_string(self);
        fs::write(path, content).context("Failed to write state file")
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
