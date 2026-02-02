//! Minimal Control Center component with expandable sections.
//!
//! This component is shared between:
//! - The panel (opened from Settings widget click)
//! - The launcher view (`~` prefix)
//!
//! Features:
//! - Quick toggles row (WiFi, Bluetooth, Mic)
//! - Volume slider
//! - Brightness slider (if available)
//! - Expandable WiFi section with available networks
//! - Expandable Bluetooth section with paired devices
//! - Battery status and power profile

use futures_signals::signal::SignalExt;
use gpui::{
    AnyElement, App, Context, FocusHandle, Focusable, FontWeight, MouseButton, ScrollHandle,
    Window, div, prelude::*, px,
};
use services::{
    AccessPoint, AudioCommand, BluetoothCommand, BluetoothDevice, BluetoothState,
    BrightnessCommand, NetworkCommand, PowerProfile, Services, UPowerCommand,
};
use ui::{accent, bg, border, font_size, icon_size, interactive, radius, spacing, status, text};
use zbus::zvariant::OwnedObjectPath;

/// Nerd Font icons.
pub mod icons {
    // Audio
    pub const VOLUME_HIGH: &str = "󰕾";
    pub const VOLUME_MED: &str = "󰖀";
    pub const VOLUME_LOW: &str = "󰕿";
    pub const VOLUME_MUTE: &str = "󰝟";
    pub const MICROPHONE: &str = "󰍬";
    pub const MICROPHONE_MUTE: &str = "󰍭";

    // Brightness
    pub const BRIGHTNESS: &str = "󰃟";

    // Connectivity
    pub const BLUETOOTH: &str = "󰂯";
    pub const BLUETOOTH_OFF: &str = "󰂲";
    pub const BLUETOOTH_CONNECTED: &str = "󰂱";
    pub const WIFI: &str = "󰤨";
    pub const WIFI_OFF: &str = "󰤭";

    // Power
    pub const BATTERY_CHARGING: &str = "󰂄";
    pub const POWER_PROFILE: &str = "󰌪";

    // UI
    pub const CHEVRON_DOWN: &str = "󰅀";
    pub const CHEVRON_UP: &str = "󰅃";
    pub const CHECK: &str = "󰄬";
    pub const SETTINGS: &str = "";
}

/// Which section is currently expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpandedSection {
    #[default]
    None,
    WiFi,
    Bluetooth,
}

/// Control Center panel component.
pub struct ControlCenter {
    services: Services,
    expanded: ExpandedSection,
    scroll_handle: ScrollHandle,
    focus_handle: FocusHandle,
}

impl ControlCenter {
    /// Create a new control center.
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();

        // Subscribe to service updates
        Self::subscribe_to_services(&services, cx);

