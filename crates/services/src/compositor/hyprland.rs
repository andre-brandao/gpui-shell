//! Hyprland compositor backend.
//!
//! This module provides integration with the Hyprland compositor via its IPC socket.
//! Uses incremental updates with direct Mutable mutation for efficiency.

use crate::lifecycle::RunToken;
use anyhow::{Context, Result};
use futures_signals::signal::Mutable;
use hyprland::{
    data::Devices,
    dispatch::{
        Dispatch, DispatchType, MonitorIdentifier, WindowIdentifier, WorkspaceIdentifierWithSpecial,
    },
    error::HyprError,
    event_listener::EventListener,
    prelude::*,
};
use itertools::Itertools;
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher},
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::{Duration, Instant},
};
use tracing::{debug, info};

use super::types::{ActiveWindow, CompositorCommand, CompositorState, Monitor, Workspace};

/// Check if Hyprland is available (running).
pub fn is_available() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// A workspace, or a reference to one, as Hyprland reports it.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct HyprWorkspace {
    /// Hyprland 0.57 sends no id for special and named workspaces.
    id: Option<i32>,
    name: String,
    monitor: String,
    #[serde(rename = "monitorID")]
    monitor_id: Option<i128>,
    windows: u16,
}

impl HyprWorkspace {
    /// The id, else a stable negative id from the name, else 0 for no workspace.
    fn id(&self) -> i32 {
        match self.id {
            Some(id) => id,
            None if self.name.is_empty() => 0,
            None => {
                -1 - (BuildHasherDefault::<DefaultHasher>::default().hash_one(&self.name) >> 33)
                    as i32
            }
        }
    }

    fn is_special(&self) -> bool {
        // No id means special or named, hidden like before.
        self.id.is_none_or(|id| id < 0)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HyprMonitor {
    id: i128,
    name: String,
    active_workspace: HyprWorkspace,
    special_workspace: HyprWorkspace,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    scale: f32,
}

/// Defaults let `activewindow` parse its `{}` reply when no window has focus.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct HyprClient {
    address: String,
    at: (i32, i32),
    size: (i32, i32),
    workspace: HyprWorkspace,
    monitor: Option<i128>,
    class: String,
    title: String,
    pid: i32,
}

/// Run a `hyprctl -j` query. The crate's types need workspace ids that Hyprland 0.57 omits.
fn query<T: DeserializeOwned>(command: &str) -> Result<T> {
    let path =
        PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR not set")?)
            .join("hypr")
            .join(
                std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")
                    .context("HYPRLAND_INSTANCE_SIGNATURE not set")?,
            )
            .join(".socket.sock");
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(format!("j/{command}").as_bytes())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    Ok(serde_json::from_str(&reply)?)
}

/// Map a Hyprland client into our generic `Window`, resolving its
/// monitor id to a name via the already-fetched monitor list.
fn map_client(c: HyprClient, monitors: &[super::types::Monitor]) -> super::types::Window {
    let monitor = c
        .monitor
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .map(|m| m.name.clone())
        .unwrap_or_default();

    super::types::Window {
        id: c.address,
        app_id: c.class,
        title: c.title,
        monitor,
        workspace_id: c.workspace.id(),
        is_focused: false,
        is_minimized: false,
        is_urgent: false,
        pid: u32::try_from(c.pid).ok(),
        geometry: Some(super::types::WindowGeometry {
            x: c.at.0,
            y: c.at.1,
            width: c.size.0 as u32,
            height: c.size.1 as u32,
        }),
    }
}

/// Refetch the full client list and replace `state.windows` with it.
/// Called from the window opened/closed/moved event handlers, which don't
/// carry enough information in their event payload to patch the list
/// incrementally.
fn refresh_windows(state: &mut super::types::CompositorState) {
    if let Ok(clients) = query::<Vec<HyprClient>>("clients") {
        let monitors = state.monitors.clone();
        let focused_address = state.active_window.as_ref().map(|w| w.address.clone());
        // Clients carry no urgency, so keep what the `urgent` events set.
        let urgent: Vec<String> = state
            .windows
            .iter()
            .filter(|w| w.is_urgent)
            .map(|w| w.id.clone())
            .collect();
        state.windows = clients
            .into_iter()
            .map(|c| {
                let mut w = map_client(c, &monitors);
                w.is_focused = Some(&w.id) == focused_address.as_ref();
                w.is_urgent = !w.is_focused && urgent.contains(&w.id);
                w
            })
            .collect();
        sync_urgent_workspaces(state);
    }
}

fn sync_urgent_workspaces(state: &mut super::types::CompositorState) {
    for ws in state.workspaces.iter_mut() {
        ws.has_urgent = state
            .windows
            .iter()
            .any(|w| w.is_urgent && w.workspace_id == ws.id);
    }
}

/// Execute a compositor command synchronously.
pub fn execute_command(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
                id,
            )))?;
        }
        CompositorCommand::FocusSpecialWorkspace(name) => {
            Dispatch::call(DispatchType::Workspace(
                WorkspaceIdentifierWithSpecial::Special(Some(name.as_str())),
            ))?;
        }
        CompositorCommand::ToggleSpecialWorkspace(name) => {
            Dispatch::call(DispatchType::ToggleSpecialWorkspace(Some(name)))?;
        }
        CompositorCommand::FocusMonitor(id) => {
            Dispatch::call(DispatchType::FocusMonitor(MonitorIdentifier::Id(id)))?;
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            let d = if dir > 0 { "+1" } else { "-1" };
            Dispatch::call(DispatchType::Workspace(
                WorkspaceIdentifierWithSpecial::Relative(d.to_string().parse()?),
            ))?;
        }
        CompositorCommand::NextKeyboardLayout => {
            hyprland::ctl::switch_xkb_layout::call(
                "all",
                hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next,
            )?;
        }
        CompositorCommand::FocusWindow(address) => {
            Dispatch::call(DispatchType::FocusWindow(WindowIdentifier::Address(
                hyprland::shared::Address::new(address),
            )))?;
        }
        CompositorCommand::CloseWindow(address) => {
            Dispatch::call(DispatchType::CloseWindow(WindowIdentifier::Address(
                hyprland::shared::Address::new(address),
            )))?;
        }
        CompositorCommand::Custom(dispatcher, args) => {
            Dispatch::call(DispatchType::Custom(&dispatcher, &args))?;
        }
    }
    Ok(())
}

