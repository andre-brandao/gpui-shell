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
}

/// Status bar configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    /// Main axis thickness in px (height for horizontal, width for vertical).
    pub size: f32,
    /// Screen edge where the bar is placed.
    pub position: BarPosition,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            clock: ClockConfig::default(),
            battery: BatteryConfig::default(),
            workspaces: WorkspacesConfig::default(),
            tray: TrayConfig::default(),
            sysinfo: SysInfoConfig::default(),
            mpris: MprisConfig::default(),
            active_window: ActiveWindowConfig::default(),
            keyboard_layout: KeyboardLayoutConfig::default(),
            launcher_btn: LauncherBtnConfig::default(),
            settings: SettingsConfig::default(),
        }
    }
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            size: 32.0,
            position: BarPosition::Left,
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
