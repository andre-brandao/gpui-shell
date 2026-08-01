use serde::{Deserialize, Serialize};

use super::modules::{
    ActiveWindowConfig, BatteryConfig, ClockConfig, KeyboardLayoutConfig, LauncherBtnConfig,
    MprisConfig, SettingsConfig, SysInfoConfig, TrayConfig, WorkspacesConfig,
};

/// Bar screen position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarPosition {
    /// Left edge of the screen.
    #[default]
    Left,
    /// Right edge of the screen.
    Right,
    /// Top edge of the screen.
    Top,
    /// Bottom edge of the screen.
    Bottom,
}

impl BarPosition {
    #[inline(always)]
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// The same edge, as the UI layer names it. Config keeps its own enum
    /// because it carries the serde spelling.
    pub fn edge(self) -> ui::patterns::BarEdge {
        use ui::patterns::BarEdge;
        match self {
            Self::Left => BarEdge::Left,
            Self::Right => BarEdge::Right,
            Self::Top => BarEdge::Top,
            Self::Bottom => BarEdge::Bottom,
        }
    }
}

/// Status bar configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    /// Main axis thickness in px (height for horizontal, width for vertical).
    pub size: f32,
    /// Horizontal outer padding applied to the bar contents.
    pub padding: f32,
    /// Screen edge where the bar is placed.
    pub position: BarPosition,
    /// Whether the bar itself draws an edge border.
    pub show_border: bool,
    /// Whether widgets render a resting background fill.
    pub widget_background: bool,
    /// Whether widgets render a subtle border.
    pub widget_border: bool,
    /// Start section widgets (left for horizontal, top for vertical).
    pub start: Vec<String>,
    /// Center section widgets.
    pub center: Vec<String>,
    /// End section widgets (right for horizontal, bottom for vertical).
    pub end: Vec<String>,
    /// Bar modules
    pub modules: ModulesConfig,
}

/// Bar module configurations.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModulesConfig {
    pub clock: ClockConfig,
    pub battery: BatteryConfig,
    pub workspaces: WorkspacesConfig,
    pub tray: TrayConfig,
    pub sysinfo: SysInfoConfig,
    pub mpris: MprisConfig,
    pub active_window: ActiveWindowConfig,
    pub keyboard_layout: KeyboardLayoutConfig,
    pub launcher_btn: LauncherBtnConfig,
    pub settings: SettingsConfig,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            size: 32.0,
            padding: 14.0,
            position: BarPosition::Left,
            show_border: true,
            widget_background: true,
            widget_border: true,
            start: vec!["LauncherBtn".into(), "Workspaces".into(), "SysInfo".into()],
            center: vec!["ActiveWindow".into()],
            end: vec![
                "Clock".into(),
                "Mpris".into(),
                "Notifications".into(),
                "Systray".into(),
                "KeyboardLayout".into(),
                "Settings".into(),
            ],
            modules: ModulesConfig::default(),
        }
    }
}

impl BarConfig {
    #[inline(always)]
    pub fn is_vertical(&self) -> bool {
        self.position.is_vertical()
    }
}