/// Fetch the complete compositor state from Hyprland.
/// Used for initial state and occasional full refresh.
pub fn fetch_full_state() -> Result<CompositorState> {
    let workspaces = query::<Vec<HyprWorkspace>>("workspaces")?
        .into_iter()
        .sorted_by_key(|w| w.id())
        .map(|w| Workspace {
            id: w.id(),
            index: w.id(),
            is_special: w.is_special(),
            name: w.name,
            monitor: w.monitor,
            monitor_id: w.monitor_id,
            windows: w.windows,
            has_urgent: false,
        })
        .collect();

    let monitors: Vec<Monitor> = query::<Vec<HyprMonitor>>("monitors all")?
        .into_iter()
        .map(|m| Monitor {
            id: m.id,
            name: m.name,
            active_workspace_id: m.active_workspace.id(),
            special_workspace_id: m.special_workspace.id(),
            width: m.width,
            height: m.height,
            x: m.x,
            y: m.y,
            scale: m.scale,
        })
        .collect();

    let mut windows: Vec<super::types::Window> = query::<Vec<HyprClient>>("clients")?
        .into_iter()
        .map(|c| map_client(c, &monitors))
        .collect();

    let active_workspace_id = query::<HyprWorkspace>("activeworkspace")
        .ok()
        .filter(|w| !w.name.is_empty())
        .map(|w| w.id());

    let active_window = query::<HyprClient>("activewindow")
        .ok()
        .filter(|w| !w.address.is_empty())
        .map(|w| ActiveWindow {
            title: w.title,
            class: w.class,
            address: w.address,
        });
    for w in windows.iter_mut() {
        w.is_focused = active_window.as_ref().is_some_and(|a| a.address == w.id);
    }

    let keyboard_layout = Devices::get()
        .ok()
        .and_then(|d| {
            d.keyboards
                .into_iter()
                .find(|k| k.main)
                .map(|k| k.active_keymap)
        })
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(CompositorState {
        workspaces,
        monitors,
        active_workspace_id,
        active_window,
        windows,
        keyboard_layout,
        submap: None,
    })
}

