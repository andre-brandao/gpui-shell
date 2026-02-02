//! Control Center launcher view for quick settings access.

use crate::launcher::view::{LauncherView, ViewContext};
use crate::services::audio::AudioCommand;
use crate::services::bluetooth::{BluetoothCommand, BluetoothState};
use crate::services::brightness::BrightnessCommand;
use crate::services::network::NetworkCommand;
use crate::services::upower::{BatteryStatus, PowerProfile, UPowerCommand};
use gpui::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, rgba};

/// Nerd Font icons
mod icons {
    pub const VOLUME_HIGH: &str = "󰕾";
    pub const VOLUME_MUTE: &str = "󰝟";
    pub const BRIGHTNESS: &str = "󰃠";
    pub const BLUETOOTH: &str = "󰂯";
    pub const BLUETOOTH_OFF: &str = "󰂲";
    pub const BLUETOOTH_CONNECTED: &str = "󰂱";
    pub const WIFI: &str = "󰤨";
    pub const WIFI_OFF: &str = "󰤭";
    pub const BATTERY: &str = "󰁹";
    pub const BATTERY_CHARGING: &str = "󰂄";
    pub const POWER_PROFILE: &str = "󰌪";
    pub const MICROPHONE: &str = "";
    pub const MICROPHONE_MUTE: &str = "";
}

pub struct ControlView;

#[derive(Clone)]
enum ControlItem {
    Toggle {
        id: &'static str,
        icon: String,
        title: String,
        subtitle: String,
        active: bool,
        action: ControlAction,
    },
    Slider {
        id: &'static str,
        icon: String,
        title: String,
        value: u8,
        action_increase: ControlAction,
        action_decrease: ControlAction,
    },
    Info {
        id: &'static str,
        icon: String,
        title: String,
        subtitle: String,
    },
}

#[derive(Clone)]
enum ControlAction {
    ToggleWifi,
    ToggleBluetooth,
    ToggleMute,
    ToggleMicMute,
    VolumeUp,
    VolumeDown,
    BrightnessUp,
    BrightnessDown,
    CyclePowerProfile,
}

