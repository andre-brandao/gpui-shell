//! Settings widget showing system status icons and opening a placeholder control center.
//!
//! Displays:
//! - Privacy indicators (mic/webcam/screenshare) when active
//! - Volume
//! - Network
//! - Bluetooth (if connected)
//! - Power profile
//! - Battery icon + percentage

use crate::panel::{PanelConfig, toggle_panel};
use futures_signals::signal::SignalExt;
use gpui::{
    App, Context, Hsla, MouseButton, Window, div, layer_shell::Anchor, prelude::*, px, rems,
};
use services::{
    ActiveConnectionInfo, AudioData, BluetoothData, BluetoothState, NetworkData, PrivacyData,
    Services, UPowerData,
};
use ui::prelude::*;

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

    // Battery (fallback)
    pub const BATTERY_NONE: &str = "󰂑";
}

pub struct Settings {
    services: Services,
    audio: AudioData,
    bluetooth: BluetoothData,
    network: NetworkData,
    privacy: PrivacyData,
    upower: UPowerData,
}

impl Settings {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
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
                    if this
                        .update(cx, |this, cx| {
                            this.audio = data;
                            cx.notify();
                        })
                        .is_err()
                    {
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
                    if this
                        .update(cx, |this, cx| {
                            this.bluetooth = data;
                            cx.notify();
                        })
                        .is_err()
                    {
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
                    if this
                        .update(cx, |this, cx| {
                            this.network = data;
                            cx.notify();
                        })
                        .is_err()
                    {
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
                    if this
                        .update(cx, |this, cx| {
                            this.privacy = data;
                            cx.notify();
                        })
                        .is_err()
                    {
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
                    if this
                        .update(cx, |this, cx| {
                            this.upower = data;
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
        })
        .detach();

        Settings {
            services,
            audio,
            bluetooth,
            network,
            privacy,
            upower,
        }
    }

    fn toggle_panel(&self, cx: &mut App) {
        let summary = self.summary_strings();
        let config = PanelConfig {
            width: 320.0,
            height: 260.0,
            anchor: Anchor::TOP | Anchor::RIGHT,
            margin: (0.0, 8.0, 0.0, 0.0),
            namespace: "control-center".to_string(),
        };

        toggle_panel("control-center", config, cx, move |cx| {
            ControlCenterStub::new(summary.clone(), cx)
        });
    }

    fn privacy_icons(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.privacy.microphone_access() {
            out.push(icons::MICROPHONE);
        }
        if self.privacy.webcam_access() {
            out.push(icons::WEBCAM);
        }
        if self.privacy.screenshare_access() {
            out.push(icons::SCREENSHARE);
        }
        out
    }

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

    fn network_icon(&self) -> &'static str {
        if self
            .network
            .active_connections
            .iter()
            .any(|c| matches!(c, ActiveConnectionInfo::Wired { .. }))
        {
            return icons::ETHERNET;
        }
        if !self.network.wifi_enabled {
            return icons::WIFI_OFF;
        }
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

    fn bluetooth_icon(&self) -> Option<&'static str> {
        if self.bluetooth.state == BluetoothState::Active
            && self.bluetooth.devices.iter().any(|d| d.connected)
        {
            Some(icons::BLUETOOTH_CONNECTED)
        } else {
            None
        }
    }

    fn power_profile_icon(&self) -> &'static str {
        self.upower.power_profile.icon()
    }

    fn battery_icon(&self) -> &'static str {
        match &self.upower.battery {
            Some(battery) => battery.icon(),
            None => icons::BATTERY_NONE,
        }
    }

    fn battery_text(&self) -> String {
        match &self.upower.battery {
            Some(battery) => format!("{}%", battery.percentage),
            None => String::new(),
        }
    }

    fn battery_color(&self, cx: &Context<Self>) -> Hsla {
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        match &self.upower.battery {
            Some(battery) => {
                if battery.is_critical() {
                    status.error
                } else if battery.is_low() {
                    status.warning
                } else if battery.is_charging() {
                    status.success
                } else {
                    colors.text
                }
            }
            None => colors.text_muted,
        }
    }

