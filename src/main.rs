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
use std::io::{BufRead, BufReader};

fn main() -> Result<()> {
    // 0. Init Logger
    let _ = logger::init();

    // 1. Parse Args
    let args: Args = argh::from_env();

    if args.listen {
        info!("Starting event listener...");
        return run_listener();
    }

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
            
            let stream = hyprland::connect_event_socket().ok();
            if let Some(ref s) = stream {
                let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
            }
            
            hyprland::launch_command(&command)?;

            let detected_addr = if let Some(s) = stream {
                wait_for_new_window(s, Duration::from_secs(5))?.into_iter().next()
            } else {
                warn!("Could not connect to event socket, falling back to polling");
                poll_for_new_window(Duration::from_secs(5))?
            };

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
    let stream = hyprland::connect_event_socket().ok();
    if let Some(ref s) = stream {
        let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    }

    info!("Launching command: {}", command_input);
    hyprland::launch_command(&command_input)?;

    let new_windows = if let Some(s) = stream {
        wait_for_new_window(s, Duration::from_secs(5))?
    } else {
        warn!("Could not connect to event socket, falling back to polling");
        poll_for_new_window(Duration::from_secs(5))?.into_iter().collect()
    };

    if !new_windows.is_empty() {
        info!("Detected {} new window(s)", new_windows.len());
        for addr in new_windows {
            info!("Linking window: {}", addr);
            state.add_window(&command_input, addr);
        }
        state.save().context("Failed to save state")?;
        return Ok(());
    }

    // If we timed out, we just assume the app launched but didn't create a window 
    warn!("Command launched, but no new window detected within timeout.");
    
    Ok(())
}

fn run_listener() -> Result<()> {
    loop {
        let stream = match hyprland::connect_event_socket() {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to connect to event socket: {}. Retrying in 5s...", e);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        while let Ok(n) = reader.read_line(&mut line) {
            if n == 0 { break; } // Connection closed

            if line.starts_with("closewindow>>") {
                let addr = line["closewindow>>".len()..].trim();
                if !addr.is_empty() {
                    let addr = if addr.starts_with("0x") { addr.to_string() } else { format!("0x{}", addr) };
                    info!("Window closed: {}", addr);
                    if let Ok(mut state) = State::load() {
                        state.remove_window_by_address(&addr);
                        let _ = state.save();
                    }
                }
            }
            line.clear();
        }
        warn!("Event socket connection lost. Reconnecting in 5s...");
        thread::sleep(Duration::from_secs(5));
    }
}

fn wait_for_new_window(stream: std::os::unix::net::UnixStream, timeout: Duration) -> Result<Vec<String>> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let start = Instant::now();
    let mut detected = Vec::new();

    while start.elapsed() < timeout {
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.starts_with("openwindow>>") {
                    let data = &line["openwindow>>".len()..];
                    if let Some(addr) = data.split(',').next() {
                        let addr = addr.trim();
                        let addr = if addr.starts_with("0x") { addr.to_string() } else { format!("0x{}", addr) };
                        detected.push(addr);
                        
                        // Wait a bit more for potential sibling windows
                        thread::sleep(Duration::from_millis(400));
                        
                        // Try to read more if they are already in the buffer
                        let _ = reader.get_mut().set_nonblocking(true);
                        while {
                            line.clear();
                            reader.read_line(&mut line).is_ok()
                        } {
                            if line.starts_with("openwindow>>") {
                                let data = &line["openwindow>>".len()..];
                                if let Some(addr) = data.split(',').next() {
                                    let addr = addr.trim();
                                    let addr = if addr.starts_with("0x") { addr.to_string() } else { format!("0x{}", addr) };
                                    detected.push(addr);
                                }
                            }
                        }
                        return Ok(detected);
                    }
                }
                line.clear();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(detected)
}

fn poll_for_new_window(timeout: Duration) -> Result<Option<String>> {
    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);
    let clients_before = hyprland::get_client_addresses().unwrap_or_default();

    while start.elapsed() < timeout {
        thread::sleep(poll_interval);
        if let Ok(clients_after) = hyprland::get_client_addresses() {
            let new_windows: Vec<String> = clients_after
                .difference(&clients_before)
                .cloned()
                .collect();

            if !new_windows.is_empty() {
                return Ok(Some(new_windows[0].clone()));
            }
        }
    }
    Ok(None)
}