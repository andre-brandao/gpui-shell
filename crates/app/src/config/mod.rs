//! Application configuration stored as a GPUI global.

mod persistence;
mod theme_persistence;

use gpui::{App, Global};
use serde::{Deserialize, Serialize};
use services::FileWatcher;
use ui::Theme;

pub use crate::bar::config::{BarConfig, BarPosition, ModulesConfig};
pub use crate::control_center::ControlCenterConfig;
pub use crate::launcher::config::LauncherConfig;
pub use crate::notification::{NotificationConfig, NotificationPopupPosition};
pub use crate::osd::{OsdConfig, OsdPosition};

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bar: BarConfig,
    pub launcher: LauncherConfig,
    pub osd: OsdConfig,
    pub notification: NotificationConfig,
    pub control_center: ControlCenterConfig,
    /// Watch config.toml for changes and hot-reload (requires restart to change).
    pub watch_config: bool,
    /// Watch theme.toml for changes and hot-reload (requires restart to change).
    pub watch_theme: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            launcher: LauncherConfig::default(),
            osd: OsdConfig::default(),
            notification: NotificationConfig::default(),
            control_center: ControlCenterConfig::default(),
            watch_config: true,
            watch_theme: true,
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

    /// Reload theme from disk and replace the global theme.
    fn reload_theme(cx: &mut App) {
        match theme_persistence::load_theme() {
            Ok(theme) => Theme::set(theme, cx),
            Err(err) => tracing::warn!("Failed to reload theme from disk: {}", err),
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
        let config = cx.global::<Config>();
        let watch_config = config.watch_config;
        let watch_theme = config.watch_theme;

        // Start config file watcher
        if watch_config {
            let config_path = match persistence::config_path() {
                Ok(path) => path,
                Err(err) => {
                    tracing::warn!("Failed to determine config path for hot reload: {}", err);
                    return;
                }
            };

            let mut rx = FileWatcher::watch(config_path);

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

        // Start theme file watcher
        if watch_theme {
            let theme_path = match theme_persistence::theme_path() {
                Ok(path) => path,
                Err(err) => {
                    tracing::warn!("Failed to determine theme path for hot reload: {}", err);
                    return;
                }
            };

            let mut rx = FileWatcher::watch(theme_path);

            cx.spawn(async move |cx| {
                while rx.recv().await.is_some() {
                    cx.update(|cx| {
                        tracing::info!("Theme file changed, reloading");
                        Self::reload_theme(cx);
                        cx.refresh_windows();
                    });
                }
            })
            .detach();
        }
    }
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
