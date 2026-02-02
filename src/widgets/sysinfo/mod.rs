use crate::services::Services;
use crate::ui::{PanelConfig, toggle_panel};
use gpui::{Context, MouseButton, Window, div, layer_shell::Anchor, prelude::*, px, rgba};

mod panel;
pub use panel::SysInfoPanelContent;

/// Nerd Font icons for system info
pub mod icons {
    // CPU icons
    pub const CPU: &str = ""; // nf-oct-cpu
    pub const CPU_HIGH: &str = ""; // nf-oct-flame

    // Memory icons
    pub const MEMORY: &str = "󰍛"; // nf-md-memory
    pub const SWAP: &str = "󰾴"; // nf-md-swap_horizontal

    // Temperature icons
    pub const TEMP: &str = ""; // nf-oct-thermometer
    pub const TEMP_HOT: &str = "󰸁"; // nf-md-thermometer_high

    // Disk icons
    pub const DISK: &str = "󰋊"; // nf-md-harddisk
    pub const DISK_FOLDER: &str = "󰉋"; // nf-md-folder

    // Network icons
    pub const NETWORK: &str = "󰛳"; // nf-md-network
    pub const IP: &str = "󰩟"; // nf-md-ip_network
    pub const DOWNLOAD: &str = "󰇚"; // nf-md-download
    pub const UPLOAD: &str = "󰕒"; // nf-md-upload

    // System/header icon
    pub const SYSTEM: &str = ""; // nf-oct-server
}

/// SysInfo widget showing CPU and memory usage in the bar.
/// Clicking opens a detailed system info panel.
pub struct SysInfoWidget {
    services: Services,
}

impl SysInfoWidget {
    pub fn with_services(services: Services, cx: &mut Context<Self>) -> Self {
        // Observe sysinfo service for updates
        cx.observe(&services.sysinfo, |_, _, cx| cx.notify())
            .detach();

        SysInfoWidget { services }
    }

    fn toggle_panel(&mut self, cx: &mut gpui::App) {
        let services = self.services.clone();
        let config = PanelConfig {
            width: 350.0,
            height: 450.0,
            anchor: Anchor::TOP | Anchor::LEFT,
            margin: (0.0, 0.0, 0.0, 8.0),
            namespace: "sysinfo-panel".to_string(),
        };

        toggle_panel("sysinfo", config, cx, move |cx| {
            SysInfoPanelContent::new(services, cx)
        });
    }

    fn cpu_icon(&self, cx: &Context<Self>) -> &'static str {
        let sysinfo = self.services.sysinfo.read(cx);
        if sysinfo.cpu_usage >= 90 {
            icons::CPU_HIGH
        } else {
            icons::CPU
        }
    }

    fn memory_icon(&self, cx: &Context<Self>) -> &'static str {
        let sysinfo = self.services.sysinfo.read(cx);
        if sysinfo.memory_usage >= 90 {
            icons::SWAP // Use swap icon to indicate high memory pressure
        } else {
            icons::MEMORY
        }
    }

    fn cpu_color(&self, cx: &Context<Self>) -> gpui::Hsla {
        let sysinfo = self.services.sysinfo.read(cx);
        let usage = sysinfo.cpu_usage;

        if usage >= 90 {
            gpui::rgb(0xef4444).into() // red - critical
        } else if usage >= 70 {
            gpui::rgb(0xf59e0b).into() // amber - warning
        } else {
            gpui::rgb(0xffffff).into() // white - normal
        }
    }

    fn memory_color(&self, cx: &Context<Self>) -> gpui::Hsla {
        let sysinfo = self.services.sysinfo.read(cx);
        let usage = sysinfo.memory_usage;

        if usage >= 90 {
            gpui::rgb(0xef4444).into() // red - critical
        } else if usage >= 70 {
            gpui::rgb(0xf59e0b).into() // amber - warning
        } else {
            gpui::rgb(0xffffff).into() // white - normal
        }
    }
}

impl Render for SysInfoWidget {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sysinfo = self.services.sysinfo.read(cx);
        let cpu_usage = sysinfo.cpu_usage;
        let memory_usage = sysinfo.memory_usage;
        let cpu_icon = self.cpu_icon(cx);
        let memory_icon = self.memory_icon(cx);
        let cpu_color = self.cpu_color(cx);
        let memory_color = self.memory_color(cx);

        div()
            .id("sysinfo-widget")
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
            // CPU usage
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(cpu_color)
                            .child(cpu_icon),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(cpu_color)
                            .child(format!("{}%", cpu_usage)),
                    ),
            )
            // Memory usage
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(memory_color)
                            .child(memory_icon),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(memory_color)
                            .child(format!("{}%", memory_usage)),
                    ),
            )
    }
}
