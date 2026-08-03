//! Wallpaper service with pluggable engine support.
//!
//! Manages wallpaper setting via external tools. Currently supports `awww`.
//! Spawns the daemon on startup and provides commands to set wallpapers.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::{debug, error, warn};

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
#[derive(Debug, Clone, Default)]
pub struct WallpaperSubscriber {
    data: Mutable<WallpaperData>,
    lifecycle: Lifecycle,
}

impl WallpaperSubscriber {
    /// Create a new wallpaper subscriber. The daemon is spawned by
    /// [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a signal that emits when wallpaper state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<WallpaperData> {
        self.data.signal_cloned()
    }

    /// Get the current wallpaper data snapshot.
    pub fn get(&self) -> WallpaperData {
        self.data.get_cloned()
    }

    /// Execute a wallpaper command.
    pub fn dispatch(&self, command: WallpaperCommand) {
        match command {
            WallpaperCommand::SetWallpaper(path) => {
                // Optimistic update
                self.data.lock_mut().current = Some(path.clone());
                set_wallpaper_awww(&path);
            }
        }
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
        start_daemon(self.lifecycle.begin());
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

/// Set wallpaper using awww.
fn set_wallpaper_awww(path: &Path) {
    let path = path.to_path_buf();
    thread::spawn(move || {
        let result = Command::new("awww")
            .args([
                "img",
                &path.to_string_lossy(),
                "--transition-type",
                "fade",
                "--transition-duration",
                "1",
            ])
            .spawn();

        match result {
            Ok(_) => debug!("Wallpaper set to: {}", path.display()),
            Err(e) => error!("Failed to set wallpaper via awww: {}", e),
        }
    });
}
