mod panel;

use crate::services::Services;
use crate::services::upower::BatteryStatus;
use crate::ui::{PanelConfig, toggle_panel};
use gpui::{Context, MouseButton, Window, div, layer_shell::Anchor, prelude::*, px, rgba};
use panel::InfoPanelContent;

pub use panel::InfoPanelContent as InfoPanel;

/// Info widget showing battery, volume, and network status icons.
/// Clicking opens a detailed settings panel.
pub struct Info {
    services: Services,
}

impl Info {
    pub fn with_services(services: Services, cx: &mut Context<Self>) -> Self {
        // Observe services for updates
        cx.observe(&services.network, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.upower, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.audio, |_, _, cx| cx.notify()).detach();
        cx.observe(&services.privacy, |_, _, cx| cx.notify())
            .detach();

        Info { services }
    }

    fn toggle_panel(&mut self, cx: &mut gpui::App) {
        let services = self.services.clone();
        let config = PanelConfig {
            width: 320.0,
            height: 400.0,
            anchor: Anchor::TOP | Anchor::RIGHT,
            margin: (0.0, 8.0, 0.0, 0.0),
            namespace: "info-panel".to_string(),
        };

        toggle_panel("info", config, cx, move |cx| {
            InfoPanelContent::new(services, cx)
        });
    }

    fn battery_icon(&self, cx: &Context<Self>) -> &'static str {
        let upower = self.services.upower.read(cx);

        match &upower.battery {
            Some(battery) => {
                let charging = battery.status == BatteryStatus::Charging;
                let percent = battery.percentage;

                match (charging, percent) {
                    (true, _) => "󰂄", // charging
                    (false, p) if p >= 90 => "󰁹",
                    (false, p) if p >= 70 => "󰂀",
                    (false, p) if p >= 50 => "󰁾",
                    (false, p) if p >= 30 => "󰁼",
                    (false, p) if p >= 10 => "󰁺",
                    (false, _) => "󰂃", // low
                }
            }
            None => "󰂑", // unknown/no battery
        }
    }

    fn battery_percent(&self, cx: &Context<Self>) -> Option<u8> {
        self.services
            .upower
            .read(cx)
            .battery
            .as_ref()
            .map(|b| b.percentage)
    }

    fn volume_icon(&self, cx: &Context<Self>) -> &'static str {
        let audio = self.services.audio.read(cx);

        if audio.sink_muted {
            "󰝟"
        } else if audio.sink_volume >= 70 {
            "󰕾"
        } else if audio.sink_volume >= 30 {
            "󰖀"
        } else if audio.sink_volume > 0 {
            "󰕿"
        } else {
            "󰝟"
        }
    }

    fn wifi_icon(&self, cx: &Context<Self>) -> &'static str {
        let network = self.services.network.read(cx);
        if !network.wifi_enabled {
            "󰤭" // disabled
        } else if network.connectivity == crate::services::network::ConnectivityState::Full {
            "󰤨" // connected
        } else if network.connectivity == crate::services::network::ConnectivityState::Limited {
            "󰤠" // limited
        } else {
            "󰤯" // disconnected
        }
    }

    fn privacy_icons(&self, cx: &Context<Self>) -> Vec<(&'static str, &'static str)> {
        let privacy = self.services.privacy.read(cx);
        let mut icons = Vec::new();

        if privacy.microphone_access() {
            icons.push(("", "#ef4444")); // FontAwesome microphone
        }
        if privacy.webcam_access() {
            icons.push(("", "#ef4444")); // FontAwesome camera
        }
        if privacy.screenshare_access() {
            icons.push(("󰍹", "#ef4444")); // red screen
        }

        icons
    }
}

impl Render for Info {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let battery_icon = self.battery_icon(cx);
        let volume_icon = self.volume_icon(cx);
        let wifi_icon = self.wifi_icon(cx);
        let privacy_icons = self.privacy_icons(cx);
        let battery_text = self
            .battery_percent(cx)
            .map(|p| format!("{}%", p))
            .unwrap_or_default();

        div()
            .id("info-widget")
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(8.))
            .py(px(4.))
            .rounded(px(4.))
            .cursor_pointer()
            .hover(|s| s.bg(rgba(0x333333ff)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.toggle_panel(cx);
                }),
            )
            // Privacy icons (only shown when active)
            .children(privacy_icons.into_iter().map(|(icon, color)| {
                div()
                    .text_size(px(14.))
                    .text_color(gpui::Hsla::from(gpui::rgb(0xef4444)))
                    .child(icon)
            }))
            // Volume icon
            .child(div().text_size(px(14.)).child(volume_icon))
            // WiFi icon
            .child(div().text_size(px(14.)).child(wifi_icon))
            // Battery icon and percentage
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(div().text_size(px(14.)).child(battery_icon))
                    .child(battery_text),
            )
    }
}
