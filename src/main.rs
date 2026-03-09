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
    let command_input = args.command.join(" ");
    
    // Resolve aliases to ensure 'ff' and 'firefox' are treated as the same command.
    // We'll use the resolved path/name for state tracking, but keep original for launch.
    let command_key = hyprland::resolve_command(&args.command[0]);
    // Re-attach arguments if any
    let command_key = if args.command.len() > 1 {
        format!("{} {}", command_key, args.command[1..].join(" "))
    } else {
        command_key
    };

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

    // Launch the command (using original input to ensure shell features/aliases work)
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

                for addr in all_new {
                    state.add_window(&command_key, addr);
                }
                state.save().context("Failed to save state")?;
                
                return Ok(());
            }
        }
    }

    // If we timed out, we just assume the app launched but didn't create a window 
    eprintln!("Command launched, but no new window detected within timeout.");
    
    Ok(())
}