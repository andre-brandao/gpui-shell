//! Wallpaper service with pluggable engine support.
//!
//! Manages wallpaper setting via external tools. Currently supports `awww`.
//! Spawns the daemon on startup and provides commands to set wallpapers.

use std::path::PathBuf;
use std::process::Command;
use std::thread;

use tracing::{debug, warn};

use crate::jobs::JobQueue;
use crate::lifecycle::{Lifecycle, ManagedService, RunToken};

/// Supported wallpaper engines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WallpaperEngine {
    #[default]
    Awww,
}

/// Current wallpaper state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WallpaperData {
    /// Path to the currently set wallpaper, if known.
    pub current: Option<PathBuf>,
    /// Active wallpaper engine.
    pub engine: WallpaperEngine,
}

/// Commands for the wallpaper service.
#[derive(Debug, Clone)]
pub enum WallpaperCommand {
    /// Set wallpaper to the given image path.
    SetWallpaper(PathBuf),
}

/// Reactive wallpaper subscriber.
#[derive(Debug, Clone)]
pub struct WallpaperSubscriber {
    core: crate::ServiceCore<WallpaperData, WallpaperCommand>,
}

impl std::ops::Deref for WallpaperSubscriber {
    type Target = crate::ServiceCore<WallpaperData, WallpaperCommand>;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl Default for WallpaperSubscriber {
    fn default() -> Self {
        Self {
            core: crate::ServiceCore::with_jobs(
                // Clicking through the picker only needs the last pick applied.
                JobQueue::new("Wallpaper").coalesce(|_, _| true),
            ),
        }
    }
}

impl WallpaperSubscriber {
    /// Create a new wallpaper subscriber. The daemon is spawned by
    /// [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
    }

    async fn execute(&self, command: WallpaperCommand) -> anyhow::Result<()> {
        match command {
            WallpaperCommand::SetWallpaper(path) => {
                let shown = path.clone();
                tokio::task::spawn_blocking(move || set_wallpaper_awww(&shown)).await??;
                self.data.lock_mut().current = Some(path);
            }
        }
        Ok(())
    }
}

impl ManagedService for WallpaperSubscriber {
    fn name(&self) -> &'static str {
        "Wallpaper"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    fn memory_bytes(&self) -> usize {
        size_of::<WallpaperData>()
            + self
                .data
                .lock_ref()
                .current
                .as_ref()
                .map_or(0, |path| path.as_os_str().len())
    }

    /// Stopping only detaches the shell: `awww-daemon` is an external process
    /// and keeps the current wallpaper up.
    fn start(&self) {
        let token = self.lifecycle.begin();

        let subscriber = self.clone();
        self.jobs.run(token.clone(), move |command| {
            let subscriber = subscriber.clone();
            async move { subscriber.execute(command).await }
        });

        start_daemon(token);
    }
}

/// Start the awww daemon if it's not already running.
fn start_daemon(token: RunToken) {
    thread::spawn(move || {
        // Check if awww is already running
        let running = Command::new("awww")
            .arg("query")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if running {
            debug!("awww daemon already running");
            token.active();
            return;
        }

        debug!("Starting awww daemon");
        match Command::new("awww-daemon").spawn() {
            Ok(_) => {
                debug!("awww daemon started");
                token.active();
            }
            Err(e) => {
                warn!("Failed to start awww daemon: {}", e);
                token.error(e.to_string());
            }
        }
    });
}

/// Set wallpaper using awww, waiting for it so the child is reaped and a
/// failure is reported instead of assumed away.
fn set_wallpaper_awww(path: &std::path::Path) -> anyhow::Result<()> {
    let status = Command::new("awww")
        .args([
            "img",
            &path.to_string_lossy(),
            "--transition-type",
            "fade",
            "--transition-duration",
            "1",
        ])
        .status()?;
    anyhow::ensure!(status.success(), "awww img exited with {status}");
    debug!("Wallpaper set to: {}", path.display());
    Ok(())
}
