//! Niri compositor backend.
//!
//! This module provides integration with the Niri compositor.
//! Currently a stub implementation - Niri support is not yet complete.

use anyhow::Result;
use futures_signals::signal::Mutable;
use tracing::warn;

use super::types::{CompositorCommand, CompositorState};

/// Check if Niri is available (running).
pub fn is_available() -> bool {
    std::env::var_os("NIRI_SOCKET").is_some()
}

/// Execute a compositor command.
pub fn execute_command(_cmd: CompositorCommand) -> Result<()> {
    anyhow::bail!("Niri compositor support is not yet implemented")
}

/// Fetch the full compositor state from Niri.
pub fn fetch_full_state() -> Result<CompositorState> {
    anyhow::bail!("Niri compositor support is not yet implemented")
}

/// Start the Niri event listener in a dedicated thread.
///
/// Currently a no-op - Niri support is not yet implemented.
pub fn start_listener(_data: Mutable<CompositorState>) {
    warn!("Niri compositor support is not yet implemented - no event listener started");
}
