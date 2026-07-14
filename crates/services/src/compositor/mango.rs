//! Mango compositor backend.
//!
//! This module provides integration with the Mango compositor via its `mmsg` IPC
//! socket (see `mango(1)`/`mmsg(1)`). The socket path is provided by the
//! `MANGO_INSTANCE_SIGNATURE` environment variable, and the protocol is
//! newline-delimited JSON: one-shot `get`/`dispatch` commands close the
//! connection after a single reply, while `watch` commands keep the connection
//! open and stream JSON updates.
//!
//! Mango models workspaces as per-monitor "tags" rather than globally unique
//! workspace IDs, so this backend synthesizes workspace IDs as
//! `monitor_index * 100 + tag_index`.

use std::{
    env,
    io::{BufRead, BufReader, Write as _},
    os::unix::net::UnixStream,
    thread,
};

use anyhow::{Context, Result, anyhow};
use futures_signals::signal::Mutable;
use serde_json::Value;
use tracing::{debug, error, info};

use super::types::{ActiveWindow, CompositorCommand, CompositorState, Monitor, Workspace};

/// Check if Mango is available (running).
pub fn is_available() -> bool {
    env::var_os("MANGO_INSTANCE_SIGNATURE").is_some()
}

/// Connect to the Mango IPC socket.
fn connect() -> Result<UnixStream> {
    let socket_path = env::var_os("MANGO_INSTANCE_SIGNATURE")
        .ok_or_else(|| anyhow!("MANGO_INSTANCE_SIGNATURE environment variable not set"))?;
    UnixStream::connect(socket_path).context("Failed to connect to Mango socket")
}

/// Send a one-shot command and parse the single-line JSON reply.
fn send_command(cmd: &str) -> Result<Value> {
    let mut stream = connect()?;
    stream.write_all(cmd.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    serde_json::from_str(&response).context("Failed to parse Mango reply")
}

/// Resolve a monitor's name from its synthesized index (position in `get all-monitors`).
fn monitor_name_at(index: usize) -> Result<String> {
    let reply = send_command("get all-monitors")?;
    reply["monitors"]
        .as_array()
        .and_then(|arr| arr.get(index))
        .and_then(|m| m["name"].as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Monitor index {} out of range for Mango backend", index))
}

/// Decode a synthesized workspace ID into `(monitor_index, tag_index)`.
fn decode_workspace_id(id: i32) -> (i32, i32) {
    (id / 100, id % 100)
}

/// Execute a compositor command synchronously via mmsg IPC.
pub fn execute_command(cmd: CompositorCommand) -> Result<()> {
    let dispatch = match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            let (monitor_index, tag) = decode_workspace_id(id);
            let monitor_name = monitor_name_at(monitor_index as usize)?;
            format!("dispatch viewcrossmon,{},{}", tag, monitor_name)
        }
        CompositorCommand::FocusMonitor(id) => {
            let monitor_name = monitor_name_at(id as usize)?;
            format!("dispatch focusmon,{}", monitor_name)
        }
        CompositorCommand::FocusSpecialWorkspace(_) => {
            anyhow::bail!("Focusing a named scratchpad directly is not supported in Mango backend");
        }
        CompositorCommand::ToggleSpecialWorkspace(name) => {
            format!("dispatch toggle_named_scratchpad,{}", name)
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            if dir > 0 {
                "dispatch viewtoright".to_string()
            } else {
                "dispatch viewtoleft".to_string()
            }
        }
        CompositorCommand::NextKeyboardLayout => {
            // With no index argument, mango's switch_keyboard_layout falls back
            // to `(current + 1) % num_layouts`, i.e. cycling to the next layout.
            "dispatch switch_keyboard_layout".to_string()
        }
        CompositorCommand::Custom(action, args) => {
            if args.is_empty() {
                format!("dispatch {}", action)
            } else {
                format!("dispatch {},{}", action, args)
            }
        }
    };

    let reply = send_command(&dispatch)?;
    if let Some(err) = reply.get("error").and_then(Value::as_str) {
        anyhow::bail!("Mango error: {}", err);
    }
    Ok(())
}

/// Extract the focused-client reply into our generic `ActiveWindow`.
///
/// Handles both the one-shot `{"error": "no focused client"}` reply and the
/// watch stream's `{"id": null, "title": null, "appid": null}` reply.
fn map_active_window(v: &Value) -> Option<ActiveWindow> {
    let id = v.get("id")?.as_i64()?;
    Some(ActiveWindow {
        title: v["title"].as_str().unwrap_or_default().to_string(),
        class: v["appid"].as_str().unwrap_or_default().to_string(),
        address: id.to_string(),
    })
}