/// Follow Hyprland's event socket on a dedicated thread. The socket closing
/// fails the attempt, and the retry refetches whatever it missed.
pub fn start_listener(data: Mutable<CompositorState>, token: RunToken) {
    let mut resync = false;
    token.supervise_blocking("Hyprland", move |token| {
        loop {
            if std::mem::replace(&mut resync, true) {
                resync_state(&data)?;
            }
            token.active();
            let started = Instant::now();
            match run_listener(data.clone()) {
                // The crate fails on 0.57 workspace events that send a name in place of an id.
                Err(err)
                    if token.alive()
                        && matches!(
                            err.downcast_ref::<HyprError>(),
                            Some(HyprError::Internal(_))
                        ) =>
                {
                    debug!("Resyncing after an unparsed Hyprland event: {err:#}");
                    if started.elapsed() < RESYNC_GAP && !token.sleep_blocking(RESYNC_GAP) {
                        return Ok(());
                    }
                }
                Err(err) => return Err(err),
                Ok(()) => anyhow::bail!("Hyprland event socket closed"),
            }
        }
    });
}

/// Least time between two resyncs after unparsed events.
const RESYNC_GAP: Duration = Duration::from_millis(500);

/// Refetch the full state, keeping what Hyprland does not send again.
fn resync_state(data: &Mutable<CompositorState>) -> Result<()> {
    let mut state = fetch_full_state()?;
    {
        let old = data.lock_ref();
        state.submap = old.submap.clone();
        for w in state.windows.iter_mut() {
            w.is_urgent = old.windows.iter().any(|o| o.is_urgent && o.id == w.id);
        }
    }
    sync_urgent_workspaces(&mut state);
    data.set_neq(state);
    Ok(())
}

