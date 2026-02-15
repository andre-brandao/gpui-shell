//! Application configuration stored as a GPUI global.

mod bar;
mod launcher;
mod persistence;
mod theme_persistence;

use gpui::{App, Global};
use serde::{Deserialize, Serialize};
use ui::Theme;

pub use bar::{BarConfig, BarPosition};
pub use launcher::LauncherConfig;

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bar: BarConfig,
    pub launcher: LauncherConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            launcher: LauncherConfig::default(),
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

    /// Replace the global config.
    pub fn set(config: Config, cx: &mut App) {
        *cx.global_mut::<Config>() = config;
        if let Err(err) = persistence::save(cx.global::<Config>()) {
            tracing::warn!("Failed to persist config: {}", err);
        }
    }

    /// Reload config from disk and replace the global config.
    pub fn reload(cx: &mut App) {
        match persistence::load() {
            Ok(config) => Self::set(config, cx),
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
