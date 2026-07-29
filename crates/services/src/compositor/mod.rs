//! Compositor service for workspace and monitor management.
//!
//! This module provides an event-driven subscriber for monitoring compositor state
//! (workspaces, monitors, active window, keyboard layout) and executing commands.
//! It supports multiple compositor backends (Hyprland, Niri, Mango).
//!
//! Uses incremental updates with direct Mutable mutation for efficiency.

pub mod hyprland;
pub mod mango;
pub mod niri;
pub mod types;

use anyhow::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::info;

use crate::lifecycle::{Lifecycle, ManagedService, ServiceMode};
pub use types::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, Monitor, Window,
    WindowGeometry, Workspace,
};

/// Event-driven compositor subscriber.
///
/// This subscriber monitors compositor state (workspaces, monitors, windows)
/// and provides reactive state updates through `futures_signals`.
///
/// Uses incremental updates - only the changed fields are updated on each event,
/// rather than refetching the entire state.
#[derive(Debug, Clone)]
pub struct CompositorSubscriber {
    data: Mutable<CompositorState>,
    backend: CompositorBackend,
    lifecycle: Lifecycle,
}

impl CompositorSubscriber {
    /// Create a new compositor subscriber and start monitoring.
    ///
    /// Automatically detects the running compositor backend.
    /// Returns an error if no supported compositor is detected.
    pub async fn new() -> Result<Self> {
        let backend = detect_backend().ok_or_else(|| {
            anyhow::anyhow!("No supported compositor detected (Hyprland, Niri or Mango)")
        })?;

        info!("Detected compositor backend: {}", backend.name());

        // Fetch initial state
        let initial_state = match backend {
            CompositorBackend::Hyprland => hyprland::fetch_full_state()?,
            CompositorBackend::Niri => niri::fetch_full_state()?,
            CompositorBackend::Mango => mango::fetch_full_state()?,
        };

        let data = Mutable::new(initial_state);

        // Start the event listener (runs in a dedicated thread with sync handlers)
        match backend {
            CompositorBackend::Hyprland => hyprland::start_listener(data.clone()),
            CompositorBackend::Niri => niri::start_listener(data.clone()),
            CompositorBackend::Mango => mango::start_listener(data.clone()),
        }

        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        lifecycle.begin().active();

        Ok(Self {
            data,
            backend,
            lifecycle,
        })
    }

    /// Get a signal that emits when compositor state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<CompositorState> {
        self.data.signal_cloned()
    }

    /// Get the current state snapshot.
    pub fn get(&self) -> CompositorState {
        self.data.get_cloned()
    }

    /// Get the detected compositor backend.
    pub fn backend(&self) -> CompositorBackend {
        self.backend
    }

    /// Execute a compositor command.
    pub fn dispatch(&self, command: CompositorCommand) -> Result<()> {
        match self.backend {
            CompositorBackend::Hyprland => hyprland::execute_command(command),
            CompositorBackend::Niri => niri::execute_command(command),
            CompositorBackend::Mango => mango::execute_command(command),
        }
    }

    /// Force a full state refresh.
    ///
    /// Normally not needed as incremental updates keep state in sync,
    /// but can be useful if state gets out of sync for some reason.
    pub fn refresh(&self) -> Result<()> {
        let new_state = match self.backend {
            CompositorBackend::Hyprland => hyprland::fetch_full_state()?,
            CompositorBackend::Niri => niri::fetch_full_state()?,
            CompositorBackend::Mango => mango::fetch_full_state()?,
        };
        self.data.set(new_state);
        Ok(())
    }
}

impl ManagedService for CompositorSubscriber {
    fn name(&self) -> &'static str {
        "Compositor"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    fn memory_bytes(&self) -> usize {
        let data = self.data.lock_ref();
        size_of::<CompositorState>()
            + data.keyboard_layout.len()
            + data.submap.as_ref().map_or(0, String::len)
            + data
                .workspaces
                .iter()
                .map(|ws| size_of::<Workspace>() + ws.name.len() + ws.monitor.len())
                .sum::<usize>()
            + data
                .monitors
                .iter()
                .map(|monitor| size_of::<Monitor>() + monitor.name.len())
                .sum::<usize>()
            + data
                .windows
                .iter()
                .map(|window| {
                    size_of::<Window>()
                        + window.id.len()
                        + window.app_id.len()
                        + window.title.len()
                        + window.monitor.len()
                })
                .sum::<usize>()
            + data.active_window.as_ref().map_or(0, |window| {
                size_of::<ActiveWindow>()
                    + window.title.len()
                    + window.class.len()
                    + window.address.len()
            })
    }

    /// The shell has no meaning without a compositor connection, so this one
    /// is brought up by [`CompositorSubscriber::new`] and shown read-only.
    fn start(&self) {}

    fn controllable(&self) -> bool {
        false
    }
}

/// Detect which compositor backend is available.
fn detect_backend() -> Option<CompositorBackend> {
    if hyprland::is_available() {
        Some(CompositorBackend::Hyprland)
    } else if niri::is_available() {
        Some(CompositorBackend::Niri)
    } else if mango::is_available() {
        Some(CompositorBackend::Mango)
    } else {
        None
    }
}
