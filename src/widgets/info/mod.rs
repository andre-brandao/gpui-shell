mod panel;

use crate::services::Services;
use gpui::{
    App, AppContext, Bounds, Context, MouseButton, Point, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, prelude::*, px, rgba,
};
use panel::InfoPanel;
use std::time::Duration;

/// Info widget showing battery, volume, and network status icons.
/// Clicking opens a detailed settings panel.
pub struct Info {
    services: Services,
    battery_percent: Option<u8>,
    battery_charging: bool,
    volume_percent: u8,
    volume_muted: bool,
    panel_open: bool,
}

impl Info {
    pub fn with_services(services: Services, cx: &mut Context<Self>) -> Self {
        // Observe network service for WiFi status
        cx.observe(&services.network, |_, _, cx| cx.notify())
            .detach();

        // Poll battery and volume periodically
        cx.spawn(async move |this, cx| {
            loop {
                let (battery_percent, battery_charging) = read_battery_status();
                let (volume_percent, volume_muted) = read_volume_status();

                let _ = this.update(cx, |this, cx| {
                    this.battery_percent = battery_percent;
                    this.battery_charging = battery_charging;
                    this.volume_percent = volume_percent;
                    this.volume_muted = volume_muted;
                    cx.notify();
                });

                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        })
        .detach();

        let (battery_percent, battery_charging) = read_battery_status();
        let (volume_percent, volume_muted) = read_volume_status();

        Info {
            services,
            battery_percent,
            battery_charging,
            volume_percent,
            volume_muted,
            panel_open: false,
        }
    }

    fn toggle_panel(&mut self, cx: &mut App) {
        if self.panel_open {
            // Panel will close itself
            self.panel_open = false;
        } else {
            self.panel_open = true;
            let services = self.services.clone();

            cx.open_window(
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
                        margin: Some((px(36.), px(8.), px(0.), px(0.))),
                        keyboard_interactivity: KeyboardInteractivity::OnDemand,
                        ..Default::default()
                    }),
                    focus: true,
                    ..Default::default()
                },
                move |_, cx| cx.new(|cx| InfoPanel::with_services(services, cx)),
            )
            .ok();
        }
    }

    fn battery_icon(&self) -> &'static str {
        match (self.battery_charging, self.battery_percent) {
            (true, _) => "󰂄", // charging
            (false, Some(p)) if p >= 90 => "󰁹",
            (false, Some(p)) if p >= 70 => "󰂀",
            (false, Some(p)) if p >= 50 => "󰁾",
            (false, Some(p)) if p >= 30 => "󰁼",
            (false, Some(p)) if p >= 10 => "󰁺",
            (false, Some(_)) => "󰂃", // low
            (false, None) => "󰂑",    // unknown
        }
    }

    fn volume_icon(&self) -> &'static str {
        if self.volume_muted {
            "󰝟"
        } else if self.volume_percent >= 70 {
            "󰕾"
        } else if self.volume_percent >= 30 {
            "󰖀"
        } else if self.volume_percent > 0 {
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
        let battery_icon = self.battery_icon();
        let volume_icon = self.volume_icon();
        let wifi_icon = self.wifi_icon(cx);
        let battery_text = self
            .battery_percent
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

fn read_battery_status() -> (Option<u8>, bool) {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let capacity = fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok());

        let charging = fs::read_to_string("/sys/class/power_supply/BAT0/status")
            .map(|s| s.trim() == "Charging")
            .unwrap_or(false);

        (capacity, charging)
    }

    #[cfg(not(target_os = "linux"))]
    (None, false)
}

fn read_volume_status() -> (u8, bool) {
    // Try to read volume using wpctl (WirePlumber/PipeWire)
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let output = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .ok();

        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Format: "Volume: 0.50" or "Volume: 0.50 [MUTED]"
            let muted = stdout.contains("[MUTED]");
            let volume = stdout
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<f32>().ok())
                .map(|v| (v * 100.0) as u8)
                .unwrap_or(0);

            return (volume, muted);
        }
    }

    (50, false) // fallback
}
