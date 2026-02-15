//! Niri compositor backend.
//!
//! This module provides integration with the Niri compositor via its IPC socket.
//! Uses the niri-ipc crate for protocol types and the EventStreamState tracker
//! for maintaining compositor state from the event stream.

use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader, Write as _},
    os::unix::net::UnixStream,
    thread,
};

use anyhow::{Context, Result, anyhow};
use futures_signals::signal::Mutable;
use itertools::Itertools;
use niri_ipc::{
    Action, Event, Reply, Request, WorkspaceReferenceArg,
    state::{EventStreamState, EventStreamStatePart},
};
use tracing::{debug, error, info};

use super::types::{ActiveWindow, CompositorCommand, CompositorState, Monitor, Workspace};

/// Check if Niri is available (running).
pub fn is_available() -> bool {
    env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .is_some()
}

/// Execute a compositor command synchronously via niri IPC.
pub fn execute_command(cmd: CompositorCommand) -> Result<()> {
    let action = match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            let id = u64::try_from(id)
                .map_err(|_| anyhow!("Workspace ID {} is out of range for Niri backend", id))?;
            Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Id(id),
            }
        }
        CompositorCommand::FocusSpecialWorkspace(_) => {
            anyhow::bail!("Special workspaces not supported in Niri backend");
        }
        CompositorCommand::ToggleSpecialWorkspace(_) => {
            anyhow::bail!("Special workspaces not supported in Niri backend");
        }
        CompositorCommand::FocusMonitor(_) => {
            anyhow::bail!("FocusMonitor by ID not supported in Niri backend");
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            if dir > 0 {
                Action::FocusWorkspaceUp {}
            } else {
                Action::FocusWorkspaceDown {}
            }
        }
        CompositorCommand::NextKeyboardLayout => Action::SwitchLayout {
            layout: niri_ipc::LayoutSwitchTarget::Next,
        },
        CompositorCommand::Custom(action, args) => {
            if action == "spawn" {
                Action::Spawn {
                    command: vec![args],
                }
            } else {
                anyhow::bail!("Unknown custom dispatch: {}", action);
            }
        }
    };

    send_action(action)
}

/// Fetch the full compositor state from Niri.
///
/// Connects to the event stream, reads the initial burst of events
/// to populate the state tracker, then disconnects.
pub fn fetch_full_state() -> Result<CompositorState> {
    let mut stream = connect()?;

    // Request event stream
    let request_json = serde_json::to_string(&Request::EventStream)? + "\n";
    stream.write_all(request_json.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);

    // Read the handshake reply
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let reply: Reply = serde_json::from_str(&line).context("Failed to parse handshake")?;
    if let Err(e) = reply {
        anyhow::bail!("Niri refused EventStream: {}", e);
    }

    // Shutdown write half — we only read from here
    let stream_ref = reader.get_ref();
    stream_ref.shutdown(std::net::Shutdown::Write).ok();

    let mut internal_state = EventStreamState::default();

    // Read the initial burst of events (niri sends current state immediately).
    // Use a short timeout to detect when the burst is done.
    stream_ref
        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .ok();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(event) = serde_json::from_str::<Event>(&line) {
                    internal_state.apply(event);
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(map_state(&internal_state))
}

/// Start the Niri event listener in a dedicated thread.
///
/// Spawns a thread that connects to the niri event stream and continuously
/// updates the shared `Mutable<CompositorState>` as events arrive.
pub fn start_listener(data: Mutable<CompositorState>) {
    thread::spawn(move || {
        if let Err(e) = run_listener(data) {
            error!("Niri event listener error: {}", e);
        }
    });
}

/// Run the blocking event listener loop.
fn run_listener(data: Mutable<CompositorState>) -> Result<()> {
    info!("Starting Niri event listener");

    let mut stream = connect()?;

    // Request event stream
    let request_json = serde_json::to_string(&Request::EventStream)? + "\n";
    stream.write_all(request_json.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);

    // Read the handshake reply
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let reply: Reply = serde_json::from_str(&line).context("Failed to parse handshake")?;
    if let Err(e) = reply {
        anyhow::bail!("Niri refused EventStream: {}", e);
    }

    // Shutdown write half
    reader.get_ref().shutdown(std::net::Shutdown::Write).ok();

    let mut internal_state = EventStreamState::default();

    // Read events forever
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break; // EOF — niri disconnected
        }

        let event: Event = match serde_json::from_str(&line) {
            Ok(ev) => ev,
            Err(e) => {
                // Niri IPC is not version-bound — new fields/variants may appear.
                // Log and skip unknown events gracefully.
                debug!("Failed to parse Niri event (IPC version mismatch): {:?}", e);
                continue;
            }
        };

        internal_state.apply(event);

        // Map to our generic state and update the Mutable
        let state = map_state(&internal_state);
        data.set(state);
    }

    info!("Niri event stream ended");
    Ok(())
}

