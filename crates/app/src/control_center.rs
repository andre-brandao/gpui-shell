//! Control Center component for quick settings.
//!
//! This component is shared between:
//! - The launcher view (`:cc` or `~` prefix)
//! - The info widget panel
//!
//! Provides toggles and sliders for:
//! - Audio volume and mute
//! - Screen brightness
//! - Bluetooth on/off and device list
//! - WiFi toggle
//! - Power profile selection
//! - Battery status

use futures_signals::signal::SignalExt;
use gpui::{
    AnyElement, App, Context, FocusHandle, Focusable, FontWeight, MouseButton, ScrollHandle,
    Window, div, prelude::*, px, rgba,
};
use services::{
    AudioCommand, BluetoothCommand, BluetoothState, BrightnessCommand, NetworkCommand,
    PowerProfile, Services, UPowerCommand,
};
use ui::{accent, bg, border, font_size, icon_size, interactive, radius, spacing, status, text};

/// Nerd Font icons for control center.
pub mod icons {
    // Audio
    pub const VOLUME_HIGH: &str = "󰕾";
    pub const VOLUME_MED: &str = "󰖀";
    pub const VOLUME_LOW: &str = "󰕿";
    pub const VOLUME_MUTE: &str = "󰝟";
    pub const MICROPHONE: &str = "󰍬";
    pub const MICROPHONE_MUTE: &str = "󰍭";

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
    pub const POWER_PERFORMANCE: &str = "󱐋";
    pub const POWER_BALANCED: &str = "󰗑";

    // Settings
    pub const SETTINGS: &str = "";
}

/// Control Center panel/component.
pub struct ControlCenter {
    services: Services,
    scroll_handle: ScrollHandle,
    focus_handle: FocusHandle,
}

impl ControlCenter {
    /// Create a new control center with the given services.
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();

