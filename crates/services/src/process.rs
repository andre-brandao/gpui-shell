//! Children the shell starts and never waits on.

use std::io;
use std::process::Command;
use std::thread;

/// Spawn `command` and reap it on a thread so it never lingers as a zombie.
pub fn spawn_reaped(mut command: Command, label: impl Into<String>) -> io::Result<()> {
    let mut child = command.spawn()?;
    let label = label.into();
    thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => tracing::debug!("{label} exited with {status}"),
        Err(err) => tracing::warn!("Failed to reap {label}: {err}"),
        Ok(_) => {}
    });
    Ok(())
}
