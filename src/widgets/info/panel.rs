use crate::services::Services;
use crate::services::network::NetworkCommand;
use gpui::{Context, FocusHandle, Focusable, MouseButton, Window, div, prelude::*, px, rgba};
use std::time::Duration;

/// Info panel showing detailed battery, volume, and network settings.
pub struct InfoPanel {
    services: Services,
    battery_percent: Option<u8>,
    battery_charging: bool,
    battery_time_to_full: Option<String>,
    battery_time_to_empty: Option<String>,
    volume_percent: u8,
    volume_muted: bool,
    wifi_enabled: bool,
    focus_handle: FocusHandle,
}

impl InfoPanel {
    pub fn with_services(services: Services, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Observe network for updates
        cx.observe(&services.network, |this, network, cx| {
            this.wifi_enabled = network.read(cx).wifi_enabled;
            cx.notify();
        })
        .detach();

        // Poll battery and volume
        cx.spawn(async move |this, cx| {
            loop {
                let battery = read_battery_info();
                let (volume_percent, volume_muted) = super::read_volume_status();

                let _ = this.update(cx, |this, cx| {
                    this.battery_percent = battery.percent;
                    this.battery_charging = battery.charging;
                    this.battery_time_to_full = battery.time_to_full;
                    this.battery_time_to_empty = battery.time_to_empty;
                    this.volume_percent = volume_percent;
                    this.volume_muted = volume_muted;
                    cx.notify();
                });

                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        })
        .detach();

        let battery = read_battery_info();
        let (volume_percent, volume_muted) = super::read_volume_status();
        let wifi_enabled = services.network.read(cx).wifi_enabled;

        InfoPanel {
            services,
            battery_percent: battery.percent,
            battery_charging: battery.charging,
            battery_time_to_full: battery.time_to_full,
            battery_time_to_empty: battery.time_to_empty,
            volume_percent,
            volume_muted,
            wifi_enabled,
            focus_handle,
        }
    }

    fn toggle_wifi(&mut self, cx: &mut Context<Self>) {
        self.services.network.update(cx, |network, cx| {
            network.dispatch(NetworkCommand::ToggleWiFi, cx);
        });
    }

    fn toggle_mute(&mut self) {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            let _ = Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                .spawn();
        }
    }

    fn set_volume(&mut self, percent: u8) {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            let _ = Command::new("wpctl")
                .args([
                    "set-volume",
                    "@DEFAULT_AUDIO_SINK@",
                    &format!("{}%", percent),
                ])
                .spawn();
        }
        self.volume_percent = percent;
    }
}

impl Focusable for InfoPanel {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InfoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let wifi_enabled = self.wifi_enabled;

        div()
            .id("info-panel")
            .track_focus(&self.focus_handle)
            .key_context("InfoPanel")
            .size_full()
            .bg(rgba(0x1a1a1aee))
            .border_1()
            .border_color(rgba(0x333333ff))
            .rounded(px(12.))
            .p(px(16.))
            .text_color(rgba(0xffffffff))
            .flex()
            .flex_col()
            .gap(px(16.))
            // Battery section
            .child(self.render_battery_section())
            // Volume section
            .child(self.render_volume_section(cx))
            // Divider
            .child(div().h(px(1.)).w_full().bg(rgba(0x333333ff)))
            // Quick toggles
            .child(self.render_quick_toggles(wifi_enabled, cx))
    }
}

impl InfoPanel {
    fn render_battery_section(&self) -> impl IntoElement {
        let battery_icon = if self.battery_charging {
            "󰂄"
        } else {
            match self.battery_percent {
                Some(p) if p >= 90 => "󰁹",
                Some(p) if p >= 70 => "󰂀",
                Some(p) if p >= 50 => "󰁾",
                Some(p) if p >= 30 => "󰁼",
                Some(p) if p >= 10 => "󰁺",
                Some(_) => "󰂃",
                None => "󰂑",
            }
        };

        let battery_text = self
            .battery_percent
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "N/A".to_string());