        // Subscribe to service updates for reactive rendering
        cx.spawn({
            let mut audio_signal = services.audio.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while audio_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        cx.spawn({
            let mut bluetooth_signal = services.bluetooth.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while bluetooth_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        cx.spawn({
            let mut brightness_signal = services.brightness.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while brightness_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        cx.spawn({
            let mut network_signal = services.network.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while network_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        cx.spawn({
            let mut upower_signal = services.upower.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while upower_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        ControlCenter {
            services,
            scroll_handle,
            focus_handle,
        }
    }

    /// Render a section header.
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

    /// Render a toggle button.
    fn render_toggle(
        id: impl Into<String>,
        icon: &str,
        label: &str,
        is_active: bool,
        on_click: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id.into())
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w(px(72.))
            .h(px(72.))
            .rounded(px(radius::MD))
            .cursor_pointer()
            .when(is_active, |el| el.bg(accent::primary()))
            .when(!is_active, |el| el.bg(interactive::default()))
            .hover(|s| s.bg(interactive::hover()))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_click(cx);
            })
            .child(
                div()
                    .text_size(px(20.))
                    .mb(px(spacing::XS))
                    .text_color(if is_active {
                        text::primary()
                    } else {
                        text::secondary()
                    })
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

    /// Render a slider control.
    fn render_slider(
        id: impl Into<String>,
        icon: &str,
        value: u8,
        muted: bool,
        on_click_icon: impl Fn(&mut App) + 'static,
        on_increase: impl Fn(&mut App) + 'static,
        on_decrease: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        let id_str = id.into();
        let percentage = value.min(100);
        let width_fraction = percentage as f32 / 100.0;

        div()
            .flex()
            .items_center()
            .gap(px(spacing::SM))
            .w_full()
            // Icon (clickable for mute toggle)
            .child(
                div()
                    .id(format!("{}-icon", id_str))
                    .w(px(32.))
                    .h(px(32.))
                    .rounded(px(radius::SM))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(muted, |el| el.bg(rgba(0xff444466)))
                    .when(!muted, |el| el.bg(interactive::default()))
                    .hover(|s| s.bg(interactive::hover()))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        on_click_icon(cx);
                    })
                    .child(
                        div()
                            .text_size(px(icon_size::MD))
                            .text_color(if muted {
                                status::error()
                            } else {
                                text::primary()
                            })
                            .child(icon.to_string()),
                    ),
            )
            // Slider track
            .child(
                div()
                    .id(id_str.clone())
                    .flex_1()
                    .h(px(8.))
                    .bg(bg::tertiary())
                    .rounded(px(4.))
                    .overflow_hidden()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(gpui::relative(width_fraction))
                            .bg(if muted {
                                text::disabled()
                            } else {
                                accent::primary()
                            })
                            .rounded(px(4.)),
                    ),
            )
            // Value display
            .child(
                div()
                    .w(px(40.))
                    .text_size(px(font_size::SM))
                    .text_color(text::secondary())
                    .text_right()
                    .child(format!("{}%", percentage)),
            )
            // +/- buttons
            .child(
                div()
                    .flex()
                    .gap(px(2.))
                    .child(
                        div()
                            .id(format!("{}-dec", id_str))
                            .w(px(24.))
                            .h(px(24.))
                            .rounded(px(radius::SM))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(interactive::default())
                            .hover(|s| s.bg(interactive::hover()))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                on_decrease(cx);
                            })
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(text::secondary())
                                    .child("−"),
                            ),
                    )
                    .child(
                        div()
                            .id(format!("{}-inc", id_str))
                            .w(px(24.))
                            .h(px(24.))
                            .rounded(px(radius::SM))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(interactive::default())
                            .hover(|s| s.bg(interactive::hover()))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                on_increase(cx);
                            })
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(text::secondary())
                                    .child("+"),
                            ),
                    ),
            )
    }

    /// Render audio controls section.
    fn render_audio_section(&self) -> impl IntoElement {
        let audio = self.services.audio.get();
        let volume = audio.sink_volume;
        let muted = audio.sink_muted;
        let mic_muted = audio.source_muted;

        let volume_icon = if muted {
            icons::VOLUME_MUTE
        } else if volume >= 66 {
            icons::VOLUME_HIGH
        } else if volume >= 33 {
            icons::VOLUME_MED
        } else {
            icons::VOLUME_LOW
        };

        let mic_icon = if mic_muted {
            icons::MICROPHONE_MUTE
        } else {
            icons::MICROPHONE
        };

        let services = self.services.clone();
        let services_inc = self.services.clone();
        let services_dec = self.services.clone();
        let services_mic = self.services.clone();

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
            .child(Self::render_slider(
                "volume-slider",
                volume_icon,
                volume,
                muted,
                move |_cx| {
                    services.audio.dispatch(AudioCommand::ToggleSinkMute);
                },
                move |_cx| {
                    services_inc
                        .audio
                        .dispatch(AudioCommand::AdjustSinkVolume(5));
                },
                move |_cx| {
                    services_dec
                        .audio
                        .dispatch(AudioCommand::AdjustSinkVolume(-5));
                },
            ))
            // Microphone toggle
            .child(div().flex().gap(px(spacing::SM)).mt(px(spacing::XS)).child(
                Self::render_toggle(
                    "mic-toggle",
                    mic_icon,
                    if mic_muted { "Mic Off" } else { "Mic On" },
                    !mic_muted,
                    move |_cx| {
                        services_mic.audio.dispatch(AudioCommand::ToggleSourceMute);
                    },
                ),
            ))
    }

    /// Render brightness controls section.
    fn render_brightness_section(&self) -> AnyElement {
        let brightness = self.services.brightness.get();

        if brightness.max == 0 {
            // No brightness control available
            return div().into_any_element();
        }

        let percentage = brightness.percentage();
        let icon = if percentage >= 66 {
            icons::BRIGHTNESS_HIGH
        } else if percentage >= 33 {
            icons::BRIGHTNESS_MED
        } else {
            icons::BRIGHTNESS_LOW
        };

        let services = self.services.clone();
        let services_inc = self.services.clone();
        let services_dec = self.services.clone();

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
                false,
                move |cx| {
                    let services = services.clone();
                    cx.spawn(async move |_cx| {
                        let _ = services
                            .brightness
                            .dispatch(BrightnessCommand::SetPercent(50))
                            .await;
                    })
                    .detach();
                },
                move |cx| {
                    let services = services_inc.clone();
                    cx.spawn(async move |_cx| {
                        let _ = services
                            .brightness
                            .dispatch(BrightnessCommand::Increase(5))
                            .await;
                    })
                    .detach();
                },
                move |cx| {
                    let services = services_dec.clone();
                    cx.spawn(async move |_cx| {
                        let _ = services
                            .brightness
                            .dispatch(BrightnessCommand::Decrease(5))
                            .await;
                    })
                    .detach();
                },
            ))
            .into_any_element()
    }

    /// Render quick toggles (WiFi, Bluetooth).
    fn render_quick_toggles(&self) -> impl IntoElement {
        let network = self.services.network.get();
        let bluetooth = self.services.bluetooth.get();

        let wifi_enabled = network.wifi_enabled;
        let bt_state = bluetooth.state;
        let bt_active = bt_state == BluetoothState::Active;
        let bt_connected = bluetooth.devices.iter().any(|d| d.connected);

        let bt_icon = match bt_state {
            BluetoothState::Active if bt_connected => icons::BLUETOOTH_CONNECTED,
            BluetoothState::Active => icons::BLUETOOTH,
            _ => icons::BLUETOOTH_OFF,
        };

        let services_wifi = self.services.clone();
        let services_bt = self.services.clone();

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
                    .child(Self::render_toggle(
                        "wifi-toggle",
                        if wifi_enabled {
                            icons::WIFI
                        } else {
                            icons::WIFI_OFF
                        },
                        "WiFi",
                        wifi_enabled,
                        move |cx| {
                            let services = services_wifi.clone();
                            cx.spawn(async move |_cx| {
                                let _ = services.network.dispatch(NetworkCommand::ToggleWifi).await;
                            })
                            .detach();
                        },
                    ))
                    .child(Self::render_toggle(
                        "bt-toggle",
                        bt_icon,
                        "Bluetooth",
                        bt_active,
                        move |cx| {
                            let services = services_bt.clone();
                            cx.spawn(async move |_cx| {
                                let _ = services.bluetooth.dispatch(BluetoothCommand::Toggle).await;
                            })
                            .detach();
                        },
                    )),
            )
    }

    /// Render Bluetooth devices list.
    fn render_bluetooth_devices(&self) -> AnyElement {
        let bluetooth = self.services.bluetooth.get();

        if bluetooth.state != BluetoothState::Active || bluetooth.devices.is_empty() {
            return div().into_any_element();
        }

        let services = self.services.clone();

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
                        let services = services_clone.clone();
                        let path = path.clone();
                        cx.spawn(async move |_cx| {
                            if connected {
                                let _ = services
                                    .bluetooth
                                    .dispatch(BluetoothCommand::DisconnectDevice(path))
                                    .await;
                            } else {
                                let _ = services
                                    .bluetooth
                                    .dispatch(BluetoothCommand::ConnectDevice(path))
                                    .await;
                            }
                        })
                        .detach();
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

    /// Render battery and power profile section.
    fn render_power_section(&self) -> AnyElement {
        let upower = self.services.upower.get();
        let services = self.services.clone();

        let Some(battery) = &upower.battery else {
            // No battery, just show power profile if available
            if !upower.power_profiles_available {
                return div().into_any_element();
            }

            let profile = upower.power_profile;
            let profile_icon = profile.icon();
            let profile_label = profile.label();

            return div()
                .w_full()
                .p(px(spacing::MD))
                .bg(bg::secondary())
                .rounded(px(radius::MD))
                .flex()
                .flex_col()
                .gap(px(spacing::SM))
                .child(Self::render_section_header(icons::POWER_PROFILE, "Power"))
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
                                .child(div().text_size(px(icon_size::MD)).child(profile_icon))
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
                                    let services = services.clone();
                                    cx.spawn(async move |_cx| {
                                        let _ = services
                                            .upower
                                            .dispatch(UPowerCommand::CyclePowerProfile)
                                            .await;
                                    })
                                    .detach();
                                })
                                .child(
                                    div()
                                        .text_size(px(font_size::SM))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(profile_label),
                                ),
                        ),
                )
                .into_any_element();
        };

        let bat_icon = battery.icon();
        let bat_percentage = battery.percentage;
        let is_charging = battery.is_charging();
        let is_critical = battery.is_critical();

        let status_text = if battery.is_full() {
            "Fully Charged".to_string()
        } else if is_charging {
            match battery.time_remaining_str() {
                Some(time) => format!("Charging · {}", time),
                None => "Charging".to_string(),
            }
        } else {
            match battery.time_remaining_str() {
                Some(time) => format!("On Battery · {}", time),
                None => "On Battery".to_string(),
            }
        };

        let profile = upower.power_profile;
        let profile_icon = profile.icon();
        let profile_label = profile.label();
        let profiles_available = upower.power_profiles_available;

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
                            .text_color(if is_critical {
                                status::error()
                            } else if is_charging {
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
                                    .text_color(if is_critical {
                                        status::error()
                                    } else {
                                        text::primary()
                                    })
                                    .child(format!("{}%", bat_percentage)),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(text::muted())
                                    .child(status_text),
                            ),
                    ),
            )
            // Power profile selector (if available)
            .when(profiles_available, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(spacing::SM))
                                .child(div().text_size(px(icon_size::MD)).child(profile_icon))
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
                                    let services = services.clone();
                                    cx.spawn(async move |_cx| {
                                        let _ = services
                                            .upower
                                            .dispatch(UPowerCommand::CyclePowerProfile)
                                            .await;
                                    })
                                    .detach();
                                })
                                .child(
                                    div()
                                        .text_size(px(font_size::SM))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(profile_label),
                                ),
                        ),
                )
            })
            .into_any_element()
    }
}

