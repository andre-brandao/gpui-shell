//! Application configuration stored as a GPUI global.

mod persistence;
mod state;
mod theme;

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context;
use gpui::{App, Global};
use serde::{Deserialize, Serialize};
use services::{FileWatcher, ServiceMode};
use ui::{StoredTheme, Theme};

pub use state::State;

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
    /// Setting a mode from `;s` writes `state.toml`, which then overrides
    /// this per service.
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
    ///
    /// A file we can't parse costs the session its settings, but never the
    /// file itself: the app only ever writes `state.toml` (see
    /// [`State`]), and both watchers keep the last good value until the
    /// next save fixes the file. `gpuishell --validate` reports where the
    /// parse gave up.
    pub fn init(cx: &mut App) {
        let config = match persistence::load() {
            Ok(config) => config,
            Err(err) => {
                tracing::warn!("Failed to load config, using defaults: {err:#}");
                Config::default()
            }
        };

        let theme = match theme::persistence::load_theme() {
            Ok(theme) => theme,
            Err(err) => {
                tracing::warn!("Failed to load theme, using defaults: {err:#}");
                Theme::default()
            }
        };

        cx.set_global(theme);
        cx.set_global(config);
        State::init(cx);
        Self::start_hot_reload(cx);
    }

    /// Get the global config.
    #[inline(always)]
    pub fn global(cx: &App) -> &Config {
        cx.global::<Config>()
    }

    /// Reload config from disk and replace the global config. A file that
    /// stopped parsing mid-edit leaves the running config alone.
    pub fn reload(cx: &mut App) {
        match persistence::load() {
            Ok(config) => *cx.global_mut::<Config>() = config,
            Err(err) => tracing::warn!("Failed to reload config from disk: {err:#}"),
        }
    }

    /// Reload theme from disk and replace the global theme.
    fn reload_theme(cx: &mut App) {
        match theme::persistence::load_theme() {
            Ok(theme) => Theme::set(theme, cx),
            Err(err) => tracing::warn!("Failed to reload theme from disk: {err:#}"),
        }
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
                    tracing::warn!("Failed to determine config path for hot reload: {err:#}");
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
                    tracing::warn!("Failed to determine theme path for hot reload: {err:#}");
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

/// Parse every config file without applying it, printing each verdict and any
/// parse diagnostic. `false` when one of them failed, so `--validate` can exit
/// non-zero - the point is to check a file before the shell reloads it.
pub fn validate() -> bool {
    fn check<T: serde::de::DeserializeOwned>(path: anyhow::Result<PathBuf>) -> bool {
        let path = match path {
            Ok(path) => path,
            Err(err) => {
                eprintln!("{err:#}");
                return false;
            }
        };

        if !path.exists() {
            println!("{}: absent, defaults apply", path.display());
            return true;
        }

        let parsed = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))
            .and_then(|raw| Ok(toml::from_str::<T>(&raw)?));

        match parsed {
            Ok(_) => {
                println!("{}: ok", path.display());
                true
            }
            Err(err) => {
                eprintln!("{}:\n{err:#}", path.display());
                false
            }
        }
    }

    // `&`, not `&&`: report every file rather than stopping at the first bad one.
    check::<Config>(persistence::config_path())
        & check::<StoredTheme>(theme::persistence::theme_path())
        & check::<State>(state::state_path())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-written service modes stay readable from `config.toml`; the
    /// overlay that `state.toml` puts on top of them is tested in `state`.
    #[test]
    fn service_modes_round_trip_through_toml() {
        let mut config = Config::default();
        config
            .services
            .insert("tray".to_string(), ServiceMode::Lazy);
        config
            .services
            .insert("bluetooth".to_string(), ServiceMode::Off);

        let encoded = toml::to_string_pretty(&config).expect("config serializes");
        let decoded: Config = toml::from_str(&encoded).expect("config parses back");

        assert_eq!(decoded.services.get("tray"), Some(&ServiceMode::Lazy));
        assert_eq!(decoded.services.get("bluetooth"), Some(&ServiceMode::Off));
        assert_eq!(decoded.services.get("audio"), None);
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
