//! Settings widget showing status icons that opens the control center panel.
//!
//! This widget displays:
//! - Privacy indicators (mic, webcam, screenshare) when active
//! - Volume icon
//! - WiFi/Network icon
//! - Bluetooth icon (when connected)
//! - Power profile icon
//! - Battery icon + percentage
//!
//! Clicking opens the Control Center panel.

use gpui::{Context, MouseButton, Window, div, prelude::*, px, Size};
use services::{
    ActiveConnectionInfo, AudioData, BluetoothData, BluetoothState, NetworkData, PrivacyData,
    UPowerData,
};
use ui::{ActiveTheme, radius};

mod config;
pub use config::SettingsConfig;

use super::style;
use crate::bar::modules::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::control_center::{
    ControlCenter, CONTROL_CENTER_PANEL_HEIGHT_COLLAPSED, CONTROL_CENTER_PANEL_WIDTH,
};
use crate::panel::{PanelConfig, panel_placement_from_event, toggle_panel};
use crate::state::{AppState, watch};

/// Nerd Font icons for status display.
mod icons {
    // Privacy
    pub const MICROPHONE: &str = "󰍬";
    pub const WEBCAM: &str = "󰄀";
    pub const SCREENSHARE: &str = "󰍹";

    // Audio
    pub const VOLUME_HIGH: &str = "󰕾";
    pub const VOLUME_MED: &str = "󰖀";
    pub const VOLUME_LOW: &str = "󰕿";
    pub const VOLUME_MUTE: &str = "󰝟";

    // Network
    pub const WIFI: &str = "󰤨";
    pub const WIFI_OFF: &str = "󰤭";
    pub const ETHERNET: &str = "󰈀";

    // Bluetooth
    pub const BLUETOOTH_CONNECTED: &str = "󰂱";

    // Battery (using BatteryInfo::icon() from services, but keep BATTERY_NONE for no-battery case)
    pub const BATTERY_NONE: &str = "󰂑";
}

/// Settings widget for the bar that shows system status icons.
pub struct Settings {
    slot: WidgetSlot,
    audio: AudioData,
    bluetooth: BluetoothData,
    network: NetworkData,
    privacy: PrivacyData,
    upower: UPowerData,
}

impl Settings {
    /// Create a new settings widget.
    pub fn new(slot: WidgetSlot, cx: &mut Context<Self>) -> Self {
        let audio = AppState::audio(cx).get();
        let bluetooth = AppState::bluetooth(cx).get();
        let network = AppState::network(cx).get();
        let privacy = AppState::privacy(cx).get();
        let upower = AppState::upower(cx).get();

        // Subscribe to audio updates
        watch(cx, AppState::audio(cx).subscribe(), |this, data, cx| {
            this.audio = data;
            cx.notify();
        });

        // Subscribe to bluetooth updates
        watch(cx, AppState::bluetooth(cx).subscribe(), |this, data, cx| {
            this.bluetooth = data;
            cx.notify();
        });

        // Subscribe to network updates
        watch(cx, AppState::network(cx).subscribe(), |this, data, cx| {
            this.network = data;
            cx.notify();
        });

        // Subscribe to privacy updates
        watch(cx, AppState::privacy(cx).subscribe(), |this, data, cx| {
            this.privacy = data;
            cx.notify();
        });

        // Subscribe to upower updates
        watch(cx, AppState::upower(cx).subscribe(), |this, data, cx| {
            this.upower = data;
            cx.notify();
        });

        Settings {
            slot,
            audio,
            bluetooth,
            network,
            privacy,
            upower,
        }
    }

    /// Toggle the control center panel.
    fn toggle_panel(
        &self,
        event: &gpui::MouseDownEvent,
        window: &Window,
        cx: &mut gpui::App,
    ) {
        let config = Config::global(cx);
        let panel_size = Size::new(
            px(CONTROL_CENTER_PANEL_WIDTH),
            px(CONTROL_CENTER_PANEL_HEIGHT_COLLAPSED),
        );
        let (anchor, margin) =
            panel_placement_from_event(config.bar.position, event, window, cx, panel_size);
        let config = PanelConfig {
            width: CONTROL_CENTER_PANEL_WIDTH,
            height: CONTROL_CENTER_PANEL_HEIGHT_COLLAPSED,
            anchor,
            margin,
            namespace: "control-center".to_string(),
        };

        toggle_panel("control-center", config, cx, move |cx| {
            ControlCenter::new(cx)
        });
    }

    /// Get privacy indicator icons (only when active).
    fn privacy_icons(&self, is_vertical: bool) -> Vec<&'static str> {
        if is_vertical {
            let mut icons = Vec::new();

            if self.privacy.microphone_access() {
                icons.push(icons::MICROPHONE);
            }
            if self.privacy.webcam_access() {
                icons.push(icons::WEBCAM);
            }
            if self.privacy.screenshare_access() {
                icons.push(icons::SCREENSHARE);
            }

            return icons;
        }