impl Focusable for ControlCenter {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ControlCenter {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("control-center")
            .track_focus(&self.focus_handle)
            .key_context("ControlCenter")
            .w_full()
            .h_full()
            .p(px(spacing::MD))
            .bg(bg::primary())
            .border_1()
            .border_color(border::default())
            .rounded(px(radius::LG))
            .text_color(text::primary())
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
            .child(self.render_quick_toggles())
            // Bluetooth devices (if any)
            .child(self.render_bluetooth_devices())
            // Audio controls
            .child(self.render_audio_section())
            // Brightness (only on devices with backlight)
            .child(self.render_brightness_section())
            // Power/Battery section
            .child(self.render_power_section())
    }
}

// ============================================================================
// Launcher View Integration
// ============================================================================

use crate::launcher::view::{LauncherView, ViewContext as LauncherViewContext};

/// Control Center launcher view.
pub struct ControlCenterView;

impl LauncherView for ControlCenterView {
    fn prefix(&self) -> &'static str {
        "~"
    }

    fn name(&self) -> &'static str {
        "Control Center"
    }

    fn icon(&self) -> &'static str {
        icons::SETTINGS
    }

    fn description(&self) -> &'static str {
        "Quick settings and controls"
    }

    fn render(&self, vx: &LauncherViewContext, _cx: &App) -> (AnyElement, usize) {
        let audio = vx.services.audio.get();
        let network = vx.services.network.get();
        let bluetooth = vx.services.bluetooth.get();
        let brightness = vx.services.brightness.get();
        let upower = vx.services.upower.get();

        let query_lower = vx.query.to_lowercase();

        // Build list of control items
        let mut items: Vec<ControlItem> = Vec::new();

        // WiFi toggle
        items.push(ControlItem {
            id: "wifi",
            icon: if network.wifi_enabled {
                icons::WIFI
            } else {
                icons::WIFI_OFF
            },
            title: "WiFi".to_string(),
            subtitle: if network.wifi_enabled {
                "Enabled"
            } else {
                "Disabled"
            }
            .to_string(),
            active: network.wifi_enabled,
            action: ControlAction::ToggleWifi,
        });

        // Bluetooth toggle
        let bt_active = bluetooth.state == BluetoothState::Active;
        let bt_connected = bluetooth.devices.iter().any(|d| d.connected);
        items.push(ControlItem {
            id: "bluetooth",
            icon: match bluetooth.state {
                BluetoothState::Active if bt_connected => icons::BLUETOOTH_CONNECTED,
                BluetoothState::Active => icons::BLUETOOTH,
                _ => icons::BLUETOOTH_OFF,
            },
            title: "Bluetooth".to_string(),
            subtitle: if bt_connected {
                let count = bluetooth.devices.iter().filter(|d| d.connected).count();
                format!("{} connected", count)
            } else if bt_active {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            active: bt_active,
            action: ControlAction::ToggleBluetooth,
        });

        // Volume mute toggle
        items.push(ControlItem {
            id: "mute",
            icon: if audio.sink_muted {
                icons::VOLUME_MUTE
            } else {
                icons::VOLUME_HIGH
            },
            title: "Speaker".to_string(),
            subtitle: if audio.sink_muted {
                "Muted".to_string()
            } else {
                format!("{}%", audio.sink_volume)
            },
            active: !audio.sink_muted,
            action: ControlAction::ToggleMute,
        });

        // Mic mute toggle
        items.push(ControlItem {
            id: "mic",
            icon: if audio.source_muted {
                icons::MICROPHONE_MUTE
            } else {
                icons::MICROPHONE
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

        // Brightness (if available)
        if brightness.max > 0 {
            let pct = brightness.percentage();
            items.push(ControlItem {
                id: "brightness",
                icon: if pct >= 66 {
                    icons::BRIGHTNESS_HIGH
                } else if pct >= 33 {
                    icons::BRIGHTNESS_MED
                } else {
                    icons::BRIGHTNESS_LOW
                },
                title: "Brightness".to_string(),
                subtitle: format!("{}%", pct),
                active: true,
                action: ControlAction::CycleBrightness,
            });
        }

        // Power profile
        let profile = upower.power_profile;
        items.push(ControlItem {
            id: "power-profile",
            icon: profile.icon(),
            title: "Power Profile".to_string(),
            subtitle: profile.label().to_string(),
            active: profile == PowerProfile::Performance,
            action: ControlAction::CyclePowerProfile,
        });

        // Battery info (if present)
        if let Some(battery) = &upower.battery {
            items.push(ControlItem {
                id: "battery",
                icon: battery.icon(),
                title: format!("Battery {}%", battery.percentage),
                subtitle: if battery.is_charging() {
                    "Charging".to_string()
                } else if battery.is_full() {
                    "Full".to_string()
                } else {
                    battery
                        .time_remaining_str()
                        .unwrap_or_else(|| "On Battery".to_string())
                },
                active: false,
                action: ControlAction::None,
            });
        }

        // Filter by query
        let filtered: Vec<_> = items
            .into_iter()
            .filter(|item| {
                if query_lower.is_empty() {
                    true
                } else {
                    item.title.to_lowercase().contains(&query_lower)
                        || item.subtitle.to_lowercase().contains(&query_lower)
                }
            })
            .collect();

        let count = filtered.len();
        let services = vx.services.clone();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(spacing::SM))
            .children(filtered.into_iter().enumerate().map(|(i, item)| {
                let is_selected = i == vx.selected_index;
                let services = services.clone();
                let action = item.action.clone();

                div()
                    .id(format!("cc-{}", item.id))
                    .w_full()
                    .h(px(56.))
                    .px(px(spacing::MD))
                    .rounded(px(radius::MD))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(spacing::MD))
                    .when(is_selected, |el| el.bg(rgba(0x3b82f6ff)))
                    .when(!is_selected, |el| el.hover(|s| s.bg(interactive::hover())))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        execute_action(&services, &action, cx);
                    })
                    // Icon
                    .child(
                        div()
                            .w(px(40.))
                            .h(px(40.))
                            .rounded(px(radius::MD))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(if item.active {
                                accent::primary()
                            } else {
                                interactive::default()
                            })
                            .child(
                                div()
                                    .text_size(px(icon_size::LG))
                                    .text_color(text::primary())
                                    .child(item.icon),
                            ),
                    )
                    // Text
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(font_size::BASE))
                                    .text_color(text::primary())
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(item.title),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(text::muted())
                                    .child(item.subtitle),
                            ),
                    )
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &LauncherViewContext, cx: &mut App) -> bool {
        let audio = vx.services.audio.get();
        let network = vx.services.network.get();
        let bluetooth = vx.services.bluetooth.get();
        let brightness = vx.services.brightness.get();
        let upower = vx.services.upower.get();

        let query_lower = vx.query.to_lowercase();

        // Rebuild items list to find the selected one
        let mut items: Vec<ControlAction> = Vec::new();

        // WiFi
        if query_lower.is_empty() || "wifi".contains(&query_lower) {
            items.push(ControlAction::ToggleWifi);
        }

        // Bluetooth
        if query_lower.is_empty()
            || "bluetooth".contains(&query_lower)
            || "bt".contains(&query_lower)
        {
            items.push(ControlAction::ToggleBluetooth);
        }

        // Mute
        if query_lower.is_empty()
            || "speaker".contains(&query_lower)
            || "mute".contains(&query_lower)
            || "volume".contains(&query_lower)
        {
            items.push(ControlAction::ToggleMute);
        }

        // Mic
        if query_lower.is_empty()
            || "microphone".contains(&query_lower)
            || "mic".contains(&query_lower)
        {
            items.push(ControlAction::ToggleMicMute);
        }

        // Brightness
        if brightness.max > 0 && (query_lower.is_empty() || "brightness".contains(&query_lower)) {
            items.push(ControlAction::CycleBrightness);
        }

        // Power profile
        if query_lower.is_empty()
            || "power".contains(&query_lower)
            || "profile".contains(&query_lower)
        {
            items.push(ControlAction::CyclePowerProfile);
        }

        // Battery (no action)
        if upower.battery.is_some() && (query_lower.is_empty() || "battery".contains(&query_lower))
        {
            items.push(ControlAction::None);
        }

        if let Some(action) = items.get(index) {
            execute_action(vx.services, action, cx);
        }

        false // Don't close launcher
    }

    fn footer_actions(&self, _vx: &LauncherViewContext) -> Vec<(&'static str, &'static str)> {
        vec![("Toggle", "Enter"), ("Close", "Esc")]
    }
}

