//! Wallpaper service with pluggable engine support.
//!
//! Manages wallpaper setting via external tools. Currently supports `swww`.
//! Spawns the daemon on startup and provides commands to set wallpapers.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::{debug, error, warn};

/// Supported wallpaper engines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WallpaperEngine {
    #[default]
    Swww,
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
    data: Mutable<WallpaperData>,
}

impl WallpaperSubscriber {
    /// Create a new wallpaper subscriber and start the daemon.
    pub fn new() -> Self {
        let data = Mutable::new(WallpaperData::default());
        start_daemon();
        Self { data }
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
                set_wallpaper_swww(&path);
            }
        }
    }
}

impl Default for WallpaperSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

/// Start the swww daemon if it's not already running.
fn start_daemon() {
    thread::spawn(|| {
        // Check if swww is already running
        let running = Command::new("swww")
            .arg("query")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if running {
            debug!("swww daemon already running");
            return;
        }

        debug!("Starting swww daemon");
        match Command::new("swww-daemon").spawn() {
            Ok(_) => debug!("swww daemon started"),
            Err(e) => warn!("Failed to start swww daemon: {}", e),
        }
    });
}

/// Set wallpaper using swww.
fn set_wallpaper_swww(path: &Path) {
    let path = path.to_path_buf();
    thread::spawn(move || {
        let result = Command::new("swww")
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
            Err(e) => error!("Failed to set wallpaper via swww: {}", e),
        }
    });
}
