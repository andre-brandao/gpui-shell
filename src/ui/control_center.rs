//! Control Center component for quick settings.
//!
//! Provides toggles and sliders for:
//! - Audio volume and mute
//! - Screen brightness
//! - Bluetooth on/off and device list
//! - WiFi toggle
//! - Power profile selection
//! - Battery status

use crate::services::Services;
use crate::services::audio::AudioCommand;
use crate::services::bluetooth::{BluetoothCommand, BluetoothState};
use crate::services::brightness::BrightnessCommand;
use crate::services::network::NetworkCommand;
use crate::services::upower::{BatteryStatus, PowerProfile, UPowerCommand};
use crate::theme::{accent, bg, font_size, icon_size, interactive, radius, spacing, status, text};
use gpui::{Context, FontWeight, MouseButton, ScrollHandle, Window, div, prelude::*, px};

/// Nerd Font icons for control center
pub mod icons {
    // Audio
    pub const VOLUME_HIGH: &str = "󰕾";
    pub const VOLUME_MED: &str = "󰖀";
    pub const VOLUME_LOW: &str = "󰕿";
    pub const VOLUME_MUTE: &str = "󰝟";
    pub const MICROPHONE: &str = "";
    pub const MICROPHONE_MUTE: &str = "";

    // Brightness
    pub const BRIGHTNESS_HIGH: &str = "󰃠";
    pub const BRIGHTNESS_MED: &str = "󰃟";
    pub const BRIGHTNESS_LOW: &str = "󰃞";

    // Bluetooth
    pub const BLUETOOTH: &str = "󰂯";
    pub const BLUETOOTH_OFF: &str = "󰂲";
    pub const BLUETOOTH_CONNECTED: &str = "󰂱";

    // WiFi
    pub const WIFI: &str = "󰤨";
    pub const WIFI_OFF: &str = "󰤭";

    // Power
    pub const BATTERY_FULL: &str = "󰁹";
    pub const BATTERY_HIGH: &str = "󰂁";
    pub const BATTERY_MED: &str = "󰁿";
    pub const BATTERY_LOW: &str = "󰁻";
    pub const BATTERY_CRITICAL: &str = "󰂃";
    pub const BATTERY_CHARGING: &str = "󰂄";
    pub const POWER_PROFILE: &str = "󰌪";

    // Quick settings
    pub const SETTINGS: &str = "";
    pub const DND: &str = "󰍶"; // Do not disturb
}

/// Control Center panel content.
pub struct ControlCenter {
    services: Services,
    scroll_handle: ScrollHandle,
}

impl ControlCenter {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        // Observe all relevant services for updates
        cx.observe(&services.audio, |_, _, cx| cx.notify()).detach();
        cx.observe(&services.bluetooth, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.brightness, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.network, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.upower, |_, _, cx| cx.notify())
            .detach();