        ControlCenter {
            services,
            expanded: ExpandedSection::None,
            scroll_handle,
            focus_handle,
        }
    }

    fn subscribe_to_services(services: &Services, cx: &mut Context<Self>) {
        // Audio
        cx.spawn({
            let mut signal = services.audio.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        // Bluetooth
        cx.spawn({
            let mut signal = services.bluetooth.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        // Brightness
        cx.spawn({
            let mut signal = services.brightness.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        // Network
        cx.spawn({
            let mut signal = services.network.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        // UPower
        cx.spawn({
            let mut signal = services.upower.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();
    }

    fn toggle_section(&mut self, section: ExpandedSection) {
        if self.expanded == section {
            self.expanded = ExpandedSection::None;
        } else {
            self.expanded = section;
        }
    }

    // ========================================================================
    // Quick Toggles Row
    // ========================================================================

    fn render_quick_toggles(&self, cx: &Context<Self>) -> impl IntoElement {
        let audio = self.services.audio.get();
        let network = self.services.network.get();
        let bluetooth = self.services.bluetooth.get();

        let wifi_enabled = network.wifi_enabled;
        let bt_active = bluetooth.state == BluetoothState::Active;
        let mic_muted = audio.source_muted;

        let services_wifi = self.services.clone();
        let services_bt = self.services.clone();
        let services_mic = self.services.clone();

        div()
            .flex()
            .gap(px(spacing::SM))
            // WiFi toggle
            .child(self.render_quick_toggle_btn(
                "wifi-toggle",
                if wifi_enabled {
                    icons::WIFI
                } else {
                    icons::WIFI_OFF
                },
                wifi_enabled,
                ExpandedSection::WiFi,
                move |cx| {
                    let services = services_wifi.clone();
                    cx.spawn(async move |_| {
                        let _ = services.network.dispatch(NetworkCommand::ToggleWifi).await;
                    })
                    .detach();
                },
                cx,
            ))
            // Bluetooth toggle
            .child(self.render_quick_toggle_btn(
                "bt-toggle",
                if bt_active {
                    icons::BLUETOOTH
                } else {
                    icons::BLUETOOTH_OFF
                },
                bt_active,
                ExpandedSection::Bluetooth,
                move |cx| {
                    let services = services_bt.clone();
                    cx.spawn(async move |_| {
                        let _ = services.bluetooth.dispatch(BluetoothCommand::Toggle).await;
                    })
                    .detach();
                },
                cx,
            ))
            // Mic mute toggle
            .child(self.render_icon_toggle(
                "mic-toggle",
                if mic_muted {
                    icons::MICROPHONE_MUTE
                } else {
                    icons::MICROPHONE
                },
                !mic_muted,
                move |_cx| {
                    services_mic.audio.dispatch(AudioCommand::ToggleSourceMute);
                },
            ))
    }

    /// Quick toggle button with expand arrow.
    fn render_quick_toggle_btn(
        &self,
        id: &str,
        icon: &'static str,
        active: bool,
        section: ExpandedSection,
        on_toggle: impl Fn(&mut App) + 'static,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let is_expanded = self.expanded == section;
        let chevron = if is_expanded {
            icons::CHEVRON_UP
        } else {
            icons::CHEVRON_DOWN
        };

        div()
            .flex()
            .items_center()
            .rounded(px(radius::MD))
            .overflow_hidden()
            .child(
                // Main toggle button
                div()
                    .id(format!("{}-main", id))
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(40.))
                    .h(px(36.))
                    .cursor_pointer()
                    .when(active, |el| el.bg(accent::primary()))
                    .when(!active, |el| el.bg(interactive::default()))
                    .hover(|s| s.bg(interactive::hover()))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        on_toggle(cx);
                    })
                    .child(
                        div()
                            .text_size(px(icon_size::MD))
                            .text_color(text::primary())
                            .child(icon),
                    ),
            )
            .child(
                // Expand arrow
                div()
                    .id(format!("{}-expand", id))
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(24.))
                    .h(px(36.))
                    .cursor_pointer()
                    .when(active, |el| el.bg(accent::hover()))
                    .when(!active, |el| el.bg(interactive::default()))
                    .hover(|s| s.bg(interactive::hover()))
                    .border_l_1()
                    .border_color(border::subtle())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.toggle_section(section);
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(text::muted())
                            .child(chevron),
                    ),
            )
    }

    /// Simple icon toggle without expand.
    fn render_icon_toggle(
        &self,
        id: impl Into<gpui::ElementId>,
        icon: &'static str,
        active: bool,
        on_click: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id.into())
            .flex()
            .items_center()
            .justify_center()
            .w(px(40.))
            .h(px(36.))
            .rounded(px(radius::MD))
            .cursor_pointer()
            .when(active, |el| el.bg(accent::primary()))
            .when(!active, |el| el.bg(interactive::default()))
            .hover(|s| s.bg(interactive::hover()))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_click(cx);
            })
            .child(
                div()
                    .text_size(px(icon_size::MD))
                    .text_color(text::primary())
                    .child(icon),
            )
    }

    // ========================================================================
    // Sliders
    // ========================================================================

    fn render_volume_slider(&self) -> impl IntoElement {
        let audio = self.services.audio.get();
        let volume = audio.sink_volume;
        let muted = audio.sink_muted;

        let icon = if muted || volume == 0 {
            icons::VOLUME_MUTE
        } else if volume >= 66 {
            icons::VOLUME_HIGH
        } else if volume >= 33 {
            icons::VOLUME_MED
        } else {
            icons::VOLUME_LOW
        };

        let services = self.services.clone();
        let services_inc = self.services.clone();
        let services_dec = self.services.clone();

        Self::render_slider(
            "volume",
            icon,
            volume,
            muted,
            move |_| services.audio.dispatch(AudioCommand::ToggleSinkMute),
            move |_| {
                services_inc
                    .audio
                    .dispatch(AudioCommand::AdjustSinkVolume(5))
            },
            move |_| {
                services_dec
                    .audio
                    .dispatch(AudioCommand::AdjustSinkVolume(-5))
            },
        )
    }

    fn render_brightness_slider(&self) -> AnyElement {
        let brightness = self.services.brightness.get();

        if brightness.max == 0 {
            return div().into_any_element();
        }

        let percent = brightness.percentage();
        let services = self.services.clone();
        let services_inc = self.services.clone();
        let services_dec = self.services.clone();

        Self::render_slider(
            "brightness",
            icons::BRIGHTNESS,
            percent,
            false,
            move |cx| {
                let s = services.clone();
                cx.spawn(async move |_| {
                    let _ = s
                        .brightness
                        .dispatch(BrightnessCommand::SetPercent(50))
                        .await;
                })
                .detach();
            },
            move |cx| {
                let s = services_inc.clone();
                cx.spawn(async move |_| {
                    let _ = s.brightness.dispatch(BrightnessCommand::Increase(5)).await;
                })
                .detach();
            },
            move |cx| {
                let s = services_dec.clone();
                cx.spawn(async move |_| {
                    let _ = s.brightness.dispatch(BrightnessCommand::Decrease(5)).await;
                })
                .detach();
            },
        )
        .into_any_element()
    }

    fn render_slider(
        id: &'static str,
        icon: &'static str,
        value: u8,
        muted: bool,
        on_icon_click: impl Fn(&mut App) + 'static,
        on_increase: impl Fn(&mut App) + 'static,
        on_decrease: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        let percent = value.min(100);
        let width_frac = percent as f32 / 100.0;
        let id_icon = format!("{}-icon", id);
        let id_dec = format!("{}-dec", id);
        let id_inc = format!("{}-inc", id);

        div()
            .flex()
            .items_center()
            .gap(px(spacing::SM))
            .w_full()
            // Icon
            .child(
                div()
                    .id(id_icon)
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(radius::SM))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .bg(interactive::default())
                    .hover(|s| s.bg(interactive::hover()))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| on_icon_click(cx))
                    .child(
                        div()
                            .text_size(px(icon_size::SM))
                            .text_color(if muted {
                                status::error()
                            } else {
                                text::primary()
                            })
                            .child(icon),
                    ),
            )
            // Track
            .child(
                div()
                    .id(id)
                    .flex_1()
                    .h(px(6.))
                    .bg(bg::tertiary())
                    .rounded(px(3.))
                    .overflow_hidden()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(gpui::relative(width_frac))
                            .bg(if muted {
                                text::disabled()
                            } else {
                                accent::primary()
                            })
                            .rounded(px(3.)),
                    ),
            )
            // Percent
            .child(
                div()
                    .w(px(32.))
                    .text_size(px(font_size::XS))
                    .text_color(text::muted())
                    .text_right()
                    .child(format!("{}%", percent)),
            )
            // +/- buttons
            .child(
                div()
                    .flex()
                    .gap(px(2.))
                    .child(
                        div()
                            .id(id_dec)
                            .w(px(20.))
                            .h(px(20.))
                            .rounded(px(radius::SM))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(interactive::default())
                            .hover(|s| s.bg(interactive::hover()))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| on_decrease(cx))
                            .child(
                                div()
                                    .text_size(px(font_size::XS))
                                    .text_color(text::muted())
                                    .child("−"),
                            ),
                    )
                    .child(
                        div()
                            .id(id_inc)
                            .w(px(20.))
                            .h(px(20.))
                            .rounded(px(radius::SM))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(interactive::default())
                            .hover(|s| s.bg(interactive::hover()))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| on_increase(cx))
                            .child(
                                div()
                                    .text_size(px(font_size::XS))
                                    .text_color(text::muted())
                                    .child("+"),
                            ),
                    ),
            )
    }

    // ========================================================================
    // Expandable Sections
    // ========================================================================

    fn render_wifi_section(&self) -> AnyElement {
        if self.expanded != ExpandedSection::WiFi {
            return div().into_any_element();
        }

        let network = self.services.network.get();
        let services = self.services.clone();

        // Get current connection name
        let connected_name: Option<String> = network.active_connections.iter().find_map(|c| {
            if let services::ActiveConnectionInfo::WiFi { name, .. } = c {
                Some(name.clone())
            } else {
                None
            }
        });

        // Sort access points: connected first, then by signal strength
        let mut aps: Vec<AccessPoint> = network.wireless_access_points.clone();
        aps.sort_by(|a, b| {
            let a_connected = connected_name.as_ref() == Some(&a.ssid);
            let b_connected = connected_name.as_ref() == Some(&b.ssid);
            match (a_connected, b_connected) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.strength.cmp(&a.strength),
            }
        });
        let aps_empty = aps.is_empty();

        div()
            .w_full()
            .p(px(spacing::SM))
            .bg(bg::secondary())
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(2.))
            .children(aps.into_iter().take(8).map(|ap| {
                let ssid = ap.ssid.clone();
                let is_connected = connected_name.as_ref() == Some(&ssid);
                let strength = ap.strength;
                let device_path: OwnedObjectPath = ap.device_path.clone().into();
                let ap_path: OwnedObjectPath = ap.path.clone().into();
                let services_clone = services.clone();

                render_wifi_item(ssid, strength, is_connected, move |cx| {
                    if !is_connected {
                        let s = services_clone.clone();
                        let dev = device_path.clone();
                        let path = ap_path.clone();
                        cx.spawn(async move |_| {
                            let _ = s
                                .network
                                .dispatch(NetworkCommand::ConnectToAccessPoint {
                                    device_path: dev,
                                    ap_path: path,
                                    password: None,
                                })
                                .await;
                        })
                        .detach();
                    }
                })
            }))
            .when(aps_empty, |el| {
                el.child(
                    div()
                        .py(px(spacing::SM))
                        .text_size(px(font_size::SM))
                        .text_color(text::muted())
                        .child("No networks found"),
                )
            })
            .into_any_element()
    }
}