        let time_text = if self.battery_charging {
            self.battery_time_to_full
                .as_ref()
                .map(|t| format!("Full in {}", t))
        } else {
            self.battery_time_to_empty
                .as_ref()
                .map(|t| format!("{} remaining", t))
        };

        let battery_color = match self.battery_percent {
            Some(p) if p >= 50 => rgba(0x22c55eff), // green
            Some(p) if p >= 20 => rgba(0xeab308ff), // yellow
            Some(_) => rgba(0xef4444ff),            // red
            None => rgba(0x888888ff),               // gray
        };

        div().flex().flex_col().gap(px(8.)).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(24.))
                                .text_color(battery_color)
                                .child(battery_icon),
                        )
                        .child(
                            div()
                                .text_size(px(18.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(battery_text),
                        ),
                )
                .when_some(time_text, |el, text| {
                    el.child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgba(0x888888ff))
                            .child(text),
                    )
                }),
        )
    }

    fn render_volume_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let volume_icon = if self.volume_muted {
            "󰝟"
        } else if self.volume_percent >= 70 {
            "󰕾"
        } else if self.volume_percent >= 30 {
            "󰖀"
        } else {
            "󰕿"
        };

        let current = self.volume_percent;
        let muted = self.volume_muted;

        div().flex().flex_col().gap(px(8.)).child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                // Mute button
                .child(
                    div()
                        .id("mute-btn")
                        .text_size(px(20.))
                        .cursor_pointer()
                        .px(px(8.))
                        .py(px(4.))
                        .rounded(px(4.))
                        .hover(|s| s.bg(rgba(0x333333ff)))
                        .text_color(if muted {
                            rgba(0x666666ff)
                        } else {
                            rgba(0xffffffff)
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, _| this.toggle_mute()),
                        )
                        .child(volume_icon),
                )
                // Volume slider
                .child(
                    div()
                        .flex_1()
                        .h(px(8.))
                        .bg(rgba(0x333333ff))
                        .rounded(px(4.))
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .w(px(current as f32 * 2.0)) // 200px max width
                                .bg(rgba(0x3b82f6ff))
                                .rounded(px(4.)),
                        ),
                )
                // Volume percentage
                .child(
                    div()
                        .w(px(40.))
                        .text_size(px(12.))
                        .text_color(rgba(0x888888ff))
                        .child(format!("{}%", current)),
                ),
        )
    }

    fn render_quick_toggles(&self, wifi_enabled: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let wifi_bg = if wifi_enabled {
            rgba(0x3b82f6ff)
        } else {
            rgba(0x333333ff)
        };
        let wifi_hover_bg = if wifi_enabled {
            rgba(0x2563ebff)
        } else {
            rgba(0x444444ff)
        };

        div()
            .flex()
            .flex_wrap()
            .gap(px(8.))
            // WiFi toggle
            .child(
                div()
                    .id("wifi-toggle")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(16.))
                    .py(px(12.))
                    .rounded(px(8.))
                    .bg(wifi_bg)
                    .cursor_pointer()
                    .hover(move |s| s.bg(wifi_hover_bg))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.toggle_wifi(cx)),
                    )
                    .child(div().text_size(px(16.)).child("󰤨"))
                    .child(div().text_size(px(13.)).child("Wi-Fi")),
            )
    }
}

struct BatteryInfo {
    percent: Option<u8>,
    charging: bool,
    time_to_full: Option<String>,
    time_to_empty: Option<String>,
}

fn read_battery_info() -> BatteryInfo {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let percent = fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok());

        let charging = fs::read_to_string("/sys/class/power_supply/BAT0/status")
            .map(|s| s.trim() == "Charging")
            .unwrap_or(false);

        // Try to get time estimates from upower
        let time_to_full = None; // Would need upower D-Bus
        let time_to_empty = None;

        return BatteryInfo {
            percent,
            charging,
            time_to_full,
            time_to_empty,
        };
    }

    #[cfg(not(target_os = "linux"))]
    BatteryInfo {
        percent: None,
        charging: false,
        time_to_full: None,
        time_to_empty: None,
    }
}
