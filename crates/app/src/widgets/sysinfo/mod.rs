//! SysInfo widget showing CPU and memory usage in the bar.
//!
//! Clicking the widget opens a detailed system information panel.

use crate::panel::{PanelConfig, toggle_panel};
use futures_signals::signal::SignalExt;
use gpui::{
    App, Context, Hsla, MouseButton, Window, div, layer_shell::Anchor, prelude::*, px, rems,
};
use services::{Services, SysInfoData, SysInfoSubscriber};
use ui::prelude::*;

mod panel;
pub use panel::SysInfoPanel;

/// Nerd Font icons for system info
pub mod icons {
    // CPU icons
    pub const CPU: &str = ""; // microchip
    pub const CPU_HIGH: &str = ""; // flame

    // Memory icons
    pub const MEMORY: &str = "󰍛"; // nf-md-memory
    pub const SWAP: &str = "󰾴"; // nf-md-swap_horizontal

    // Temperature icons
    pub const TEMP: &str = ""; // thermometer-half
    pub const TEMP_HOT: &str = ""; // thermometer-full

    // Disk icons
    pub const DISK: &str = "󰋊"; // nf-md-harddisk
    pub const DISK_FOLDER: &str = "󰉋"; // nf-md-folder

    // Network icons
    pub const NETWORK: &str = "󰛳"; // nf-md-network
    pub const IP: &str = "󰩟"; // nf-md-ip_network
    pub const DOWNLOAD: &str = "󰇚"; // nf-md-download
    pub const UPLOAD: &str = "󰕒"; // nf-md-upload

    // System/header icon
    pub const SYSTEM: &str = ""; // server
}

pub struct SysInfo {
    subscriber: SysInfoSubscriber,
    data: SysInfoData,
}

impl SysInfo {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let subscriber = services.sysinfo.clone();
        let initial_data = subscriber.get();

        // Subscribe to updates from the sysinfo service
        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    if this
                        .update(cx, |this, cx| {
                            this.data = data;
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
        })
        .detach();

        SysInfo {
            subscriber,
            data: initial_data,
        }
    }

    fn toggle_panel(&mut self, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        let config = PanelConfig {
            width: 350.0,
            height: 450.0,
            anchor: Anchor::TOP | Anchor::LEFT,
            margin: (0.0, 0.0, 0.0, 8.0),
            namespace: "sysinfo-panel".to_string(),
        };

        toggle_panel("sysinfo", config, cx, move |cx| {
            SysInfoPanel::new(subscriber, cx)
        });
    }

    fn cpu_icon(&self) -> &'static str {
        if self.data.cpu_usage >= 90 {
            icons::CPU_HIGH
        } else {
            icons::CPU
        }
    }

    fn memory_icon(&self) -> &'static str {
        if self.data.memory_usage >= 90 {
            icons::SWAP
        } else {
            icons::MEMORY
        }
    }

    fn usage_color(&self, usage: u32, cx: &Context<Self>) -> Hsla {
        let status = cx.theme().status();
        let colors = cx.theme().colors();
        if usage >= 90 {
            status.error
        } else if usage >= 75 {
            status.warning
        } else if usage >= 50 {
            status.success
        } else {
            colors.text
        }
    }
}

impl Render for SysInfo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();

        let cpu_usage = self.data.cpu_usage;
        let memory_usage = self.data.memory_usage;
        let cpu_icon = self.cpu_icon();
        let memory_icon = self.memory_icon();
        let cpu_color = self.usage_color(cpu_usage, cx);
        let memory_color = self.usage_color(memory_usage, cx);

        let hover_bg = colors.element_hover;
        let active_bg = colors.element_active;

        div()
            .id("sysinfo-widget")
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(10.0))
            .py(px(5.0))
            .rounded(px(9.0))
            .cursor_pointer()
            .bg(colors.element_background)
            .hover(move |s| s.bg(hover_bg))
            .active(move |s| s.bg(active_bg))
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
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(rems(0.9))
                            .text_color(cpu_color)
                            .child(cpu_icon),
                    )
                    .child(
                        div()
                            .text_size(rems(0.8))
                            .text_color(cpu_color)
                            .child(format!("{}%", cpu_usage)),
                    ),
            )
            // Memory usage
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(rems(0.9))
                            .text_color(memory_color)
                            .child(memory_icon),
                    )
                    .child(
                        div()
                            .text_size(rems(0.8))
                            .text_color(memory_color)
                            .child(format!("{}%", memory_usage)),
                    ),
            )
    }
}