fn render_wifi_item(
    ssid: String,
    strength: u8,
    connected: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let wifi_icon = match strength {
        0..=25 => "󰤟",
        26..=50 => "󰤢",
        51..=75 => "󰤥",
        _ => "󰤨",
    };
    let id = format!("wifi-{}", ssid);

    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .px(px(spacing::SM))
        .py(px(spacing::XS))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .hover(|s| s.bg(interactive::hover()))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_click(cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(if connected {
                            accent::primary()
                        } else {
                            text::muted()
                        })
                        .child(wifi_icon),
                )
                .child(
                    div()
                        .text_size(px(font_size::SM))
                        .text_color(if connected {
                            text::primary()
                        } else {
                            text::secondary()
                        })
                        .child(ssid),
                ),
        )
        .when(connected, |el| {
            el.child(
                div()
                    .text_size(px(icon_size::SM))
                    .text_color(status::success())
                    .child(icons::CHECK),
            )
        })
}

impl ControlCenter {
    fn render_bluetooth_section(&self) -> AnyElement {
        if self.expanded != ExpandedSection::Bluetooth {
            return div().into_any_element();
        }

        let bluetooth = self.services.bluetooth.get();
        let services = self.services.clone();

        if bluetooth.devices.is_empty() {
            return div()
                .w_full()
                .p(px(spacing::SM))
                .bg(bg::secondary())
                .rounded(px(radius::MD))
                .child(
                    div()
                        .py(px(spacing::SM))
                        .text_size(px(font_size::SM))
                        .text_color(text::muted())
                        .child("No paired devices"),
                )
                .into_any_element();
        }

        // Sort: connected first
        let mut devices: Vec<BluetoothDevice> = bluetooth.devices.clone();
        devices.sort_by(|a, b| b.connected.cmp(&a.connected));

        div()
            .w_full()
            .p(px(spacing::SM))
            .bg(bg::secondary())
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(2.))
            .children(devices.into_iter().take(6).map(|device| {
                let name = device.name.clone();
                let path = device.path.clone();
                let connected = device.connected;
                let battery = device.battery;
                let services_clone = services.clone();

                render_bluetooth_item(name, connected, battery, move |cx| {
                    let s = services_clone.clone();
                    let p = path.clone();
                    cx.spawn(async move |_| {
                        if connected {
                            let _ = s
                                .bluetooth
                                .dispatch(BluetoothCommand::DisconnectDevice(p))
                                .await;
                        } else {
                            let _ = s
                                .bluetooth
                                .dispatch(BluetoothCommand::ConnectDevice(p))
                                .await;
                        }
                    })
                    .detach();
                })
            }))
            .into_any_element()
    }
}

