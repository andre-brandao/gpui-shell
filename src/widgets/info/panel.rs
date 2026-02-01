use crate::services::Services;
use crate::services::audio::AudioCommand;
use crate::services::network::NetworkCommand;
use crate::services::upower::{BatteryStatus, PowerProfile, UPowerCommand};
use gpui::{
    AnyElement, Context, FocusHandle, Focusable, MouseButton, Window, div, prelude::*, px, rgba,
};

/// Info panel content - can be used standalone or in a panel window.
pub struct InfoPanelContent {
    services: Services,
    focus_handle: FocusHandle,
}

impl InfoPanelContent {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Observe all services for updates
        cx.observe(&services.network, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.upower, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.audio, |_, _, cx| cx.notify()).detach();

        InfoPanelContent {
            services,
            focus_handle,
        }
    }

    fn toggle_wifi(&mut self, cx: &mut Context<Self>) {
        self.services.network.update(cx, |network, cx| {
            network.dispatch(NetworkCommand::ToggleWiFi, cx);
        });
    }

    fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        self.services.audio.update(cx, |audio, cx| {
            audio.dispatch(AudioCommand::ToggleSinkMute, cx);
        });
    }

    fn set_volume(&mut self, percent: u8, cx: &mut Context<Self>) {
        self.services.audio.update(cx, |audio, cx| {
            audio.dispatch(AudioCommand::SetSinkVolume(percent), cx);
        });
    }

    fn cycle_power_profile(&mut self, cx: &mut Context<Self>) {
        self.services.upower.update(cx, |upower, cx| {
            upower.dispatch(UPowerCommand::CyclePowerProfile, cx);
        });
    }
}

impl Focusable for InfoPanelContent {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InfoPanelContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let wifi_enabled = self.services.network.read(cx).wifi_enabled;

        div()
            .id("info-panel-content")
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
            .child(self.render_battery_section(cx))
            // Volume section
            .child(self.render_volume_section(cx))
            // Divider
            .child(div().h(px(1.)).w_full().bg(rgba(0x333333ff)))
            // Quick toggles
            .child(self.render_quick_toggles(wifi_enabled, cx))
    }
}

impl InfoPanelContent {
    fn render_battery_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let upower = self.services.upower.read(cx);

        let (battery_icon, battery_text, time_text, battery_color) = match &upower.battery {
            Some(battery) => {
                let charging = battery.status == BatteryStatus::Charging;
                let percent = battery.percentage;

                let icon = if charging {
                    "󰂄"
                } else {
                    match percent {
                        p if p >= 90 => "󰁹",
                        p if p >= 70 => "󰂀",
                        p if p >= 50 => "󰁾",
                        p if p >= 30 => "󰁼",
                        p if p >= 10 => "󰁺",
                        _ => "󰂃",
                    }
                };

                let text = format!("{}%", percent);

                let time = if charging {
                    battery.time_to_full.map(|d| {
                        let mins = d.as_secs() / 60;
                        let hours = mins / 60;
                        let mins = mins % 60;
                        if hours > 0 {
                            format!("Full in {}h {}m", hours, mins)
                        } else {
                            format!("Full in {}m", mins)
                        }
                    })
                } else {
                    battery.time_to_empty.map(|d| {
                        let mins = d.as_secs() / 60;
                        let hours = mins / 60;
                        let mins = mins % 60;
                        if hours > 0 {
                            format!("{}h {}m remaining", hours, mins)
                        } else {
                            format!("{}m remaining", mins)
                        }
                    })
                };

                let color = match percent {
                    p if p >= 50 => rgba(0x22c55eff), // green
                    p if p >= 20 => rgba(0xeab308ff), // yellow
                    _ => rgba(0xef4444ff),            // red
                };

                (icon, text, time, color)
            }
            None => ("󰂑", "N/A".to_string(), None, rgba(0x888888ff)),
        };

        let power_profile = upower.power_profile;
        let profile_icon = match power_profile {
            PowerProfile::Performance => "󰓅",
            PowerProfile::Balanced => "󰾅",
            PowerProfile::PowerSaver => "󰾆",
            PowerProfile::Unknown => "󰾅",
        };
        let profile_text = match power_profile {
            PowerProfile::Performance => "Performance",
            PowerProfile::Balanced => "Balanced",
            PowerProfile::PowerSaver => "Power Saver",
            PowerProfile::Unknown => "Unknown",
        };

        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            // Battery row
            .child(
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
            // Power profile row
            .child(
                div()
                    .id("power-profile")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .bg(rgba(0x333333ff))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgba(0x444444ff)))
                    .child(div().text_size(px(16.)).child(profile_icon))
                    .child(div().text_size(px(13.)).child(profile_text)),
            )
    }

    fn render_volume_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let audio = self.services.audio.read(cx);
        let volume = audio.sink_volume;
        let muted = audio.sink_muted;

        let volume_icon = if muted {
            "󰝟"
        } else if volume >= 70 {
            "󰕾"
        } else if volume >= 30 {
            "󰖀"
        } else {
            "󰕿"
        };

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
                            cx.listener(|this, _, _, cx| this.toggle_mute(cx)),
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
                                .w(px(volume as f32 * 2.0)) // 200px max width
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
                        .child(format!("{}%", volume)),
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
