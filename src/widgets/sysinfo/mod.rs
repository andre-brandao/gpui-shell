use crate::services::Services;
use crate::theme::{font_size, icon_size, interactive, radius, spacing, status};
use crate::ui::{PanelConfig, toggle_panel};
use gpui::{Context, Hsla, MouseButton, Window, div, layer_shell::Anchor, prelude::*, px};

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

    fn cpu_color(&self, cx: &Context<Self>) -> Hsla {
        let sysinfo = self.services.sysinfo.read(cx);
        status::from_percentage(sysinfo.cpu_usage)
    }

    fn memory_color(&self, cx: &Context<Self>) -> Hsla {
        let sysinfo = self.services.sysinfo.read(cx);
        status::from_percentage(sysinfo.memory_usage)
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
            .gap(px(spacing::SM))
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .hover(|s| s.bg(interactive::hover()))
            .active(|s| s.bg(interactive::active()))
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
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(icon_size::MD))
                            .text_color(cpu_color)
                            .child(cpu_icon),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(cpu_color)
                            .child(format!("{}%", cpu_usage)),
                    ),
            )
            // Memory usage
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(icon_size::MD))
                            .text_color(memory_color)
                            .child(memory_icon),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(memory_color)
                            .child(format!("{}%", memory_usage)),
                    ),
            )
    }
}