/// Connect to the niri IPC socket.
fn connect() -> Result<UnixStream> {
    let socket_path = env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .ok_or_else(|| anyhow!("NIRI_SOCKET or NIRI_SOCKET_PATH environment variable not set"))?;

    UnixStream::connect(socket_path).context("Failed to connect to Niri socket")
}

/// Send a single action to niri and read the reply.
fn send_action(action: Action) -> Result<()> {
    let mut stream = connect()?;

    let mut json = serde_json::to_string(&Request::Action(action))?;
    json.push('\n');
    stream.write_all(json.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    let reply: Reply = serde_json::from_str(&response_line)?;
    reply.map_err(|e| anyhow!("Niri error: {}", e)).map(|_| ())
}

/// Map niri's internal EventStreamState to our generic CompositorState.
fn map_state(niri: &EventStreamState) -> CompositorState {
    // Build a map of output name -> active workspace id
    let output_to_active_ws: HashMap<_, _> = niri
        .workspaces
        .workspaces
        .values()
        .filter_map(|ws| {
            if let Some(out) = &ws.output
                && ws.is_active
            {
                Some((out.clone(), ws.id as i32))
            } else {
                None
            }
        })
        .collect();

    // Sort outputs for stable monitor IDs (matches niri's internal ordering)
    let outputs: Vec<_> = output_to_active_ws.keys().sorted_unstable().collect();

    let mut workspaces: Vec<Workspace> = niri
        .workspaces
        .workspaces
        .values()
        .sorted_by_key(|w| w.idx)
        .map(|w| Workspace {
            id: w.id as i32,
            index: w.idx as i32,
            name: w.name.clone().unwrap_or_else(|| w.idx.to_string()),
            monitor: w.output.clone().unwrap_or_default(),
            monitor_id: w.output.as_ref().map(|wo| {
                outputs
                    .iter()
                    .position(|o| *o == wo)
                    .map_or(-1, |i| i as i32) as i128
            }),
            windows: 0,
            is_special: false,
        })
        .collect();

    // Calculate window counts per workspace
    for win in niri.windows.windows.values() {
        if let Some(ws_id) = win.workspace_id
            && let Some(ws) = niri.workspaces.workspaces.get(&ws_id)
            && let Some(generic_ws) = workspaces.iter_mut().find(|w| w.id == ws.id as i32)
        {
            generic_ws.windows += 1;
        }
    }

    // Build monitors
    let monitors: Vec<Monitor> = output_to_active_ws
        .iter()
        .map(|(name, active_ws_id)| Monitor {
            id: outputs
                .iter()
                .position(|o| *o == name)
                .map_or(-1, |i| i as i128),
            name: name.clone(),
            active_workspace_id: *active_ws_id,
            special_workspace_id: -1,
            ..Default::default()
        })
        .collect();

    let active_workspace_id = niri
        .workspaces
        .workspaces
        .values()
        .find(|w| w.is_focused)
        .map(|w| w.id as i32);

    let active_window = niri
        .windows
        .windows
        .values()
        .find(|w| w.is_focused)
        .map(|w| ActiveWindow {
            title: w.title.clone().unwrap_or_default(),
            class: w.app_id.clone().unwrap_or_default(),
            address: w.id.to_string(),
        });

    let keyboard_layout = niri.keyboard_layouts.keyboard_layouts.as_ref().map_or_else(
        || "Unknown".to_string(),
        |k| {
            k.names
                .get(k.current_idx as usize)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string())
        },
    );

    CompositorState {
        workspaces,
        monitors,
        active_workspace_id,
        active_window,
        keyboard_layout,
        submap: None,
    }
}
