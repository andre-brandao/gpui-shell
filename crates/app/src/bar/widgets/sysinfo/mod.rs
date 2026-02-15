//! SysInfo widget showing CPU and memory usage in the bar.
//!
//! Clicking the widget opens a detailed system information panel.

use crate::panel::{PanelConfig, toggle_panel};
use futures_signals::signal::SignalExt;
use gpui::{App, Context, MouseButton, Window, div, prelude::*, px};
use services::SysInfoData;
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

use crate::bar::widgets::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::panel::panel_placement;
use crate::state::AppState;

mod panel;
pub use panel::SysInfoPanel;

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
pub struct SysInfo {
    slot: WidgetSlot,
    subscriber: services::SysInfoSubscriber,
    data: SysInfoData,
}

impl SysInfo {
    /// Create a new SysInfo widget.
    pub fn new(slot: WidgetSlot, cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::services(cx).sysinfo.clone();
        let initial_data = subscriber.get();

        // Subscribe to updates from the sysinfo service
        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.data = data;
                            cx.notify();
                        })
                        .is_ok();

                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        SysInfo {
            slot,
            subscriber,
            data: initial_data,
        }
    }

    fn toggle_panel(&mut self, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        let config = Config::global(cx);
        let (anchor, margin) = panel_placement(config.bar.position, self.slot);
        let config = PanelConfig {
            width: 350.0,
            height: 450.0,
            anchor,
            margin,
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
}

impl Render for SysInfo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();

        let cpu_usage = self.data.cpu_usage;
        let memory_usage = self.data.memory_usage;
        let cpu_icon = self.cpu_icon();
        let memory_icon = self.memory_icon();
        let cpu_color = theme.status.from_percentage(cpu_usage);
        let memory_color = theme.status.from_percentage(memory_usage);

        // Pre-compute colors for closures
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;

        div()
            .id("sysinfo-widget")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(spacing::SM))
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
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
