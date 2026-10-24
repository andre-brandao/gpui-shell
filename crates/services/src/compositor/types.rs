//! Type definitions for compositor state and events.

/// A workspace managed by the compositor.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Workspace {
    /// Unique workspace ID.
    pub id: i32,
    /// Display index (may differ from ID for special workspaces).
    pub index: i32,
    /// Workspace name (user-defined or auto-generated).
    pub name: String,
    /// Name of the monitor this workspace is on.
    pub monitor: String,
    /// ID of the monitor (if available).
    pub monitor_id: Option<i128>,
    /// Number of windows on this workspace.
    pub windows: u16,
    /// Whether this is a special/scratchpad workspace.
    pub is_special: bool,
    /// Whether a window on this workspace asked for attention.
    pub has_urgent: bool,
}

/// A monitor/output managed by the compositor.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Monitor {
    /// Unique monitor ID.
    pub id: i128,
    /// Monitor name (e.g., "eDP-1", "HDMI-A-1").
    pub name: String,
    /// ID of the currently active workspace on this monitor.
    pub active_workspace_id: i32,
    /// ID of the special workspace on this monitor (if any).
    pub special_workspace_id: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// X position in the layout.
    pub x: i32,
    /// Y position in the layout.
    pub y: i32,
    /// Scale factor.
    pub scale: f32,
}

/// Information about the currently focused window.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActiveWindow {
    /// Window title.
    pub title: String,
    /// Window class (application identifier).
    pub class: String,
    /// Window address/handle.
    pub address: String,
}

/// A single open window/toplevel tracked by the compositor.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Window {
    /// Stable window address/handle (backend-specific format).
    pub id: String,
    /// Application ID / window class.
    pub app_id: String,
    /// Window title.
    pub title: String,
    /// Name of the monitor this window is on.
    pub monitor: String,
    /// ID of the workspace this window is on.
    pub workspace_id: i32,
    /// Whether this window currently has focus.
    pub is_focused: bool,
    /// Whether this window is minimized (always `false` on backends with
    /// no minimize concept, e.g. Niri).
    pub is_minimized: bool,
    /// Whether this window asked for attention.
    pub is_urgent: bool,
    /// Process id of the window's client, when the backend reports it.
    pub pid: Option<u32>,
    /// Window position and size, when the backend can report it. `None` on
    /// backends with no fixed on-screen geometry (e.g. Niri's scrollable
    /// tiling layout has no absolute window position).
    pub geometry: Option<WindowGeometry>,
}

/// On-screen position and size of a window, in the compositor's global
/// layout space (same space as `Monitor.x`/`Monitor.y`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Complete compositor state snapshot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompositorState {
    /// All workspaces.
    pub workspaces: Vec<Workspace>,
    /// All monitors.
    pub monitors: Vec<Monitor>,
    /// ID of the currently active workspace.
    pub active_workspace_id: Option<i32>,
    /// Currently focused window (if any).
    pub active_window: Option<ActiveWindow>,
    /// All currently open windows.
    pub windows: Vec<Window>,
    /// Current keyboard layout name.
    pub keyboard_layout: String,
    /// Current submap/mode (if any).
    pub submap: Option<String>,
}

impl CompositorState {
    /// Get the active workspace.
    pub fn active_workspace(&self) -> Option<&Workspace> {
        self.active_workspace_id
            .and_then(|id| self.workspaces.iter().find(|w| w.id == id))
    }

    /// Get workspaces for a specific monitor.
    pub fn workspaces_for_monitor(&self, monitor_name: &str) -> Vec<&Workspace> {
        self.workspaces
            .iter()
            .filter(|w| w.monitor == monitor_name && !w.is_special)
            .collect()
    }

    /// Get all non-special workspaces.
    pub fn regular_workspaces(&self) -> Vec<&Workspace> {
        self.workspaces.iter().filter(|w| !w.is_special).collect()
    }

    /// Get all special workspaces.
    pub fn special_workspaces(&self) -> Vec<&Workspace> {
        self.workspaces.iter().filter(|w| w.is_special).collect()
    }

    /// Get a monitor by name.
    pub fn monitor_by_name(&self, name: &str) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.name == name)
    }

    /// Get a monitor by ID.
    pub fn monitor_by_id(&self, id: i128) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.id == id)
    }

    /// The window a process owns, else the first one whose app id is one of
    /// `app_ids` (compared ignoring case).
    pub fn app_window(&self, pid: Option<u32>, app_ids: &[String]) -> Option<&Window> {
        pid.and_then(|pid| self.windows.iter().find(|w| w.pid == Some(pid)))
            .or_else(|| {
                self.windows.iter().find(|w| {
                    app_ids
                        .iter()
                        .any(|app| !w.app_id.is_empty() && app.eq_ignore_ascii_case(&w.app_id))
                })
            })
    }
}

/// Supported compositor backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompositorBackend {
    #[default]
    Hyprland,
    Niri,
    Mango,
}

impl CompositorBackend {
    /// Get a human-readable name for this backend.
    pub fn name(&self) -> &'static str {
        match self {
            CompositorBackend::Hyprland => "Hyprland",
            CompositorBackend::Niri => "Niri",
            CompositorBackend::Mango => "Mango",
        }
    }
}

/// Commands that can be sent to the compositor.
#[derive(Debug, Clone)]
pub enum CompositorCommand {
    /// Focus a workspace by ID.
    FocusWorkspace(i32),
    /// Focus a special workspace by name.
    FocusSpecialWorkspace(String),
    /// Focus a monitor by ID.
    FocusMonitor(i128),
    /// Toggle a special workspace by name.
    ToggleSpecialWorkspace(String),
    /// Scroll through workspaces (+1 for next, -1 for previous).
    ScrollWorkspace(i32),
    /// Switch to the next keyboard layout.
    NextKeyboardLayout,
    /// Focus a specific window by its `Window.id`.
    FocusWindow(String),
    /// Custom dispatcher command (dispatcher name, arguments).
    Custom(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: &str, app_id: &str, pid: Option<u32>) -> Window {
        Window {
            id: id.to_string(),
            app_id: app_id.to_string(),
            pid,
            ..Default::default()
        }
    }

    #[test]
    fn app_window_prefers_the_sender_pid_then_the_app_id() {
        let state = CompositorState {
            windows: vec![
                window("a", "org.telegram.desktop", Some(10)),
                window("b", "firefox", Some(20)),
            ],
            ..Default::default()
        };
        let apps = vec!["Firefox".to_string()];

        assert_eq!(
            state.app_window(Some(10), &apps).map(|w| w.id.as_str()),
            Some("a")
        );
        assert_eq!(
            state.app_window(Some(99), &apps).map(|w| w.id.as_str()),
            Some("b")
        );
        assert_eq!(state.app_window(None, &[]), None);
    }
}
