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

use futures_signals::signal::SignalExt;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::{
    ActiveConnectionInfo, AudioData, BluetoothData, BluetoothState, NetworkData, PrivacyData,
    UPowerData,
};
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

use crate::bar::widgets::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::control_center::ControlCenter;
use crate::panel::{PanelConfig, panel_placement, toggle_panel};
use crate::state::AppState;

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
        let services = AppState::services(cx).clone();
        let audio = services.audio.get();
        let bluetooth = services.bluetooth.get();
        let network = services.network.get();
        let privacy = services.privacy.get();
        let upower = services.upower.get();

        // Subscribe to audio updates
        cx.spawn({
            let mut signal = services.audio.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.audio = data;
                            cx.notify();
                        })
                        .is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        // Subscribe to bluetooth updates
        cx.spawn({
            let mut signal = services.bluetooth.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.bluetooth = data;
                            cx.notify();
                        })
                        .is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        // Subscribe to network updates
        cx.spawn({
            let mut signal = services.network.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.network = data;
                            cx.notify();
                        })
                        .is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        // Subscribe to privacy updates
        cx.spawn({
            let mut signal = services.privacy.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.privacy = data;
                            cx.notify();
                        })
                        .is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        // Subscribe to upower updates
        cx.spawn({
            let mut signal = services.upower.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.upower = data;
                            cx.notify();
                        })
                        .is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

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
    fn toggle_panel(&self, cx: &mut gpui::App) {
        let services = AppState::services(cx).clone();
        let config = Config::global(cx);
        let (anchor, margin) = panel_placement(config.bar.position, self.slot);
        let config = PanelConfig {
            width: 300.0,
            height: 380.0,
            anchor,
            margin,
            namespace: "control-center".to_string(),
        };

        toggle_panel("control-center", config, cx, move |cx| {
            ControlCenter::new(services, cx)
        });
    }

    /// Get privacy indicator icons (only when active).
    fn privacy_icons(&self) -> Vec<&'static str> {
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

        icons
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
    fn battery_text(&self) -> String {
        match &self.upower.battery {
            Some(battery) => format!("{}%", battery.percentage),
            None => String::new(),
        }
    }
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();

        let privacy_icons = self.privacy_icons();
        let volume_icon = self.volume_icon();
        let network_icon = self.network_icon();
        let bluetooth_icon = self.bluetooth_icon();
        let power_profile_icon = self.power_profile_icon();
        let battery_icon = self.battery_icon();
        let battery_text = self.battery_text();

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
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let status_error = theme.status.error;
        let text_primary = theme.text.primary;

        div()
            .id("settings-widget")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(spacing::SM))
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.toggle_panel(cx);
                }),
            )
            // Privacy icons (red, only shown when active)
            .children(privacy_icons.into_iter().map(move |icon| {
                div()
                    .text_size(px(icon_size::LG))
                    .text_color(status_error)
                    .child(icon)
            }))
            // Volume icon
            .child(
                div()
                    .text_size(px(icon_size::LG))
                    .text_color(text_primary)
                    .child(volume_icon),
            )
            // Network icon
            .child(
                div()
                    .text_size(px(icon_size::LG))
                    .text_color(text_primary)
                    .child(network_icon),
            )
            // Bluetooth icon (only when connected)
            .when_some(bluetooth_icon, |el, icon| {
                el.child(
                    div()
                        .text_size(px(icon_size::LG))
                        .text_color(text_primary)
                        .child(icon),
                )
            })
            // Power profile icon
            .child(
                div()
                    .text_size(px(icon_size::LG))
                    .text_color(text_primary)
                    .child(power_profile_icon),
            )
            // Battery icon and percentage
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(icon_size::LG))
                            .text_color(battery_color)
                            .child(battery_icon),
                    )
                    .when(!battery_text.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(px(font_size::LG))
                                .text_color(battery_color)
                                .child(battery_text),
                        )
                    }),
            )
    }
}