/// Run the sync event listener with incremental updates.
fn run_listener(data: Mutable<CompositorState>) -> Result<()> {
    info!("Starting Hyprland event listener (incremental updates)");

    let mut listener = EventListener::new();

    // Workspace changed (active workspace switched)
    {
        let data = data.clone();
        listener.add_workspace_changed_handler(move |evt| {
            debug!("Workspace changed: {:?}", evt.name);
            let mut state = data.lock_mut();
            state.active_workspace_id = Some(evt.id);
        });
    }

    // Workspace added
    {
        let data = data.clone();
        listener.add_workspace_added_handler(move |evt| {
            debug!("Workspace added: {:?}", evt.name);
            let mut state = data.lock_mut();
            // Check if workspace already exists
            if !state.workspaces.iter().any(|w| w.id == evt.id) {
                state.workspaces.push(Workspace {
                    id: evt.id,
                    index: evt.id,
                    name: evt.name.to_string(),
                    monitor: String::new(), // Will be updated by monitor events
                    monitor_id: None,
                    windows: 0,
                    is_special: evt.id < 0,
                    has_urgent: false,
                });
                state.workspaces.sort_by_key(|w| w.id);
            }
        });
    }

    // Workspace deleted
    {
        let data = data.clone();
        listener.add_workspace_deleted_handler(move |evt| {
            debug!("Workspace deleted: {:?}", evt.name);
            let mut state = data.lock_mut();
            state.workspaces.retain(|w| w.id != evt.id);
        });
    }

    // Workspace moved to different monitor
    {
        let data = data.clone();
        listener.add_workspace_moved_handler(move |evt| {
            debug!("Workspace {} moved to monitor {}", evt.id, evt.monitor);
            let mut state = data.lock_mut();
            if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == evt.id) {
                ws.monitor = evt.monitor.clone();
            }
        });
    }

    // Active window changed
    {
        let data = data.clone();
        listener.add_active_window_changed_handler(move |evt| {
            debug!("Active window changed: {:?}", evt);
            let mut state = data.lock_mut();
            let focused_address = evt.as_ref().map(|w| w.address.to_string());
            for w in state.windows.iter_mut() {
                w.is_focused = Some(&w.id) == focused_address.as_ref();
                // Hyprland clears urgency once the window is focused.
                w.is_urgent &= !w.is_focused;
            }
            sync_urgent_workspaces(&mut state);
            state.active_window = evt.map(|w| ActiveWindow {
                title: w.title,
                class: w.class,
                address: w.address.to_string(),
            });
        });
    }

    // Window asked for attention
    {
        let data = data.clone();
        listener.add_urgent_state_changed_handler(move |address| {
            debug!("Urgent window: {}", address);
            let address = address.to_string();
            let mut state = data.lock_mut();
            if let Some(w) = state
                .windows
                .iter_mut()
                .find(|w| w.id == address && !w.is_focused)
            {
                w.is_urgent = true;
            }
            sync_urgent_workspaces(&mut state);
        });
    }

    // Window opened - increment window count
    {
        let data = data.clone();
        listener.add_window_opened_handler(move |evt| {
            debug!(
                "Window opened: {} on workspace {:?}",
                evt.window_class, evt.workspace_name
            );
            let mut state = data.lock_mut();
            // Try to find workspace by name and increment window count
            if let Some(ws) = state
                .workspaces
                .iter_mut()
                .find(|w| w.name == evt.workspace_name)
            {
                ws.windows = ws.windows.saturating_add(1);
            }
            refresh_windows(&mut state);
        });
    }

    // Window closed - decrement window count
    {
        let data = data.clone();
        listener.add_window_closed_handler(move |_evt| {
            debug!("Window closed");
            // We don't know which workspace, so do a lightweight refresh of window counts
            // This is a compromise - we could track window addresses but that adds complexity
            if let Ok(workspaces) = query::<Vec<HyprWorkspace>>("workspaces") {
                let mut state = data.lock_mut();
                for ws_data in workspaces {
                    if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == ws_data.id()) {
                        ws.windows = ws_data.windows;
                    }
                }
                refresh_windows(&mut state);
            }
        });
    }

    // Window moved between workspaces
    {
        let data = data.clone();
        listener.add_window_moved_handler(move |evt| {
            debug!("Window moved to workspace {}", evt.workspace_name);
            // Refresh window counts for affected workspaces
            if let Ok(workspaces) = query::<Vec<HyprWorkspace>>("workspaces") {
                let mut state = data.lock_mut();
                for ws_data in workspaces {
                    if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == ws_data.id()) {
                        ws.windows = ws_data.windows;
                    }
                }
                refresh_windows(&mut state);
            }
        });
    }

    // Monitor added/changed
    {
        let data = data.clone();
        listener.add_active_monitor_changed_handler(move |evt| {
            debug!(
                "Active monitor changed: {} workspace {:?}",
                evt.monitor_name, evt.workspace_name
            );
            // Find workspace id first (before mutable borrow)
            let ws_id = evt.workspace_name.as_ref().and_then(|ws_type| {
                let ws_name = ws_type.to_string();
                let state = data.lock_ref();
                state
                    .workspaces
                    .iter()
                    .find(|w| w.name == ws_name)
                    .map(|w| w.id)
            });
            // Now update the monitor
            if let Some(ws_id) = ws_id {
                let mut state = data.lock_mut();
                if let Some(mon) = state
                    .monitors
                    .iter_mut()
                    .find(|m| m.name == evt.monitor_name)
                {
                    mon.active_workspace_id = ws_id;
                }
            }
        });
    }

    // Submap changed
    {
        let data = data.clone();
        listener.add_sub_map_changed_handler(move |submap| {
            debug!("Submap changed: {}", submap);
            let mut state = data.lock_mut();
            state.submap = if submap.is_empty() {
                None
            } else {
                Some(submap)
            };
        });
    }

    // Keyboard layout changed
    {
        let data = data.clone();
        listener.add_layout_changed_handler(move |evt| {
            debug!("Keyboard layout changed: {}", evt.layout_name);
            let mut state = data.lock_mut();
            state.keyboard_layout = evt.layout_name;
        });
    }

    // Special workspace events
    {
        let data = data.clone();
        listener.add_changed_special_handler(move |evt| {
            debug!("Special workspace changed: {:?}", evt.workspace_name);
            // Refresh special workspace state
            if let Ok(monitors) = query::<Vec<HyprMonitor>>("monitors all") {
                let mut state = data.lock_mut();
                for mon_data in monitors {
                    if let Some(mon) = state.monitors.iter_mut().find(|m| m.name == mon_data.name) {
                        mon.special_workspace_id = mon_data.special_workspace.id();
                    }
                }
            }
        });
    }

    {
        let data = data.clone();
        listener.add_special_removed_handler(move |_evt| {
            debug!("Special workspace removed");
            // Refresh special workspace state
            if let Ok(monitors) = query::<Vec<HyprMonitor>>("monitors all") {
                let mut state = data.lock_mut();
                for mon_data in monitors {
                    if let Some(mon) = state.monitors.iter_mut().find(|m| m.name == mon_data.name) {
                        mon.special_workspace_id = mon_data.special_workspace.id();
                    }
                }
            }
        });
    }

    // Start the blocking event listener
    listener.start_listener()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> HyprWorkspace {
        HyprWorkspace {
            name: name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn workspaces_parse_with_and_without_an_id() {
        let new: Vec<HyprWorkspace> = serde_json::from_str(
            r#"[
                {"address": "1", "id": 1, "type": "normal", "name": "1", "monitor": "DP-1", "monitorID": 0, "windows": 2},
                {"address": "special:special", "type": "special", "name": "special:special", "monitor": "DP-1", "monitorID": 0, "windows": 1},
                {"address": "notes", "type": "normal", "name": "notes", "monitor": "DP-1", "monitorID": 0, "windows": 0}
            ]"#,
        )
        .unwrap();
        assert_eq!((new[0].id(), new[0].is_special()), (1, false));
        assert!(new[1].id() < 0 && new[1].is_special());
        assert_eq!((new[1].monitor.as_str(), new[1].windows), ("DP-1", 1));
        assert!(new[2].id() < 0 && new[2].is_special());

        let old: Vec<HyprWorkspace> = serde_json::from_str(
            r#"[
                {"id": 1, "name": "1", "monitor": "DP-1", "monitorID": 0, "windows": 2},
                {"id": -99, "name": "special:special", "monitor": "DP-1", "monitorID": 0, "windows": 1}
            ]"#,
        )
        .unwrap();
        assert_eq!((old[0].id(), old[0].is_special()), (1, false));
        assert_eq!((old[1].id(), old[1].is_special()), (-99, true));

        assert_ne!(named("special:a").id(), named("special:b").id());
    }

    #[test]
    fn monitors_parse_with_and_without_a_special_workspace_id() {
        let special_id = |special: &str| {
            let monitors: Vec<HyprMonitor> = serde_json::from_str(&format!(
                r#"[{{"id": 0, "name": "DP-1", "activeWorkspace": {{"id": 1, "name": "1"}},
                    "specialWorkspace": {special}, "width": 1920, "height": 1080, "x": 0, "y": 0, "scale": 1.0}}]"#
            ))
            .unwrap();
            assert_eq!(monitors[0].active_workspace.id(), 1);
            monitors[0].special_workspace.id()
        };

        assert_eq!(special_id(r#"{"address": "", "type": "", "name": ""}"#), 0);
        assert_eq!(
            special_id(
                r#"{"address": "special:special", "type": "special", "name": "special:special"}"#
            ),
            named("special:special").id()
        );
        assert_eq!(special_id(r#"{"id": 0, "name": ""}"#), 0);
        assert_eq!(special_id(r#"{"id": -99, "name": "special:special"}"#), -99);
    }

    #[test]
    fn clients_parse_with_and_without_a_workspace_id() {
        let workspace_id = |workspace: &str| {
            let clients: Vec<HyprClient> = serde_json::from_str(&format!(
                r#"[{{"address": "0x1", "at": [10, 20], "size": [300, 200], "workspace": {workspace},
                    "monitor": 0, "class": "app", "title": "title", "pid": 1}}]"#
            ))
            .unwrap();
            map_client(clients.into_iter().next().unwrap(), &[]).workspace_id
        };

        assert_eq!(
            workspace_id(
                r#"{"address": "special:special", "type": "special", "name": "special:special"}"#
            ),
            named("special:special").id()
        );
        assert_eq!(
            workspace_id(r#"{"address": "2", "id": 2, "type": "normal", "name": "2"}"#),
            2
        );
        assert_eq!(
            workspace_id(r#"{"id": -99, "name": "special:special"}"#),
            -99
        );

        let none: HyprClient = serde_json::from_str("{}").unwrap();
        assert!(none.address.is_empty());
    }
}
