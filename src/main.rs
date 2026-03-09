mod args;
mod hyprland;
mod state;
mod logger;

use anyhow::{Result, Context};
use args::Args;
use state::State;
use std::thread;
use std::time::{Duration, Instant};
use log::{info, warn};

fn main() -> Result<()> {
    // 0. Init Logger
    let _ = logger::init();

    // 1. Parse Args
    let args: Args = argh::from_env();
    if args.command.is_empty() {
        anyhow::bail!("No command provided");
    }
    let command_input = args.command.join(" ");

    info!("Command: '{}'", command_input);

    // 2. Load and Clean State
    let mut state = State::load().unwrap_or_else(|_| State::default());
    let active_addresses = hyprland::get_client_addresses().unwrap_or_default();
    
    // Clean dead windows from state
    state.clean(&active_addresses);
    // Best effort save cleaned state
    let _ = state.save();

    // 3. Logic: Switch or Launch
    // If not forced new, and we have tracked windows for this command...
    if !args.new {
        let active_window = hyprland::get_active_window().ok().map(|c| c.address);
        if let Some(target_address) = state.get_next_window(&command_input, active_window.as_deref()) {
            info!("Focusing existing window: {}", target_address);
            hyprland::focus_window(&target_address)?;
            return Ok(());
        }
    }

    // 4. Launch and Detect
    // Snapshot active windows BEFORE launch
    let clients_before = hyprland::get_client_addresses().unwrap_or_default();

    info!("Launching command: {}", command_input);
    hyprland::launch_command(&command_input)?;

    // Poll for new window
    let start = Instant::now();
    let timeout = Duration::from_secs(5); // 5 second timeout
    let poll_interval = Duration::from_millis(100);

    while start.elapsed() < timeout {
        thread::sleep(poll_interval);

        if let Ok(clients_after) = hyprland::get_client_addresses() {
            // Find address in 'after' that was not in 'before'
            let new_windows: Vec<String> = clients_after
                .difference(&clients_before)
                .cloned()
                .collect();

            if !new_windows.is_empty() {
                // Wait a bit longer to see if more windows pop up
                thread::sleep(Duration::from_millis(400));
                
                // Final snapshot
                let final_clients = hyprland::get_client_addresses().unwrap_or(clients_after);
                let all_new: Vec<String> = final_clients
                    .difference(&clients_before)
                    .cloned()
                    .collect();

                info!("Detected {} new window(s)", all_new.len());
                for addr in all_new {
                    info!("Linking window: {}", addr);
                    state.add_window(&command_input, addr);
                }
                state.save().context("Failed to save state")?;
                
                return Ok(());
            }
        }
    }

    // If we timed out, we just assume the app launched but didn't create a window 
    warn!("Command launched, but no new window detected within timeout.");
    
    Ok(())
}