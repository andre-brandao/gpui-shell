//! Notification module configuration.

use serde::{Deserialize, Serialize};
use ui::{IconName, IconSource};

use crate::icons::{self, ConfigIcon};

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

/// Notification icons. Each is either a name from the embedded set (`bell =
/// "bell_ring"`) or a path to your own file. Omit a field - or give
/// something we can't resolve - to get the built-in icon.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationIcons {
    #[serde(with = "icon_field")]
    pub bell: Option<ConfigIcon>,
    #[serde(with = "icon_field")]
    pub bell_off: Option<ConfigIcon>,
    #[serde(with = "icon_field")]
    pub close: Option<ConfigIcon>,
    #[serde(with = "icon_field")]
    pub dnd: Option<ConfigIcon>,
}

impl NotificationIcons {
    pub fn bell(&self) -> IconSource {
        icons::source_or(self.bell.as_ref(), IconName::Bell)
    }

    pub fn bell_off(&self) -> IconSource {
        icons::source_or(self.bell_off.as_ref(), IconName::BellOff)
    }

    pub fn close(&self) -> IconSource {
        icons::source_or(self.close.as_ref(), IconName::Close)
    }

    pub fn dnd(&self) -> IconSource {
        icons::source_or(self.dnd.as_ref(), IconName::BellOff)
    }
}

/// `serde(with = ...)` pair so each icon field is both lenient on the way in
/// and omitted from the written config when it is unset.
mod icon_field {
    pub use crate::icons::deserialize_lenient as deserialize;

    pub fn serialize<S: serde::Serializer>(
        icon: &Option<crate::icons::ConfigIcon>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match icon {
            Some(icon) => serializer.serialize_some(icon),
            None => serializer.serialize_none(),
        }
    }
}