impl ControlView {
    fn get_items(&self, vx: &ViewContext, cx: &App) -> Vec<ControlItem> {
        let audio = vx.services.audio.read(cx);
        let network = vx.services.network.read(cx);
        let bluetooth = vx.services.bluetooth.read(cx);
        let brightness = vx.services.brightness.read(cx);
        let upower = vx.services.upower.read(cx);

        let mut items = Vec::new();

        // WiFi toggle
        items.push(ControlItem::Toggle {
            id: "wifi",
            icon: if network.wifi_enabled {
                icons::WIFI.to_string()
            } else {
                icons::WIFI_OFF.to_string()
            },
            title: "WiFi".to_string(),
            subtitle: if network.wifi_enabled {
                "Enabled".to_string()
            } else {
                "Disabled".to_string()
            },
            active: network.wifi_enabled,
            action: ControlAction::ToggleWifi,
        });

        // Bluetooth toggle
        let bt_active = bluetooth.state == BluetoothState::Active;
        let bt_connected = bluetooth.devices.iter().any(|d| d.connected);
        items.push(ControlItem::Toggle {
            id: "bluetooth",
            icon: match bluetooth.state {
                BluetoothState::Active if bt_connected => icons::BLUETOOTH_CONNECTED.to_string(),
                BluetoothState::Active => icons::BLUETOOTH.to_string(),
                _ => icons::BLUETOOTH_OFF.to_string(),
            },
            title: "Bluetooth".to_string(),
            subtitle: if bt_connected {
                let connected_count = bluetooth.devices.iter().filter(|d| d.connected).count();
                format!("{} connected", connected_count)
            } else if bt_active {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            active: bt_active,
            action: ControlAction::ToggleBluetooth,
        });

        // Volume slider
        items.push(ControlItem::Slider {
            id: "volume",
            icon: if audio.sink_muted {
                icons::VOLUME_MUTE.to_string()
            } else {
                icons::VOLUME_HIGH.to_string()
            },
            title: format!(
                "Volume {}{}",
                audio.sink_volume,
                if audio.sink_muted { " (Muted)" } else { "%" }
            ),
            value: audio.sink_volume,
            action_increase: ControlAction::VolumeUp,
            action_decrease: ControlAction::VolumeDown,
        });

        // Mute toggle
        items.push(ControlItem::Toggle {
            id: "mute",
            icon: if audio.sink_muted {
                icons::VOLUME_MUTE.to_string()
            } else {
                icons::VOLUME_HIGH.to_string()
            },
            title: "Mute".to_string(),
            subtitle: if audio.sink_muted {
                "Muted".to_string()
            } else {
                "Unmuted".to_string()
            },
            active: audio.sink_muted,
            action: ControlAction::ToggleMute,
        });

        // Mic mute toggle
        items.push(ControlItem::Toggle {
            id: "mic-mute",
            icon: if audio.source_muted {
                icons::MICROPHONE_MUTE.to_string()
            } else {
                icons::MICROPHONE.to_string()
            },
            title: "Microphone".to_string(),
            subtitle: if audio.source_muted {
                "Muted".to_string()
            } else {
                format!("{}%", audio.source_volume)
            },
            active: !audio.source_muted,
            action: ControlAction::ToggleMicMute,
        });

        // Brightness slider (only if available)
        if brightness.max > 0 {
            items.push(ControlItem::Slider {
                id: "brightness",
                icon: icons::BRIGHTNESS.to_string(),
                title: format!("Brightness {}%", brightness.percentage()),
                value: brightness.percentage(),
                action_increase: ControlAction::BrightnessUp,
                action_decrease: ControlAction::BrightnessDown,
            });
        }

        // Battery info
        if let Some(battery) = &upower.battery {
            let icon = if battery.status == BatteryStatus::Charging {
                icons::BATTERY_CHARGING
            } else {
                icons::BATTERY
            };
            let status = match battery.status {
                BatteryStatus::Charging => "Charging",
                BatteryStatus::Discharging => "Discharging",
                BatteryStatus::Full => "Full",
                _ => "Unknown",
            };
            items.push(ControlItem::Info {
                id: "battery",
                icon: icon.to_string(),
                title: format!("Battery {}%", battery.percentage),
                subtitle: status.to_string(),
            });
        }

        // Power profile
        let profile_name = match upower.power_profile {
            PowerProfile::Balanced => "Balanced",
            PowerProfile::Performance => "Performance",
            PowerProfile::PowerSaver => "Power Saver",
            PowerProfile::Unknown => "Unknown",
        };
        items.push(ControlItem::Toggle {
            id: "power-profile",
            icon: icons::POWER_PROFILE.to_string(),
            title: "Power Profile".to_string(),
            subtitle: profile_name.to_string(),
            active: upower.power_profile == PowerProfile::Performance,
            action: ControlAction::CyclePowerProfile,
        });

        // Filter by query
        if !vx.query.is_empty() {
            let query_lower = vx.query.to_lowercase();
            items.retain(|item| {
                let (title, subtitle) = match item {
                    ControlItem::Toggle {
                        title, subtitle, ..
                    } => (title, subtitle),
                    ControlItem::Slider { title, .. } => (title, &String::new()),
                    ControlItem::Info {
                        title, subtitle, ..
                    } => (title, subtitle),
                };
                title.to_lowercase().contains(&query_lower)
                    || subtitle.to_lowercase().contains(&query_lower)
            });
        }

        items
    }

    fn execute_action(action: &ControlAction, vx: &ViewContext, cx: &mut App) {
        match action {
            ControlAction::ToggleWifi => {
                vx.services.network.update(cx, |network, cx| {
                    network.dispatch(NetworkCommand::ToggleWiFi, cx);
                });
            }
            ControlAction::ToggleBluetooth => {
                vx.services.bluetooth.update(cx, |bt, cx| {
                    bt.dispatch(BluetoothCommand::Toggle, cx);
                });
            }
            ControlAction::ToggleMute => {
                vx.services.audio.update(cx, |audio, cx| {
                    audio.dispatch(AudioCommand::ToggleSinkMute, cx);
                });
            }
            ControlAction::ToggleMicMute => {
                vx.services.audio.update(cx, |audio, cx| {
                    audio.dispatch(AudioCommand::ToggleSourceMute, cx);
                });
            }
            ControlAction::VolumeUp => {
                vx.services.audio.update(cx, |audio, cx| {
                    audio.dispatch(AudioCommand::AdjustSinkVolume(5), cx);
                });
            }
            ControlAction::VolumeDown => {
                vx.services.audio.update(cx, |audio, cx| {
                    audio.dispatch(AudioCommand::AdjustSinkVolume(-5), cx);
                });
            }
            ControlAction::BrightnessUp => {
                vx.services.brightness.update(cx, |brightness, cx| {
                    brightness.dispatch(BrightnessCommand::Increase(5), cx);
                });
            }
            ControlAction::BrightnessDown => {
                vx.services.brightness.update(cx, |brightness, cx| {
                    brightness.dispatch(BrightnessCommand::Decrease(5), cx);
                });
            }
            ControlAction::CyclePowerProfile => {
                vx.services.upower.update(cx, |upower, cx| {
                    upower.dispatch(UPowerCommand::CyclePowerProfile, cx);
                });
            }
        }
    }

    fn render_item(
        item: &ControlItem,
        index: usize,
        selected: bool,
        vx: &ViewContext,
    ) -> AnyElement {
        let services = vx.services.clone();

        match item {
            ControlItem::Toggle {
                id,
                icon,
                title,
                subtitle,
                active,
                action,
            } => {
                let action = action.clone();
                let active = *active;

                div()
                    .id(id.to_string())
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .when(selected, |el| el.bg(rgba(0x3b82f6ff)))
                    .when(!selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        let vx = ViewContext {
                            services: &services,
                            query: "",
                            selected_index: index,
                            prefix_char: ';',
                        };
                        Self::execute_action(&action, &vx, cx);
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .w(px(36.))
                                    .h(px(36.))
                                    .rounded(px(8.))
                                    .bg(if active {
                                        rgba(0x22c55eff)
                                    } else {
                                        rgba(0x444444ff)
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(18.))
                                    .child(icon.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgba(0x888888ff))
                                            .child(subtitle.clone()),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
            ControlItem::Slider {
                id,
                icon,
                title,
                value,
                action_increase,
                action_decrease,
            } => {
                let action_inc = action_increase.clone();
                let action_dec = action_decrease.clone();
                let value = *value;
                let services_inc = services.clone();
                let services_dec = services.clone();

                div()
                    .id(id.to_string())
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(8.))
                    .when(selected, |el| el.bg(rgba(0x3b82f6ff)))
                    .when(!selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .w(px(36.))
                                    .h(px(36.))
                                    .rounded(px(8.))
                                    .bg(rgba(0x444444ff))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(18.))
                                    .child(icon.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(title.clone()),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(6.))
                                            .bg(rgba(0x333333ff))
                                            .rounded(px(3.))
                                            .child(
                                                div()
                                                    .h_full()
                                                    .w(gpui::relative(value as f32 / 100.0))
                                                    .bg(rgba(0x3b82f6ff))
                                                    .rounded(px(3.)),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .id(format!("{}-dec", id))
                                            .w(px(28.))
                                            .h(px(28.))
                                            .rounded(px(6.))
                                            .bg(rgba(0x444444ff))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgba(0x555555ff)))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_size(px(14.))
                                            .child("−")
                                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                                let vx = ViewContext {
                                                    services: &services_dec,
                                                    query: "",
                                                    selected_index: 0,
                                                    prefix_char: ';',
                                                };
                                                Self::execute_action(&action_dec, &vx, cx);
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id(format!("{}-inc", id))
                                            .w(px(28.))
                                            .h(px(28.))
                                            .rounded(px(6.))
                                            .bg(rgba(0x444444ff))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgba(0x555555ff)))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_size(px(14.))
                                            .child("+")
                                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                                let vx = ViewContext {
                                                    services: &services_inc,
                                                    query: "",
                                                    selected_index: 0,
                                                    prefix_char: ';',
                                                };
                                                Self::execute_action(&action_inc, &vx, cx);
                                            }),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
            ControlItem::Info {
                id,
                icon,
                title,
                subtitle,
            } => div()
                .id(id.to_string())
                .w_full()
                .px(px(12.))
                .py(px(10.))
                .rounded(px(8.))
                .when(selected, |el| el.bg(rgba(0x3b82f6ff)))
                .when(!selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .w(px(36.))
                                .h(px(36.))
                                .rounded(px(8.))
                                .bg(rgba(0x444444ff))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(18.))
                                .child(icon.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgba(0x888888ff))
                                        .child(subtitle.clone()),
                                ),
                        ),
                )
                .into_any_element(),
        }
    }
}

impl LauncherView for ControlView {
    fn prefix(&self) -> &'static str {
        "cc"
    }

    fn name(&self) -> &'static str {
        "Control Center"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Quick settings and toggles"
    }

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let items = self.get_items(vx, cx);
        let count = items.len();

        let element = div()
            .flex_1()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| Self::render_item(item, i, i == vx.selected_index, vx)),
            )
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let items = self.get_items(vx, cx);

        if let Some(item) = items.get(index) {
            match item {
                ControlItem::Toggle { action, .. } => {
                    Self::execute_action(action, vx, cx);
                    false // Don't close
                }
                ControlItem::Slider {
                    action_increase, ..
                } => {
                    Self::execute_action(action_increase, vx, cx);
                    false // Don't close
                }
                ControlItem::Info { .. } => false,
            }
        } else {
            false
        }
    }
}