/// Build workspaces/monitors/active workspace ID from a `get`/`watch all-monitors` reply.
fn build_monitors(monitors: &[Value]) -> (Vec<Workspace>, Vec<Monitor>, Option<i32>) {
    let mut workspaces = Vec::new();
    let mut out_monitors = Vec::new();
    let mut active_workspace_id = None;

    for (idx, m) in monitors.iter().enumerate() {
        let name = m["name"].as_str().unwrap_or_default().to_string();
        let is_active = m["active"].as_bool().unwrap_or(false);
        let active_tag = m["active_tags"]
            .as_array()
            .and_then(|tags| tags.first())
            .and_then(Value::as_i64)
            .map(|n| n as i32);

        if let Some(tags) = m["tags"].as_array() {
            for tag in tags {
                let tag_index = tag["index"].as_i64().unwrap_or(0) as i32;
                let ws_id = (idx as i32) * 100 + tag_index;
                workspaces.push(Workspace {
                    id: ws_id,
                    index: tag_index,
                    name: tag_index.to_string(),
                    monitor: name.clone(),
                    monitor_id: Some(idx as i128),
                    windows: tag["client_count"].as_i64().unwrap_or(0) as u16,
                    is_special: false,
                });
                if is_active && active_tag == Some(tag_index) {
                    active_workspace_id = Some(ws_id);
                }
            }
        }

        let active_workspace_for_monitor = active_tag.map_or(-1, |t| (idx as i32) * 100 + t);

        out_monitors.push(Monitor {
            id: idx as i128,
            name,
            active_workspace_id: active_workspace_for_monitor,
            special_workspace_id: -1,
            width: m["width"].as_i64().unwrap_or(0) as u32,
            height: m["height"].as_i64().unwrap_or(0) as u32,
            x: m["x"].as_i64().unwrap_or(0) as i32,
            y: m["y"].as_i64().unwrap_or(0) as i32,
            scale: m["scale"].as_f64().unwrap_or(1.0) as f32,
        });
    }

    (workspaces, out_monitors, active_workspace_id)
}

fn keymode_to_submap(keymode: &str) -> Option<String> {
    if keymode.is_empty() || keymode == "default" {
        None
    } else {
        Some(keymode.to_string())
    }
}

/// Fetch the full compositor state from Mango.
pub fn fetch_full_state() -> Result<CompositorState> {
    let monitors_reply = send_command("get all-monitors")?;
    let monitors = monitors_reply["monitors"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let (workspaces, monitors, active_workspace_id) = build_monitors(&monitors);

    let focusing_reply = send_command("get focusing-client")?;
    let active_window = map_active_window(&focusing_reply);

    let keymode_reply = send_command("get keymode")?;
    let submap = keymode_reply["keymode"]
        .as_str()
        .and_then(keymode_to_submap);

    let layout_reply = send_command("get keyboardlayout")?;
    let keyboard_layout = layout_reply["layout"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    Ok(CompositorState {
        workspaces,
        monitors,
        active_workspace_id,
        active_window,
        keyboard_layout,
        submap,
    })
}

/// Start the Mango event listeners in dedicated threads.
///
/// Mango's `watch` protocol only streams one kind of update per connection,
/// so we open one persistent connection per state category we care about.
pub fn start_listener(data: Mutable<CompositorState>) {
    spawn_watch_thread(data.clone(), "watch all-monitors", |data, json| {
        if let Some(monitors) = json["monitors"].as_array() {
            let (workspaces, monitors, active_workspace_id) = build_monitors(monitors);
            let mut state = data.lock_mut();
            state.workspaces = workspaces;
            state.monitors = monitors;
            state.active_workspace_id = active_workspace_id;
        }
    });

    spawn_watch_thread(data.clone(), "watch focusing-client", |data, json| {
        let mut state = data.lock_mut();
        state.active_window = map_active_window(&json);
    });

    spawn_watch_thread(data.clone(), "watch keymode", |data, json| {
        if let Some(keymode) = json["keymode"].as_str() {
            let mut state = data.lock_mut();
            state.submap = keymode_to_submap(keymode);
        }
    });

    spawn_watch_thread(data.clone(), "watch keyboardlayout", |data, json| {
        if let Some(layout) = json["layout"].as_str() {
            let mut state = data.lock_mut();
            state.keyboard_layout = layout.to_string();
        }
    });
}

fn spawn_watch_thread(
    data: Mutable<CompositorState>,
    cmd: &'static str,
    handler: impl Fn(&Mutable<CompositorState>, Value) + Send + 'static,
) {
    thread::spawn(move || {
        if let Err(e) = run_watch(cmd, &data, &handler) {
            error!("Mango '{}' listener error: {}", cmd, e);
        }
    });
}

/// Run a single blocking watch connection, invoking `handler` for each JSON update.
fn run_watch(
    cmd: &str,
    data: &Mutable<CompositorState>,
    handler: &impl Fn(&Mutable<CompositorState>, Value),
) -> Result<()> {
    info!("Starting Mango listener: {}", cmd);

    let mut stream = connect()?;
    stream.write_all(cmd.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break; // EOF — mango disconnected
        }

        match serde_json::from_str::<Value>(&line) {
            Ok(json) => handler(data, json),
            Err(e) => debug!("Failed to parse Mango event for '{}': {:?}", cmd, e),
        }
    }

    info!("Mango watch stream ended: {}", cmd);
    Ok(())
}