        // Horizontal mode: keep privacy signal lightweight and avoid icon crowding.
        if self.privacy.screenshare_access() {
            vec![icons::SCREENSHARE]
        } else if self.privacy.webcam_access() {
            vec![icons::WEBCAM]
        } else if self.privacy.microphone_access() {
            vec![icons::MICROPHONE]
        } else {
            Vec::new()
        }
    }

    /// Get the volume icon based on current state.
    fn volume_icon(&self) -> &'static str {
        if self.audio.sink_muted {
            icons::VOLUME_MUTE
        } else if self.audio.sink_volume >= 66 {
            icons::VOLUME_HIGH
        } else if self.audio.sink_volume >= 33 {
            icons::VOLUME_MED
        } else {
            icons::VOLUME_LOW
        }
    }

    /// Get the network icon based on current state.
    fn network_icon(&self) -> &'static str {
        // Check for wired connection first
        if self
            .network
            .active_connections
            .iter()
            .any(|c| matches!(c, ActiveConnectionInfo::Wired { .. }))
        {
            return icons::ETHERNET;
        }

        // WiFi status
        if !self.network.wifi_enabled {
            return icons::WIFI_OFF;
        }

        // Check for active WiFi connection
        if self
            .network
            .active_connections
            .iter()
            .any(|c| matches!(c, ActiveConnectionInfo::WiFi { .. }))
        {
            icons::WIFI
        } else {
            icons::WIFI_OFF
        }
    }

    /// Get bluetooth icon if any device is connected.
    fn bluetooth_icon(&self) -> Option<&'static str> {
        if self.bluetooth.state == BluetoothState::Active
            && self.bluetooth.devices.iter().any(|d| d.connected)
        {
            Some(icons::BLUETOOTH_CONNECTED)
        } else {
            None
        }
    }

    /// Get the power profile icon.
    fn power_profile_icon(&self) -> &'static str {
        self.upower.power_profile.icon()
    }

    /// Get the battery icon based on current state.
    fn battery_icon(&self) -> &'static str {
        match &self.upower.battery {
            Some(battery) => battery.icon(),
            None => icons::BATTERY_NONE,
        }
    }

    /// Get the battery percentage text.
    fn battery_text(&self, is_vertical: bool) -> String {
        match &self.upower.battery {
            Some(battery) => {
                if is_vertical {
                    battery.percentage.to_string()
                } else {
                    format!("{}%", battery.percentage)
                }
            }
            None => String::new(),
        }
    }
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();

        let privacy_icons = self.privacy_icons(is_vertical);
        let has_privacy = !privacy_icons.is_empty();
        let volume_icon = self.volume_icon();
        let network_icon = self.network_icon();
        let bluetooth_icon = self.bluetooth_icon();
        let power_profile_icon = self.power_profile_icon();
        let battery_icon = self.battery_icon();
        let battery_text = self.battery_text(is_vertical);
        let icon_size = style::icon(is_vertical);
        let text_size = style::label(is_vertical);

        // Get the battery icon color
        let battery_color = match &self.upower.battery {
            Some(battery) => {
                if battery.is_critical() {
                    theme.status.error
                } else if battery.is_low() {
                    theme.status.warning
                } else if battery.is_charging() {
                    theme.status.success
                } else {
                    theme.text.primary
                }
            }
            None => theme.text.muted,
        };

        // Pre-compute colors for closures
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let status_error = theme.status.error;
        let text_primary = theme.text.primary;
        let divider_color = theme.border.subtle;

        div()
            .id("settings-widget")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(style::CHIP_GAP))
            .px(px(style::chip_padding_x(is_vertical)))
            .py(px(style::CHIP_PADDING_Y))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .bg(interactive_default)
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| {
                    this.toggle_panel(event, window, cx);
                }),
            )
            // Privacy icons (red, only shown when active)
            .children(privacy_icons.into_iter().map(move |icon| {
                div()
                    .text_size(px(icon_size))
                    .text_color(status_error)
                    .child(icon)
            }))
            .when(!is_vertical && has_privacy, |el| {
                el.child(
                    div()
                        .w(px(1.0))
                        .h(px(style::SECTION_DIVIDER_HEIGHT))
                        .bg(divider_color),
                )
            })
            // Volume icon
            .child(
                div()
                    .text_size(px(icon_size))
                    .text_color(text_primary)
                    .child(volume_icon),
            )
            // Network icon
            .child(
                div()
                    .text_size(px(icon_size))
                    .text_color(text_primary)
                    .child(network_icon),
            )
            // Bluetooth icon (only when connected)
            .when_some(bluetooth_icon, |el, icon| {
                el.child(
                    div()
                        .text_size(px(icon_size))
                        .text_color(text_primary)
                        .child(icon),
                )
            })
            // Power profile icon
            .when(is_vertical, |el| {
                el.child(
                    div()
                        .text_size(px(icon_size))
                        .text_color(text_primary)
                        .child(power_profile_icon),
                )
            })
            .when(!is_vertical, |el| {
                el.child(
                    div()
                        .w(px(1.0))
                        .h(px(style::SECTION_DIVIDER_HEIGHT))
                        .bg(divider_color),
                )
            })
            // Battery icon and percentage
            .child(
                div()
                    .flex()
                    .when(is_vertical, |this| this.flex_col())
                    .items_center()
                    .gap(px(style::CHIP_GAP))
                    .child(
                        div()
                            .text_size(px(icon_size))
                            .text_color(battery_color)
                            .child(battery_icon),
                    )
                    .when(!battery_text.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(px(text_size))
                                .text_color(battery_color)
                                .child(battery_text),
                        )
                    }),
            )
    }
}
