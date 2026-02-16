//! Application configuration stored as a GPUI global.

mod bar;
mod launcher;
mod osd;
mod persistence;
mod theme_persistence;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use gpui::{App, Global};
use inotify::{EventMask, Inotify, WatchMask};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use ui::Theme;

pub use bar::{BarConfig, BarPosition};
pub use launcher::LauncherConfig;
pub use osd::{OsdConfig, OsdPosition};

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bar: BarConfig,
    pub launcher: LauncherConfig,
    pub osd: OsdConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            launcher: LauncherConfig::default(),
            osd: OsdConfig::default(),
        }
    }
}

impl Global for Config {}

impl Config {
    /// Initialize the global config.
    pub fn init(cx: &mut App) {
        let config = match persistence::load() {
            Ok(config) => config,
            Err(err) => {
                tracing::warn!("Failed to load config, using defaults: {}", err);
                Config::default()
            }
        };

        let theme = match theme_persistence::load_theme() {
            Ok(theme) => theme,
            Err(err) => {
                tracing::warn!("Failed to load theme, using defaults: {}", err);
                Theme::default()
            }
        };

        cx.set_global(theme);
        cx.set_global(config);
        Self::start_hot_reload(cx);
    }

    /// Get the global config.
    #[inline(always)]
    pub fn global(cx: &App) -> &Config {
        cx.global::<Config>()
    }

    /// Get the global config mutably.
    #[inline(always)]
    pub fn global_mut(cx: &mut App) -> &mut Config {
        cx.global_mut::<Config>()
    }

    /// Replace the global config without persisting it.
    fn replace(config: Config, cx: &mut App) {
        *cx.global_mut::<Config>() = config;
    }

    /// Replace and persist the global config.
    pub fn set(config: Config, cx: &mut App) {
        Self::replace(config, cx);
        if let Err(err) = persistence::save(cx.global::<Config>()) {
            tracing::warn!("Failed to persist config: {}", err);
        }
    }

    /// Reload config from disk and replace the global config.
    pub fn reload(cx: &mut App) {
        match persistence::load() {
            Ok(config) => Self::replace(config, cx),
            Err(err) => tracing::warn!("Failed to reload config from disk: {}", err),
        }
    }

    /// Persist the current config to disk.
    pub fn save(cx: &App) -> anyhow::Result<()> {
        persistence::save(cx.global::<Config>())
    }

    /// Persist a provided config to disk.
    pub fn save_config(config: &Config) -> anyhow::Result<()> {
        persistence::save(config)
    }

    /// Persist current global theme colors to `theme.toml`.
    pub fn save_theme(cx: &App) -> anyhow::Result<()> {
        theme_persistence::save_theme(Theme::global(cx))
    }

    /// Persist a provided theme colors to `theme.toml`.
    pub fn save_theme_value(theme: &Theme) -> anyhow::Result<()> {
        theme_persistence::save_theme(theme)
    }

    fn start_hot_reload(cx: &mut App) {
        let path = match persistence::config_path() {
            Ok(path) => path,
            Err(err) => {
                tracing::warn!("Failed to determine config path for hot reload: {}", err);
                return;
            }
        };

        let (tx, mut rx) = mpsc::unbounded_channel::<()>();
        thread::spawn(move || {
            if let Err(err) = watch_config(path, tx) {
                tracing::warn!("Config hot reload watcher stopped: {}", err);
            }
        });

        cx.spawn(async move |cx| {
            while rx.recv().await.is_some() {
                cx.update(|cx| {
                    tracing::info!("Config file changed, reloading");
                    Config::reload(cx);
                    cx.refresh_windows();
                });
            }
        })
        .detach();
    }
}

fn watch_config(path: PathBuf, tx: mpsc::UnboundedSender<()>) -> anyhow::Result<()> {
    let mut inotify = Inotify::init()?;
    let (watch_dir, watch_name) = watch_target(&path)?;

    inotify.watches().add(
        watch_dir,
        WatchMask::MODIFY
            | WatchMask::CLOSE_WRITE
            | WatchMask::CREATE
            | WatchMask::DELETE
            | WatchMask::MOVED_TO
            | WatchMask::MOVE_SELF
            | WatchMask::DELETE_SELF,
    )?;

    let mut buffer = [0u8; 4096];
    let mut last_sent: Option<Instant> = None;

    loop {
        let events = inotify.read_events_blocking(&mut buffer)?;
        let mut should_reload = false;

        for event in events {
            let renamed_or_deleted = event.mask.contains(EventMask::MOVE_SELF)
                || event.mask.contains(EventMask::DELETE_SELF);
            let same_file = event
                .name
                .map(|name| name == watch_name.as_os_str())
                .unwrap_or(false);
            if renamed_or_deleted || same_file {
                should_reload = true;
                break;
            }
        }

        if should_reload {
            let now = Instant::now();
            let debounce_elapsed = last_sent
                .map(|last| now.duration_since(last) >= Duration::from_millis(200))
                .unwrap_or(true);
            if debounce_elapsed {
                if tx.send(()).is_err() {
                    break;
                }
                last_sent = Some(now);
            }
        }
    }

    Ok(())
}

fn watch_target(path: &Path) -> anyhow::Result<(&Path, OsString)> {
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid config path has no parent directory: {}",
            path.display()
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        anyhow::anyhow!("Invalid config path has no filename: {}", path.display())
    })?;
    Ok((parent, name.to_os_string()))
}

/// Trait for accessing active app configuration from `App`.
pub trait ActiveConfig {
    fn config(&self) -> &Config;
}

impl ActiveConfig for App {
    #[inline(always)]
    fn config(&self) -> &Config {
        Config::global(self)
    }
}