fn render_bluetooth_item(
    name: String,
    connected: bool,
    battery: Option<u8>,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let icon = if connected {
        icons::BLUETOOTH_CONNECTED
    } else {
        icons::BLUETOOTH
    };
    let id = format!("bt-{}", name);

    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .px(px(spacing::SM))
        .py(px(spacing::XS))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .hover(|s| s.bg(interactive::hover()))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_click(cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(if connected {
                            accent::primary()
                        } else {
                            text::muted()
                        })
                        .child(icon),
                )
                .child(
                    div()
                        .text_size(px(font_size::SM))
                        .text_color(if connected {
                            text::primary()
                        } else {
                            text::secondary()
                        })
                        .child(name),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .when_some(battery, |el, bat| {
                    el.child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(text::muted())
                            .child(format!("{}%", bat)),
                    )
                })
                .when(connected, |el| {
                    el.child(
                        div()
                            .text_size(px(icon_size::SM))
                            .text_color(status::success())
                            .child(icons::CHECK),
                    )
                }),
        )
}

impl ControlCenter {
    // ========================================================================
    // Power Section
    // ========================================================================

    fn render_power_section(&self) -> AnyElement {
        let upower = self.services.upower.get();
        let services = self.services.clone();

        let profile = upower.power_profile;
        let profiles_available = upower.power_profiles_available;

        match &upower.battery {
            Some(battery) => {
                let icon = battery.icon();
                let percent = battery.percentage;
                let is_charging = battery.is_charging();
                let is_critical = battery.is_critical();

                let status_text = if battery.is_full() {
                    "Full".to_string()
                } else if is_charging {
                    battery
                        .time_remaining_str()
                        .map_or("Charging".to_string(), |t| format!("{} to full", t))
                } else {
                    battery
                        .time_remaining_str()
                        .map_or("On battery".to_string(), |t| t)
                };

                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(icon_size::LG))
                                    .text_color(if is_critical {
                                        status::error()
                                    } else if is_charging {
                                        status::success()
                                    } else {
                                        text::primary()
                                    })
                                    .child(icon),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(font_size::MD))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(if is_critical {
                                                status::error()
                                            } else {
                                                text::primary()
                                            })
                                            .child(format!("{}%", percent)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(font_size::XS))
                                            .text_color(text::muted())
                                            .child(status_text),
                                    ),
                            ),
                    )
                    .when(profiles_available, |el| {
                        el.child(self.render_power_profile_btn(profile, services))
                    })
                    .into_any_element()
            }
            None => {
                // No battery - just show power profile if available
                if profiles_available {
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .child(
                            div()
                                .text_size(px(font_size::SM))
                                .text_color(text::secondary())
                                .child("Power Profile"),
                        )
                        .child(self.render_power_profile_btn(profile, services))
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            }
        }
    }

    fn render_power_profile_btn(
        &self,
        profile: PowerProfile,
        services: Services,
    ) -> impl IntoElement {
        let icon = profile.icon();
        let label = profile.label();

        div()
            .id("power-profile")
            .flex()
            .items_center()
            .gap(px(spacing::XS))
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .bg(interactive::default())
            .hover(|s| s.bg(interactive::hover()))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                let s = services.clone();
                cx.spawn(async move |_| {
                    let _ = s.upower.dispatch(UPowerCommand::CyclePowerProfile).await;
                })
                .detach();
            })
            .child(
                div()
                    .text_size(px(icon_size::SM))
                    .text_color(text::secondary())
                    .child(icon),
            )
            .child(
                div()
                    .text_size(px(font_size::XS))
                    .text_color(text::secondary())
                    .child(label),
            )
    }
}

