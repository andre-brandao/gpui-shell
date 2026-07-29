//! Notification module configuration.

use serde::{Deserialize, Serialize};
use ui::IconName;

/// Notification popup screen position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationPopupPosition {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Notification module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationConfig {
    pub center_width: f32,
    pub center_height: f32,
    pub popup_position: NotificationPopupPosition,
    pub popup_width: f32,
    pub popup_height: f32,
    pub popup_margin_top: f32,
    pub popup_margin_right: f32,
    pub popup_margin_bottom: f32,
    pub popup_margin_left: f32,
    pub popup_stack_limit: usize,
    pub popup_card_collapsed_height: f32,
    pub popup_card_expanded_height: f32,
    pub icons: NotificationIcons,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            center_width: 420.0,
            center_height: 540.0,
            popup_position: NotificationPopupPosition::TopRight,
            popup_width: 360.0,
            popup_height: 320.0,
            popup_margin_top: 0.0,
            popup_margin_right: 0.0,
            popup_margin_bottom: 0.0,
            popup_margin_left: 0.0,
            popup_stack_limit: 4,
            popup_card_collapsed_height: 92.0,
            popup_card_expanded_height: 170.0,
            icons: NotificationIcons::default(),
        }
    }
}

/// Notification icons, named from the embedded icon set (e.g. `bell = "bell_ring"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationIcons {
    pub bell: IconName,
    pub bell_off: IconName,
    pub close: IconName,
    pub dnd: IconName,
}

impl Default for NotificationIcons {
    fn default() -> Self {
        Self {
            bell: IconName::Bell,
            bell_off: IconName::BellOff,
            close: IconName::Close,
            dnd: IconName::BellOff,
        }
    }
}
