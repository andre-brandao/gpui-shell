//! Hyprland compositor backend.
//!
//! This module provides integration with the Hyprland compositor via its IPC socket.
//! Uses incremental updates with direct Mutable mutation for efficiency.

use anyhow::Result;
use futures_signals::signal::Mutable;
use hyprland::{
    data::{Client, Devices, Monitors, Workspace as HWorkspace, Workspaces},
    dispatch::{Dispatch, DispatchType, MonitorIdentifier, WorkspaceIdentifierWithSpecial},
    event_listener::EventListener,
    prelude::*,
};
use itertools::Itertools;
use std::thread;
use tracing::{debug, error, info};

use super::types::{ActiveWindow, CompositorCommand, CompositorState, Monitor, Workspace};

/// Check if Hyprland is available (running).
pub fn is_available() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
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
        CompositorCommand::Custom(dispatcher, args) => {
            Dispatch::call(DispatchType::Custom(&dispatcher, &args))?;
        }
    }
    Ok(())
}

/// Fetch the complete compositor state from Hyprland.
/// Used for initial state and occasional full refresh.
pub fn fetch_full_state() -> Result<CompositorState> {
    let workspaces = Workspaces::get()?
        .into_iter()
        .sorted_by_key(|w| w.id)
        .map(|w| Workspace {
            id: w.id,
            index: w.id,
            name: w.name,
            monitor: w.monitor,
            monitor_id: w.monitor_id,
            windows: w.windows,
            is_special: w.id < 0,
        })
        .collect();

    let monitors = Monitors::get()?
        .into_iter()
        .map(|m| Monitor {
            id: m.id,
            name: m.name,
            active_workspace_id: m.active_workspace.id,
            special_workspace_id: m.special_workspace.id,
            width: m.width as u32,
            height: m.height as u32,
            x: m.x as i32,
            y: m.y as i32,
            scale: m.scale,
        })
        .collect();

    let active_workspace_id = HWorkspace::get_active().ok().map(|w| w.id);

    let active_window = Client::get_active().ok().flatten().map(|w| ActiveWindow {
        title: w.title,
        class: w.class,
        address: w.address.to_string(),
    });

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
        keyboard_layout,
        submap: None,
    })
}

/// Start the Hyprland event listener in a dedicated thread.
/// Uses sync EventListener with direct Mutable mutation for efficiency.
pub fn start_listener(data: Mutable<CompositorState>) {
    thread::spawn(move || {
        if let Err(e) = run_listener(data) {
            error!("Hyprland event listener error: {}", e);
        }
    });
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
            state.active_window = evt.map(|w| ActiveWindow {
                title: w.title,
                class: w.class,
                address: w.address.to_string(),
            });
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
        });
    }

    // Window closed - decrement window count
    {
        let data = data.clone();
        listener.add_window_closed_handler(move |_evt| {
            debug!("Window closed");
            // We don't know which workspace, so do a lightweight refresh of window counts
            // This is a compromise - we could track window addresses but that adds complexity
            if let Ok(workspaces) = Workspaces::get() {
                let mut state = data.lock_mut();
                for ws_data in workspaces {
                    if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == ws_data.id) {
                        ws.windows = ws_data.windows;
                    }
                }
            }
        });
    }

    // Window moved between workspaces
    {
        let data = data.clone();
        listener.add_window_moved_handler(move |evt| {
            debug!("Window moved to workspace {}", evt.workspace_name);
            // Refresh window counts for affected workspaces
            if let Ok(workspaces) = Workspaces::get() {
                let mut state = data.lock_mut();
                for ws_data in workspaces {
                    if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == ws_data.id) {
                        ws.windows = ws_data.windows;
                    }
                }
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
            if let Ok(monitors) = Monitors::get() {
                let mut state = data.lock_mut();
                for mon_data in monitors {
                    if let Some(mon) = state.monitors.iter_mut().find(|m| m.name == mon_data.name) {
                        mon.special_workspace_id = mon_data.special_workspace.id;
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
            if let Ok(monitors) = Monitors::get() {
                let mut state = data.lock_mut();
                for mon_data in monitors {
                    if let Some(mon) = state.monitors.iter_mut().find(|m| m.name == mon_data.name) {
                        mon.special_workspace_id = mon_data.special_workspace.id;
                    }
                }
            }
        });
    }

    // Start the blocking event listener
    listener.start_listener()?;

    Ok(())
}
