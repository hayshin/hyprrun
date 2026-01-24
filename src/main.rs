mod args;
mod hyprland;
mod state;

use anyhow::{Result, Context};
use args::Args;
use state::State;
use std::thread;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    // 1. Parse Args
    let args: Args = argh::from_env();
    if args.command.is_empty() {
        anyhow::bail!("No command provided");
    }
    let command_key = args.command.join(" ");

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
        if let Some(target_address) = state.get_next_window(&command_key, active_window.as_deref()) {
            // ... focus the next one.
            hyprland::focus_window(&target_address)?;
            return Ok(());
        }
    }

    // 4. Launch and Detect
    // Snapshot active windows BEFORE launch
    let clients_before = hyprland::get_client_addresses().unwrap_or_default();

    // Launch the command
    // We pass the command string directly to hyprctl exec
    // Note: If the user passed arguments like `kitty -e htop`, 
    // `command_key` is "kitty -e htop".
    // hyprctl exec expects the command string.
    hyprland::launch_command(&command_key)?;

    // Poll for new window
    let start = Instant::now();
    let timeout = Duration::from_secs(5); // 5 second timeout
    let poll_interval = Duration::from_millis(200);

    println!("Waiting for window to appear...");
    while start.elapsed() < timeout {
        thread::sleep(poll_interval);

        if let Ok(clients_after) = hyprland::get_client_addresses() {
            // Find address in 'after' that was not in 'before'
            let new_windows: Vec<&String> = clients_after.difference(&clients_before).collect();

            if !new_windows.is_empty() {
                // Heuristic: Pick the first new window found.
                // In a race condition (user launches 2 apps), this might be wrong,
                // but acceptable for this scope.
                let new_address = new_windows[0].clone();
                
                // Track it
                state.add_window(&command_key, new_address.clone());
                state.save().context("Failed to save state")?;
                
                // Optional: Focus it (usually auto-focused by Hyprland, but ensure it)
                // hyprland::focus_window(&new_address)?; 
                // Commented out: Hyprland usually focuses new windows. 
                // If we force focus, we might steal focus if the user quickly alt-tabbed away.
                
                return Ok(());
            }
        }
    }

    // If we timed out, we just assume the app launched but didn't create a window 
    // (active in background?) or took too long. We don't track it.
    eprintln!("Command launched, but no new window detected within timeout.");
    
    Ok(())
}