impl Focusable for ControlCenter {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ControlCenter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            // Quick toggles row
            .child(self.render_quick_toggles(cx))
            // Volume slider
            .child(self.render_volume_slider())
            // Brightness slider (if available)
            .child(self.render_brightness_slider())
            // WiFi expanded section
            .child(self.render_wifi_section())
            // Bluetooth expanded section
            .child(self.render_bluetooth_section())
            // Power/Battery section
            .child(self.render_power_section())
    }
}

// ============================================================================
// Launcher View Integration
// ============================================================================

use crate::launcher::view::{LauncherView, ViewContext as LauncherViewContext};

/// Control Center as a launcher view.
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
        "Quick settings and system controls"
    }

    fn render(&self, vx: &LauncherViewContext, _cx: &App) -> (AnyElement, usize) {
        // Render a simplified inline version for the launcher
        let audio = vx.services.audio.get();
        let network = vx.services.network.get();
        let bluetooth = vx.services.bluetooth.get();
        let upower = vx.services.upower.get();

        let items = vec![
            (
                "WiFi",
                network.wifi_enabled,
                if network.wifi_enabled {
                    icons::WIFI
                } else {
                    icons::WIFI_OFF
                },
            ),
            (
                "Bluetooth",
                bluetooth.state == BluetoothState::Active,
                if bluetooth.state == BluetoothState::Active {
                    icons::BLUETOOTH
                } else {
                    icons::BLUETOOTH_OFF
                },
            ),
            (
                "Microphone",
                !audio.source_muted,
                if audio.source_muted {
                    icons::MICROPHONE_MUTE
                } else {
                    icons::MICROPHONE
                },
            ),
        ];

        let count = items.len() + 2; // +2 for volume and battery

        let el = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            // Status items
            .children(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(idx, (name, active, icon))| {
                        div()
                            .id(format!("cc-item-{}", idx))
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .px(px(spacing::MD))
                            .py(px(spacing::SM))
                            .rounded(px(radius::MD))
                            .when(vx.selected_index == idx, |el| el.bg(interactive::hover()))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::SM))
                                    .child(
                                        div()
                                            .text_size(px(icon_size::MD))
                                            .text_color(if active {
                                                accent::primary()
                                            } else {
                                                text::muted()
                                            })
                                            .child(icon),
                                    )
                                    .child(div().text_size(px(font_size::SM)).child(name)),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size::XS))
                                    .text_color(text::muted())
                                    .child(if active { "On" } else { "Off" }),
                            )
                    }),
            )
            // Volume row
            .child(
                div()
                    .id("cc-volume")
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .px(px(spacing::MD))
                    .py(px(spacing::SM))
                    .rounded(px(radius::MD))
                    .when(vx.selected_index == 3, |el| el.bg(interactive::hover()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(icon_size::MD))
                                    .text_color(if audio.sink_muted {
                                        status::error()
                                    } else {
                                        text::primary()
                                    })
                                    .child(if audio.sink_muted {
                                        icons::VOLUME_MUTE
                                    } else {
                                        icons::VOLUME_HIGH
                                    }),
                            )
                            .child(div().text_size(px(font_size::SM)).child("Volume")),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(text::muted())
                            .child(format!("{}%", audio.sink_volume)),
                    ),
            )
            // Battery row
            .when_some(upower.battery.as_ref(), |el, battery| {
                el.child(
                    div()
                        .id("cc-battery")
                        .flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .px(px(spacing::MD))
                        .py(px(spacing::SM))
                        .rounded(px(radius::MD))
                        .when(vx.selected_index == 4, |el| el.bg(interactive::hover()))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(spacing::SM))
                                .child(
                                    div()
                                        .text_size(px(icon_size::MD))
                                        .text_color(if battery.is_critical() {
                                            status::error()
                                        } else if battery.is_charging() {
                                            status::success()
                                        } else {
                                            text::primary()
                                        })
                                        .child(battery.icon()),
                                )
                                .child(div().text_size(px(font_size::SM)).child("Battery")),
                        )
                        .child(
                            div()
                                .text_size(px(font_size::XS))
                                .text_color(text::muted())
                                .child(format!("{}%", battery.percentage)),
                        ),
                )
            })
            .into_any_element();

        (el, count)
    }

    fn on_select(&self, index: usize, vx: &LauncherViewContext, cx: &mut App) -> bool {
        let services = vx.services.clone();

        match index {
            0 => {
                // Toggle WiFi
                cx.spawn(async move |_| {
                    let _ = services.network.dispatch(NetworkCommand::ToggleWifi).await;
                })
                .detach();
            }
            1 => {
                // Toggle Bluetooth
                cx.spawn(async move |_| {
                    let _ = services.bluetooth.dispatch(BluetoothCommand::Toggle).await;
                })
                .detach();
            }
            2 => {
                // Toggle Mic
                services.audio.dispatch(AudioCommand::ToggleSourceMute);
            }
            3 => {
                // Toggle Volume mute
                services.audio.dispatch(AudioCommand::ToggleSinkMute);
            }
            _ => {}
        }

        false // Don't close launcher
    }

    fn footer_actions(&self, _vx: &LauncherViewContext) -> Vec<(&'static str, &'static str)> {
        vec![("Enter", "Toggle"), ("Esc", "Close")]
    }
}
