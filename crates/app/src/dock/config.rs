//! Dock configuration.

use serde::{Deserialize, Serialize};

use crate::bar::config::BarPosition;

/// Which monitors show a dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DockMonitors {
    /// Only the primary display gets a dock (default).
    #[default]
    PrimaryOnly,
    /// Every display gets its own dock, scoped to that monitor's windows.
    All,
}

/// When the dock hides itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DockVisibility {
    /// Never hides.
    #[default]
    AlwaysVisible,
    /// Hides after a short delay once no window on the dock's monitor is
    /// focused; reveals on pointer-enter at the dock's edge, or as soon as
    /// focus returns to this monitor.
    IntelligentHide,
    /// Hides only when the focused window's bounds geometrically overlap
    /// the dock's own bounds. Falls back to `IntelligentHide`'s behavior
    /// when the focused window has no reported geometry (e.g. on Niri).
    DodgeWindows,
}

/// Hover effect applied to a dock item under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DockHoverEffect {
    /// No hover effect.
    None,
    /// Icon rises under the cursor.
    #[default]
    Lift,
    /// Icon grows under the cursor.
    Magnify,
    /// Icon gets a themed accent-color ring under the cursor.
    Glow,
    /// Icon both grows and rises under the cursor.
    MagnifyLift,
}

/// Dock configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockConfig {
    /// Screen edge the dock is anchored to. Independent of `bar.position`.
    pub position: BarPosition,
    /// Which monitors get a dock.
    pub monitors: DockMonitors,
    /// When the dock hides itself.
    pub visibility: DockVisibility,
    /// Hover effect applied to dock items.
    pub hover_effect: DockHoverEffect,
    /// Icon size in px.
    pub icon_size: f32,
    /// Pinned app identifiers (desktop file names, e.g. `"firefox.desktop"`).
    /// Pinning from the UI writes `state.toml`, which then overrides this.
    pub pinned: Vec<String>,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            position: BarPosition::Bottom,
            monitors: DockMonitors::default(),
            visibility: DockVisibility::default(),
            hover_effect: DockHoverEffect::default(),
            icon_size: 40.0,
            pinned: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DockConfig, DockHoverEffect};

    #[test]
    fn hover_effect_deserializes_magnify_lift_from_kebab_case() {
        let config: DockConfig = toml::from_str("hover_effect = \"magnify-lift\"").unwrap();

        assert_eq!(config.hover_effect, DockHoverEffect::MagnifyLift);
    }
}