        ControlCenter {
            services,
            scroll_handle: ScrollHandle::new(),
        }
    }

    /// Render a section header
    fn render_section_header(icon: &str, title: &str) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(spacing::SM))
            .mb(px(spacing::SM))
            .child(
                div()
                    .text_size(px(icon_size::MD))
                    .text_color(text::muted())
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .text_size(px(font_size::SM))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(text::muted())
                    .child(title.to_string()),
            )
    }

    /// Render a toggle button (like WiFi, Bluetooth, etc.)
    fn render_toggle(
        id: impl Into<String>,
        icon: &str,
        label: &str,
        is_active: bool,
        on_click: impl Fn(&mut gpui::App) + 'static,
    ) -> impl IntoElement {
        let bg_color = if is_active {
            interactive::toggle_on()
        } else {
            interactive::default()
        };

        div()
            .id(id.into())
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w(px(72.))
            .h(px(72.))
            .rounded(px(radius::MD))
            .bg(bg_color)
            .cursor_pointer()
            .hover(|s| {
                if is_active {
                    s.bg(interactive::toggle_on_hover())
                } else {
                    s.bg(interactive::hover())
                }
            })
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_click(cx);
            })
            .child(
                div()
                    .text_size(px(20.))
                    .mb(px(spacing::XS))
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .text_size(px(font_size::XS))
                    .text_color(if is_active {
                        text::primary()
                    } else {
                        text::secondary()
                    })
                    .child(label.to_string()),
            )
    }

    /// Render a slider control
    fn render_slider(
        id: impl Into<String>,
        icon: &str,
        value: u8,
        max: u8,
        _on_change: impl Fn(u8, &mut gpui::App) + 'static,
    ) -> impl IntoElement {
        let id_str = id.into();
        let percentage = (value as f32 / max as f32 * 100.0).round() as u8;
        let width_fraction = value as f32 / max as f32;

        div()
            .flex()
            .items_center()
            .gap(px(spacing::MD))
            .w_full()
            .child(
                div()
                    .w(px(24.))
                    .text_size(px(icon_size::LG))
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .id(id_str.clone())
                    .flex_1()
                    .h(px(32.))
                    .bg(bg::tertiary())
                    .rounded(px(radius::MD))
                    .cursor_pointer()
                    .overflow_hidden()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(gpui::relative(width_fraction))
                            .bg(accent::primary())
                            .rounded(px(radius::MD)),
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(font_size::SM))
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!("{}%", percentage)),
                    ), // Note: Click-to-set position would require bounds tracking
                       // For now, use the +/- buttons or launcher view for adjustment
            )
            .child(
                div()
                    .w(px(36.))
                    .text_size(px(font_size::SM))
                    .text_color(text::muted())
                    .text_right()
                    .child(format!("{}%", percentage)),
            )
    }

    /// Render audio controls
    fn render_audio_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let audio = self.services.audio.read(cx);
        let volume = audio.sink_volume;
        let muted = audio.sink_muted;
        let services = self.services.clone();

        let volume_icon = if muted {
            icons::VOLUME_MUTE
        } else if volume >= 66 {
            icons::VOLUME_HIGH
        } else if volume >= 33 {
            icons::VOLUME_MED
        } else {
            icons::VOLUME_LOW
        };

        let mic_muted = audio.source_muted;
        let mic_icon = if mic_muted {
            icons::MICROPHONE_MUTE
        } else {
            icons::MICROPHONE
        };

        div()
            .w_full()
            .p(px(spacing::MD))
            .bg(bg::secondary())
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .child(Self::render_section_header(icons::VOLUME_HIGH, "Audio"))
            // Volume slider
            .child({
                let services_clone = services.clone();
                Self::render_slider("volume-slider", volume_icon, volume, 100, move |val, cx| {
                    services_clone.audio.update(cx, |audio, cx| {
                        audio.dispatch(AudioCommand::SetSinkVolume(val), cx);
                    });
                })
            })
            // Mute toggles row
            .child(
                div()
                    .flex()
                    .gap(px(spacing::SM))
                    .mt(px(spacing::XS))
                    .child({
                        let services_clone = services.clone();
                        Self::render_toggle("mute-toggle", volume_icon, "Mute", muted, move |cx| {
                            services_clone.audio.update(cx, |audio, cx| {
                                audio.dispatch(AudioCommand::ToggleSinkMute, cx);
                            });
                        })
                    })
                    .child({
                        let services_clone = services.clone();
                        Self::render_toggle(
                            "mic-mute-toggle",
                            mic_icon,
                            "Mic",
                            mic_muted,
                            move |cx| {
                                services_clone.audio.update(cx, |audio, cx| {
                                    audio.dispatch(AudioCommand::ToggleSourceMute, cx);
                                });
                            },
                        )
                    }),
            )
    }

    /// Render brightness controls
    fn render_brightness_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let brightness = self.services.brightness.read(cx);
        let percentage = brightness.percentage();
        let services = self.services.clone();

        let icon = if percentage >= 66 {
            icons::BRIGHTNESS_HIGH
        } else if percentage >= 33 {
            icons::BRIGHTNESS_MED
        } else {
            icons::BRIGHTNESS_LOW
        };

        div()
            .w_full()
            .p(px(spacing::MD))
            .bg(bg::secondary())
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .child(Self::render_section_header(
                icons::BRIGHTNESS_HIGH,
                "Display",
            ))
            .child(Self::render_slider(
                "brightness-slider",
                icon,
                percentage,
                100,
                move |val, cx| {
                    services.brightness.update(cx, |brightness, cx| {
                        brightness.dispatch(BrightnessCommand::SetPercent(val), cx);
                    });
                },
            ))
    }

    /// Render quick toggles (WiFi, Bluetooth, etc.)
    fn render_quick_toggles(&self, cx: &Context<Self>) -> impl IntoElement {
        let network = self.services.network.read(cx);
        let bluetooth = self.services.bluetooth.read(cx);
        let wifi_enabled = network.wifi_enabled;
        let bt_state = bluetooth.state;
        let bt_active = bt_state == BluetoothState::Active;

        let services = self.services.clone();

        let bt_icon = match bt_state {
            BluetoothState::Active => {
                if bluetooth.devices.iter().any(|d| d.connected) {
                    icons::BLUETOOTH_CONNECTED
                } else {
                    icons::BLUETOOTH
                }
            }
            _ => icons::BLUETOOTH_OFF,
        };

        div()
            .w_full()
            .p(px(spacing::MD))
            .bg(bg::secondary())
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .child(Self::render_section_header(
                icons::SETTINGS,
                "Quick Settings",
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::SM))
                    .child({
                        let services_clone = services.clone();
                        Self::render_toggle(
                            "wifi-toggle",
                            if wifi_enabled {
                                icons::WIFI
                            } else {
                                icons::WIFI_OFF
                            },
                            "WiFi",
                            wifi_enabled,
                            move |cx| {
                                services_clone.network.update(cx, |network, cx| {
                                    network.dispatch(NetworkCommand::ToggleWiFi, cx);
                                });
                            },
                        )
                    })
                    .child({
                        let services_clone = services.clone();
                        Self::render_toggle(
                            "bt-toggle",
                            bt_icon,
                            "Bluetooth",
                            bt_active,
                            move |cx| {
                                services_clone.bluetooth.update(cx, |bt, cx| {
                                    bt.dispatch(BluetoothCommand::Toggle, cx);
                                });
                            },
                        )
                    }),
            )
    }

    /// Render Bluetooth devices list (when Bluetooth is active)
    fn render_bluetooth_devices(&self, cx: &Context<Self>) -> impl IntoElement {
        let bluetooth = self.services.bluetooth.read(cx);
        let services = self.services.clone();

        if bluetooth.state != BluetoothState::Active || bluetooth.devices.is_empty() {
            return div().into_any_element();
        }

        div()
            .w_full()
            .p(px(spacing::MD))
            .bg(bg::secondary())
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .child(Self::render_section_header(
                icons::BLUETOOTH,
                "Bluetooth Devices",
            ))
            .children(bluetooth.devices.iter().take(5).map(|device| {
                let connected = device.connected;
                let name = device.name.clone();
                let path = device.path.clone();
                let battery = device.battery;
                let services_clone = services.clone();

                div()
                    .id(format!("bt-device-{}", name))
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .px(px(spacing::SM))
                    .py(px(spacing::SM - 2.0))
                    .rounded(px(radius::SM))
                    .cursor_pointer()
                    .hover(|s| s.bg(interactive::hover()))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        services_clone.bluetooth.update(cx, |bt, cx| {
                            if connected {
                                bt.dispatch(BluetoothCommand::DisconnectDevice(path.clone()), cx);
                            } else {
                                bt.dispatch(BluetoothCommand::ConnectDevice(path.clone()), cx);
                            }
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(icon_size::MD))
                                    .text_color(if connected {
                                        accent::primary()
                                    } else {
                                        text::muted()
                                    })
                                    .child(if connected {
                                        icons::BLUETOOTH_CONNECTED
                                    } else {
                                        icons::BLUETOOTH
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(div().text_size(px(font_size::SM)).child(name.clone()))
                                    .child(
                                        div()
                                            .text_size(px(font_size::XS))
                                            .text_color(text::disabled())
                                            .child(if connected { "Connected" } else { "Paired" }),
                                    ),
                            ),
                    )
                    .when_some(battery, |el, bat| {
                        el.child(
                            div()
                                .text_size(px(font_size::XS))
                                .text_color(text::muted())
                                .child(format!("{}%", bat)),
                        )
                    })
            }))
            .into_any_element()
    }

    /// Render battery and power profile section
    fn render_power_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let upower = self.services.upower.read(cx);
        let services = self.services.clone();

        let Some(battery) = &upower.battery else {
            return div().into_any_element();
        };

        let bat_icon = match battery.status {
            BatteryStatus::Charging => icons::BATTERY_CHARGING,
            _ => {
                if battery.percentage >= 90 {
                    icons::BATTERY_FULL
                } else if battery.percentage >= 60 {
                    icons::BATTERY_HIGH
                } else if battery.percentage >= 30 {
                    icons::BATTERY_MED
                } else if battery.percentage >= 10 {
                    icons::BATTERY_LOW
                } else {
                    icons::BATTERY_CRITICAL
                }
            }
        };

        let status_text = match battery.status {
            BatteryStatus::Charging => "Charging",
            BatteryStatus::Discharging => "On Battery",
            BatteryStatus::Full => "Fully Charged",
            BatteryStatus::NotCharging => "Not Charging",
            BatteryStatus::Unknown => "Unknown",
        };

        let profile = upower.power_profile;
        let profile_name = match profile {
            PowerProfile::Balanced => "Balanced",
            PowerProfile::Performance => "Performance",
            PowerProfile::PowerSaver => "Power Saver",
            PowerProfile::Unknown => "Unknown",
        };

        div()
            .w_full()
            .p(px(spacing::MD))
            .bg(bg::secondary())
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(spacing::MD))
            .child(Self::render_section_header(icons::BATTERY_FULL, "Power"))
            // Battery info
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::MD))
                    .child(
                        div()
                            .text_size(px(24.))
                            .text_color(if battery.percentage <= 20 {
                                status::error()
                            } else if battery.status == BatteryStatus::Charging {
                                status::success()
                            } else {
                                text::primary()
                            })
                            .child(bat_icon),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(font_size::LG))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{}%", battery.percentage)),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(text::muted())
                                    .child(status_text),
                            ),
                    ),
            )
            // Power profile selector
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(icon_size::MD))
                                    .child(icons::POWER_PROFILE),
                            )
                            .child(div().text_size(px(font_size::SM)).child("Power Profile")),
                    )
                    .child(
                        div()
                            .id("power-profile-btn")
                            .px(px(10.))
                            .py(px(spacing::XS))
                            .bg(interactive::default())
                            .rounded(px(radius::SM))
                            .cursor_pointer()
                            .hover(|s| s.bg(interactive::hover()))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                services.upower.update(cx, |upower, cx| {
                                    upower.dispatch(UPowerCommand::CyclePowerProfile, cx);
                                });
                            })
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(profile_name),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for ControlCenter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("control-center")
            .w_full()
            .h_full()
            .p(px(spacing::MD))
            .flex()
            .flex_col()
            .gap(px(spacing::MD))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::SM))
                    .mb(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(icon_size::XL))
                            .text_color(text::primary())
                            .child(icons::SETTINGS),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::LG))
                            .text_color(text::primary())
                            .font_weight(FontWeight::BOLD)
                            .child("Control Center"),
                    ),
            )
            // Quick toggles (WiFi, Bluetooth)
            .child(self.render_quick_toggles(cx))
            // Bluetooth devices (if any connected)
            .child(self.render_bluetooth_devices(cx))
            // Audio controls
            .child(self.render_audio_section(cx))
            // Brightness (only on laptops with backlight)
            .when(
                self.services.brightness.read(cx).max > 0,
                |el: gpui::Stateful<gpui::Div>| el.child(self.render_brightness_section(cx)),
            )
            // Power/Battery section
            .child(self.render_power_section(cx))
    }
}
