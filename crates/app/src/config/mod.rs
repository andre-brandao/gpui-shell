//! Application configuration stored as a GPUI global.

mod persistence;
mod theme;

use std::collections::BTreeMap;

use gpui::{App, Global};
use serde::{Deserialize, Serialize};
use services::{FileWatcher, ServiceMode};
use ui::Theme;

pub use crate::bar::config::{BarConfig, BarPosition, ModulesConfig};
pub use crate::control_center::ControlCenterConfig;
pub use crate::dock::DockConfig;
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
    pub dock: DockConfig,
    /// Startup mode per service, keyed by the lowercased service name
    /// (`audio`, `tray`, ...). Services absent from the map start eagerly.
    pub services: BTreeMap<String, ServiceMode>,
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
            dock: DockConfig::default(),
            services: BTreeMap::new(),
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

        let theme = match theme::persistence::load_theme() {
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

    /// Startup mode configured for a service.
    pub fn service_mode(&self, name: &str) -> ServiceMode {
        self.services
            .get(&name.to_lowercase())
            .copied()
            .unwrap_or_default()
    }

    /// Set a service's startup mode and persist it.
    pub fn set_service_mode(name: &str, mode: ServiceMode, cx: &mut App) {
        Self::global_mut(cx)
            .services
            .insert(name.to_lowercase(), mode);
        if let Err(err) = Self::save(cx) {
            tracing::warn!("Failed to persist service mode: {}", err);
        }
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
        match theme::persistence::load_theme() {
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
        theme::persistence::save_theme(Theme::global(cx))
    }

    /// Persist a provided theme colors to `theme.toml`.
    pub fn save_theme_value(theme: &Theme) -> anyhow::Result<()> {
        theme::persistence::save_theme(theme)
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
            let theme_path = match theme::persistence::theme_path() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_modes_round_trip_through_toml() {
        let mut config = Config::default();
        assert_eq!(config.service_mode("Tray"), ServiceMode::Eager);

        config
            .services
            .insert("tray".to_string(), ServiceMode::Lazy);
        config
            .services
            .insert("bluetooth".to_string(), ServiceMode::Off);

        let encoded = toml::to_string_pretty(&config).expect("config serializes");
        let decoded: Config = toml::from_str(&encoded).expect("config parses back");

        assert_eq!(decoded.service_mode("Tray"), ServiceMode::Lazy);
        assert_eq!(decoded.service_mode("Bluetooth"), ServiceMode::Off);
        assert_eq!(decoded.service_mode("Audio"), ServiceMode::Eager);
    }

    /// Parses the config actually on this machine, if there is one. Skips
    /// silently when there is not, so CI stays hermetic.
    #[test]
    fn the_live_config_on_this_machine_parses() {
        let Ok(path) = super::persistence::config_path() else {
            return;
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return;
        };

        if let Err(err) = toml::from_str::<Config>(&raw) {
            panic!("live config at {} does not parse: {err}", path.display());
        }
    }

    /// Configs written before the SVG icon switch still hold Nerd Font
    /// glyphs. One of those must not fail the parse - doing so drops every
    /// other setting in the file, which is how a `position = "top"` bar
    /// silently came back vertical.
    #[test]
    fn a_stale_icon_glyph_does_not_discard_the_rest_of_the_config() {
        let raw = concat!(
            "[bar]\n",
            "position = \"top\"\n",
            "\n",
            "[bar.modules.launcher_btn]\n",
            "icon = \"\u{f003b}\"\n",
            "\n",
            "[notification.icons]\n",
            "bell = \"\u{f009a}\"\n",
        );

        let config: Config = toml::from_str(raw).expect("config parses despite stale icons");

        assert_eq!(config.bar.position, crate::bar::config::BarPosition::Top);
        assert_eq!(config.bar.modules.launcher_btn.icon, None);
        assert_eq!(config.notification.icons.bell, None);
    }
}
