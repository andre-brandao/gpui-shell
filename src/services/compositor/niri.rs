use super::Compositor;
use super::types::CompositorCommand;
use crate::services::ServiceEvent;
use anyhow::Result;
use std::sync::mpsc;

pub fn is_available() -> bool {
    // Check for niri socket
    std::env::var_os("NIRI_SOCKET").is_some()
}

pub async fn run_listener(_tx: &mpsc::Sender<ServiceEvent<Compositor>>) -> Result<()> {
    // TODO: Implement niri event listener
    // For now, just return Ok to avoid blocking
    log::warn!("Niri compositor support is not yet implemented");
    std::future::pending::<()>().await;
    Ok(())
}

pub fn execute_command_sync(_cmd: CompositorCommand) -> Result<()> {
    // TODO: Implement niri command execution
    anyhow::bail!("Niri compositor support is not yet implemented")
}
