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

/// Complete compositor state snapshot.
#[derive(Debug, Clone, Default)]
pub struct CompositorState {
    /// All workspaces.
    pub workspaces: Vec<Workspace>,
    /// All monitors.
    pub monitors: Vec<Monitor>,
    /// ID of the currently active workspace.
    pub active_workspace_id: Option<i32>,
    /// Currently focused window (if any).
    pub active_window: Option<ActiveWindow>,
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
}

/// Supported compositor backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompositorBackend {
    #[default]
    Hyprland,
    Niri,
}

impl CompositorBackend {
    /// Get a human-readable name for this backend.
    pub fn name(&self) -> &'static str {
        match self {
            CompositorBackend::Hyprland => "Hyprland",
            CompositorBackend::Niri => "Niri",
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
    /// Custom dispatcher command (dispatcher name, arguments).
    Custom(String, String),
}
