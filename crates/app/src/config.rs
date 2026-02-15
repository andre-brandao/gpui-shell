//! Application configuration stored as a GPUI global.
//!
//! This module defines runtime configuration for shell UI layout.

use gpui::{App, Global};

/// Root application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub bar: BarConfig,
}

impl Global for Config {}

impl Default for Config {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
        }
    }
}

impl Config {
    /// Initialize the global config.
    pub fn init(cx: &mut App) {
        cx.set_global(Config::default());
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

/// Bar orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarOrientation {
    /// Horizontal bar (typically top of screen).
    #[default]
    Horizontal,
    /// Vertical bar (typically left side of screen).
    Vertical,
}

impl BarOrientation {
    #[inline(always)]
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Vertical)
    }
}

/// Per-widget configuration.
#[derive(Debug, Clone)]
pub struct WidgetConfig {
    pub name: String,
}

impl WidgetConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl From<&str> for WidgetConfig {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WidgetConfig {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Status bar configuration.
#[derive(Debug, Clone)]
pub struct BarConfig {
    /// Main axis thickness in px (height for horizontal, width for vertical).
    pub size: f32,
    /// Bar axis orientation.
    pub orientation: BarOrientation,
    /// Start section widgets (left for horizontal, top for vertical).
    pub start: Vec<WidgetConfig>,
    /// Center section widgets.
    pub center: Vec<WidgetConfig>,
    /// End section widgets (right for horizontal, bottom for vertical).
    pub end: Vec<WidgetConfig>,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            size: 32.0,
            orientation: BarOrientation::Vertical,
            start: vec!["LauncherBtn".into(), "Workspaces".into(), "SysInfo".into()],
            center: vec!["ActiveWindow".into()],
            end: vec![
                "Clock".into(),
                "Systray".into(),
                "KeyboardLayout".into(),
                "Settings".into(),
            ],
        }
    }
}