/// A control item for the launcher view.
struct ControlItem {
    id: &'static str,
    icon: &'static str,
    title: String,
    subtitle: String,
    active: bool,
    action: ControlAction,
}

/// Actions that can be performed from the control center.
#[derive(Clone)]
enum ControlAction {
    None,
    ToggleWifi,
    ToggleBluetooth,
    ToggleMute,
    ToggleMicMute,
    CycleBrightness,
    CyclePowerProfile,
}

/// Execute a control action.
fn execute_action(services: &Services, action: &ControlAction, cx: &mut App) {
    let services = services.clone();
    let action = action.clone();

    match action {
        ControlAction::None => {}
        ControlAction::ToggleWifi => {
            cx.spawn(async move |_cx| {
                let _ = services.network.dispatch(NetworkCommand::ToggleWifi).await;
            })
            .detach();
        }
        ControlAction::ToggleBluetooth => {
            cx.spawn(async move |_cx| {
                let _ = services.bluetooth.dispatch(BluetoothCommand::Toggle).await;
            })
            .detach();
        }
        ControlAction::ToggleMute => {
            services.audio.dispatch(AudioCommand::ToggleSinkMute);
        }
        ControlAction::ToggleMicMute => {
            services.audio.dispatch(AudioCommand::ToggleSourceMute);
        }
        ControlAction::CycleBrightness => {
            // Cycle through 25%, 50%, 75%, 100%
            let current = services.brightness.get().percentage();
            let next = match current {
                0..=25 => 50,
                26..=50 => 75,
                51..=75 => 100,
                _ => 25,
            };
            cx.spawn(async move |_cx| {
                let _ = services
                    .brightness
                    .dispatch(BrightnessCommand::SetPercent(next))
                    .await;
            })
            .detach();
        }
        ControlAction::CyclePowerProfile => {
            cx.spawn(async move |_cx| {
                let _ = services
                    .upower
                    .dispatch(UPowerCommand::CyclePowerProfile)
                    .await;
            })
            .detach();
        }
    }
}