    fn summary_strings(&self) -> Vec<String> {
        let battery = self.battery_text();
        let net = match self.network.active_connections.iter().find(|c| {
            matches!(
                c,
                ActiveConnectionInfo::WiFi { .. }
                    | ActiveConnectionInfo::Wired { .. }
                    | ActiveConnectionInfo::Vpn { .. }
            )
        }) {
            Some(ActiveConnectionInfo::WiFi { name, .. }) => format!("Wi-Fi: {}", name),
            Some(ActiveConnectionInfo::Wired { name, .. }) => format!("Wired: {}", name),
            Some(ActiveConnectionInfo::Vpn { name, .. }) => format!("VPN: {}", name),
            _ => "Network: disconnected".to_string(),
        };
        let vol = format!(
            "Volume: {}{}",
            self.audio.sink_volume,
            if self.audio.sink_muted {
                "% (muted)"
            } else {
                "%"
            }
        );
        let power = format!("Profile: {}", self.upower.power_profile.label());
        let bt = if self.bluetooth.devices.iter().any(|d| d.connected) {
            "Bluetooth: connected".to_string()
        } else {
            "Bluetooth: off/disconnected".to_string()
        };
        let batt = if battery.is_empty() {
            "Battery: n/a".to_string()
        } else {
            format!("Battery: {}", battery)
        };
        vec![net, vol, power, bt, batt]
    }
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let status = cx.theme().status();

        let privacy_icons = self.privacy_icons();
        let volume_icon = self.volume_icon();
        let network_icon = self.network_icon();
        let bluetooth_icon = self.bluetooth_icon();
        let power_profile_icon = self.power_profile_icon();
        let battery_icon = self.battery_icon();
        let battery_text = self.battery_text();
        let battery_color = self.battery_color(cx);

        let hover_bg = colors.element_hover;
        let active_bg = colors.element_active;

        div()
            .id("settings-widget")
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .py(px(5.0))
            .rounded(px(9.0))
            .cursor_pointer()
            .bg(colors.element_background)
            .hover(move |s| s.bg(hover_bg))
            .active(move |s| s.bg(active_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.toggle_panel(cx);
                }),
            )
            // Privacy icons (red, only when active)
            .children(privacy_icons.into_iter().map(move |icon| {
                div()
                    .text_size(rems(0.95))
                    .text_color(status.error)
                    .child(icon)
            }))
            // Volume
            .child(
                div()
                    .text_size(rems(0.95))
                    .text_color(colors.text)
                    .child(volume_icon),
            )
            // Network
            .child(
                div()
                    .text_size(rems(0.95))
                    .text_color(colors.text)
                    .child(network_icon),
            )
            // Bluetooth when connected
            .when_some(bluetooth_icon, |el, icon| {
                el.child(
                    div()
                        .text_size(rems(0.95))
                        .text_color(colors.text)
                        .child(icon),
                )
            })
            // Power profile
            .child(
                div()
                    .text_size(rems(0.95))
                    .text_color(colors.text)
                    .child(power_profile_icon),
            )
            // Battery icon + percent
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(3.0))
                    .child(
                        div()
                            .text_size(rems(0.95))
                            .text_color(battery_color)
                            .child(battery_icon),
                    )
                    .when(!battery_text.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(rems(0.85))
                                .text_color(battery_color)
                                .child(battery_text),
                        )
                    }),
            )
            .tooltip(ui::Tooltip::text("Open Control Center"))
    }
}

/// Minimal placeholder control center panel showing quick status summary.
struct ControlCenterStub {
    summary: Vec<String>,
}

impl ControlCenterStub {
    fn new(summary: Vec<String>, _cx: &mut Context<Self>) -> Self {
        Self { summary }
    }
}

impl Render for ControlCenterStub {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .bg(colors.background)
            .text_color(colors.text)
            .children(self.summary.iter().map(|line| {
                div()
                    .text_size(rems(0.9))
                    .text_color(colors.text_muted)
                    .child(line.clone())
            }))
    }
}
