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

use gpui::{AnyElement, Context, MouseButton, Window, div, prelude::*, px};
use services::{
    ActiveConnectionInfo, AudioData, BluetoothData, BluetoothState, NetworkData, PrivacyData,
    UPowerData,
};
use ui::ActiveTheme;

mod config;
pub use config::SettingsConfig;

use super::{BarWidget, style};
use crate::config::ActiveConfig;
use crate::control_center::{
    CONTROL_CENTER_PANEL_HEIGHT_COLLAPSED, CONTROL_CENTER_PANEL_WIDTH, ControlCenter,
};
use crate::panel::toggle_widget_panel;
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
    audio: AudioData,
    bluetooth: BluetoothData,
    network: NetworkData,
    privacy: PrivacyData,
    upower: UPowerData,
}

struct SettingsView {
    privacy_icons: Vec<&'static str>,
    volume_icon: &'static str,
    network_icon: &'static str,
    bluetooth_icon: Option<&'static str>,
    power_profile_icon: &'static str,
    battery_icon: &'static str,
    battery_text: String,
    battery_color: gpui::Hsla,
}

impl Settings {
    /// Create a new settings widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
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
            audio,
            bluetooth,
            network,
            privacy,
            upower,
        }
    }

    /// Toggle the control center panel.
    fn toggle_panel(&self, event: &gpui::MouseDownEvent, window: &Window, cx: &mut gpui::App) {
        toggle_widget_panel(
            "control-center",
            gpui::Size::new(
                CONTROL_CENTER_PANEL_WIDTH,
                CONTROL_CENTER_PANEL_HEIGHT_COLLAPSED,
            ),
            "control-center",
            event,
            window,
            cx,
            ControlCenter::new,
        );
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

    fn battery_color(&self, theme: &ui::Theme) -> gpui::Hsla {
        match &self.upower.battery {
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
        }
    }

    fn view(&self, theme: &ui::Theme, is_vertical: bool) -> SettingsView {
        SettingsView {
            privacy_icons: self.privacy_icons(is_vertical),
            volume_icon: self.volume_icon(),
            network_icon: self.network_icon(),
            bluetooth_icon: self.bluetooth_icon(),
            power_profile_icon: self.power_profile_icon(),
            battery_icon: self.battery_icon(),
            battery_text: self.battery_text(is_vertical),
            battery_color: self.battery_color(theme),
        }
    }

    fn render_status_icon(icon: &'static str, size: f32, color: gpui::Hsla) -> AnyElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .text_size(px(size))
            .text_color(color)
            .child(icon)
            .into_any_element()
    }

    fn render_battery_block(
        &self,
        theme: &ui::Theme,
        view: &SettingsView,
        is_vertical: bool,
    ) -> AnyElement {
        div()
            .flex()
            .when(is_vertical, |el| el.flex_col())
            .items_center()
            .justify_center()
            .gap(px(style::CHIP_GAP))
            .child(Self::render_status_icon(
                view.battery_icon,
                style::icon(is_vertical),
                view.battery_color,
            ))
            .when(!view.battery_text.is_empty(), |el| {
                el.child(if is_vertical {
                    style::vertical_text_line(
                        div()
                            .text_size(style::label_size(theme, is_vertical))
                            .text_color(view.battery_color)
                            .child(view.battery_text.clone()),
                    )
                } else {
                    div()
                        .text_size(style::label_size(theme, is_vertical))
                        .text_color(view.battery_color)
                        .child(view.battery_text.clone())
                        .into_any_element()
                })
            })
            .into_any_element()
    }
}

impl BarWidget for Settings {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let view = self.view(theme, true);
        let icon_size = style::icon(true);

        div()
            .id("settings-widget")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(style::CHIP_GAP))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| {
                    this.toggle_panel(event, window, cx);
                }),
            )
            // Privacy icons (red, only shown when active)
            .children(
                view.privacy_icons
                    .iter()
                    .copied()
                    .map(move |icon| Self::render_status_icon(icon, icon_size, theme.status.error)),
            )
            // Volume icon
            .child(Self::render_status_icon(
                view.volume_icon,
                icon_size,
                theme.text.primary,
            ))
            // Network icon
            .child(Self::render_status_icon(
                view.network_icon,
                icon_size,
                theme.text.primary,
            ))
            // Bluetooth icon (only when connected)
            .when_some(view.bluetooth_icon, |el, icon| {
                el.child(Self::render_status_icon(
                    icon,
                    icon_size,
                    theme.text.primary,
                ))
            })
            // Power profile icon
            .child(Self::render_status_icon(
                view.power_profile_icon,
                icon_size,
                theme.text.primary,
            ))
            // Battery icon and percentage
            .child(self.render_battery_block(theme, &view, true))
            .into_any_element()
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let bar = &cx.config().bar;
        let view = self.view(theme, false);
        let icon_size = style::icon(false);
        let divider_color = style::widget_border(theme, bar, false);

        div()
            .id("settings-widget")
            .flex()
            .items_center()
            .justify_center()
            .gap(px(3.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| {
                    this.toggle_panel(event, window, cx);
                }),
            )
            .children(
                view.privacy_icons
                    .iter()
                    .copied()
                    .map(move |icon| Self::render_status_icon(icon, icon_size, theme.status.error)),
            )
            .when(!view.privacy_icons.is_empty(), |el| {
                el.child(style::section_divider(divider_color))
            })
            .child(Self::render_status_icon(
                view.volume_icon,
                icon_size,
                theme.text.primary,
            ))
            .child(Self::render_status_icon(
                view.network_icon,
                icon_size,
                theme.text.primary,
            ))
            .when_some(view.bluetooth_icon, |el, icon| {
                el.child(Self::render_status_icon(
                    icon,
                    icon_size,
                    theme.text.primary,
                ))
            })
            .child(style::section_divider(divider_color))
            .child(self.render_battery_block(theme, &view, false))
            .into_any_element()
    }
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}
