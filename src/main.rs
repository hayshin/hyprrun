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

    // 2. Load State
    let mut state = State::load().unwrap_or_else(|_| State::default());

    // 3. Logic: Switch or Launch
    if !args.new {
        let current_focus = hyprland::get_active_window().ok().map(|c| c.address);
        while let Some(target_address) = state.get_next_window(&command_input, current_focus.as_deref()) {
            info!("Focusing existing window: {}", target_address);
            if hyprland::focus_window(&target_address)? {
                return Ok(());
            } else {
                warn!("Window {} no longer exists, removing from state", target_address);
                state.remove_window(&command_input, &target_address);
                let _ = state.save();
                // If we removed the current_focus window from state, 
                // we'll still find the next one in the next iteration.
            }
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