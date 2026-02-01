mod panel;

use crate::services::Services;
use crate::services::upower::BatteryStatus;
use gpui::{
    App, AppContext, Bounds, Context, MouseButton, Point, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, div, layer_shell::*, prelude::*, px,
    rgba,
};
use panel::InfoPanel;

/// Info widget showing battery, volume, and network status icons.
/// Clicking opens a detailed settings panel.
pub struct Info {
    services: Services,
    panel_window: Option<WindowHandle<InfoPanel>>,
}

impl Info {
    pub fn with_services(services: Services, cx: &mut Context<Self>) -> Self {
        // Observe services for updates
        cx.observe(&services.network, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.upower, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.audio, |_, _, cx| cx.notify()).detach();

        Info {
            services,
            panel_window: None,
        }
    }

    fn toggle_panel(&mut self, cx: &mut App) {
        if let Some(handle) = self.panel_window.take() {
            // Close the panel
            let _ = handle.update(cx, |_, window, _| {
                window.remove_window();
            });
        } else {
            // Open the panel
            let services = self.services.clone();

            if let Ok(window) = cx.open_window(
                WindowOptions {
                    titlebar: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::new(px(0.), px(0.)),
                        size: Size::new(px(320.), px(400.)),
                    })),
                    app_id: Some("gpui-info-panel".to_string()),
                    window_background: WindowBackgroundAppearance::Transparent,
                    kind: WindowKind::LayerShell(LayerShellOptions {
                        namespace: "info-panel".to_string(),
                        layer: Layer::Overlay,
                        anchor: Anchor::TOP | Anchor::RIGHT,
                        exclusive_zone: None,
                        margin: Some((px(0.), px(8.), px(0.), px(0.))),
                        keyboard_interactivity: KeyboardInteractivity::OnDemand,
                        ..Default::default()
                    }),
                    focus: true,
                    ..Default::default()
                },
                move |_, cx| cx.new(|cx| InfoPanel::with_services(services, cx)),
            ) {
                self.panel_window = Some(window);
            }
        }
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
}

impl Render for Info {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let battery_icon = self.battery_icon(cx);
        let volume_icon = self.volume_icon(cx);
        let wifi_icon = self.wifi_icon(cx);
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
