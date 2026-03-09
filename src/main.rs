mod args;
mod hyprland;
mod state;
mod logger;

use anyhow::{Result, Context};
use args::Args;
use state::{State, Session, WindowProperties};
use std::thread;
use std::time::{Duration, Instant};
use log::{info, warn};

fn main() -> Result<()> {
    // 0. Init Logger
    let _ = logger::init();

    // 1. Parse Args
    let args: Args = argh::from_env();

    if args.save {
        info!("Saving session...");
        let state = State::load().unwrap_or_default();
        let clients = hyprland::get_clients()?;
        let client_map: std::collections::HashMap<String, &hyprland::Client> = clients
            .iter()
            .map(|c| (c.address.clone(), c))
            .collect();

        let mut session = Session::default();
        for (command, addresses) in state.windows {
            for addr in addresses {
                if let Some(client) = client_map.get(&addr) {
                    session.entries.push((
                        command.clone(),
                        WindowProperties {
                            workspace_id: client.workspace.id,
                            floating: client.floating,
                            monitor: client.monitor,
                            pinned: client.pinned,
                            fullscreen_mode: client.fullscreen_mode,
                            at: client.at,
                            size: client.size,
                        },
                    ));
                }
            }
        }
        session.save()?;
        info!("Session saved with {} entries", session.entries.len());
        return Ok(());
    }

    if args.load {
        info!("Loading session...");
        let session = Session::load()?;
        let mut state = State::load().unwrap_or_default();

        for (command, props) in session.entries {
            info!("Launching for session: {}", command);
            let clients_before = hyprland::get_client_addresses().unwrap_or_default();
            hyprland::launch_command(&command)?;

            // Poll for new window
            let start = Instant::now();
            let timeout = Duration::from_secs(5);
            let poll_interval = Duration::from_millis(100);
            let mut detected_addr = None;

            while start.elapsed() < timeout {
                thread::sleep(poll_interval);
                if let Ok(clients_after) = hyprland::get_client_addresses() {
                    let new_windows: Vec<String> = clients_after
                        .difference(&clients_before)
                        .cloned()
                        .collect();

                    if !new_windows.is_empty() {
                        detected_addr = Some(new_windows[0].clone());
                        break;
                    }
                }
            }

            if let Some(addr) = detected_addr {
                info!("Applying properties to window: {}", addr);
                // Apply properties
                let _ = hyprland::move_to_workspace_silent(&addr, props.workspace_id);
                let _ = hyprland::set_floating(&addr, props.floating);
                if props.floating {
                    let _ = hyprland::move_window_pixel(&addr, props.at[0], props.at[1]);
                    let _ = hyprland::resize_window_pixel(&addr, props.size[0], props.size[1]);
                }
                let _ = hyprland::set_pinned(&addr, props.pinned);
                let _ = hyprland::set_fullscreen(&addr, props.fullscreen_mode);

                state.add_window(&command, addr);
                let _ = state.save();
            } else {
                warn!("Timed out waiting for window for command: {}", command);
            }
        }
        return Ok(());
    }

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