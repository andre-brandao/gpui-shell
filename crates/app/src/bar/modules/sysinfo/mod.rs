//! SysInfo widget showing CPU and memory usage in the bar.
//!
//! Clicking the widget opens a detailed system information panel.

use crate::panel::{toggle_panel, PanelConfig};
use gpui::{div, prelude::*, px, App, Context, MouseButton, Size, Window};
use services::SysInfoData;
use ui::{radius, ActiveTheme};

mod config;
pub use config::SysInfoConfig;

use super::style;
use crate::bar::modules::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::panel::panel_placement_from_event;
use crate::state::watch;
use crate::state::AppState;

mod panel;
pub use panel::SysInfoPanel;

/// Nerd Font icons for system info
pub mod icons {
    // CPU icons
    pub const CPU: &str = "󰻠"; // nf-md-chip
    pub const CPU_HIGH: &str = ""; // nf-fa-fire

    // Memory icons
    pub const MEMORY: &str = "󰍛"; // nf-md-memory
    pub const SWAP: &str = "󰾴"; // nf-md-swap_horizontal

    // Temperature icons
    pub const TEMP: &str = "󱃂"; // nf-md-thermometer
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
    pub const SYSTEM: &str = ""; // nf-fa-server
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
        let subscriber = AppState::sysinfo(cx).clone();
        let initial_data = subscriber.get();

        // Subscribe to updates from the sysinfo service
        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        SysInfo {
            slot,
            subscriber,
            data: initial_data,
        }
    }

    fn toggle_panel(&mut self, event: &gpui::MouseDownEvent, window: &Window, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        let config = Config::global(cx);
        let panel_size = Size::new(px(350.0), px(450.0));
        let (anchor, margin) =
            panel_placement_from_event(config.bar.position, event, window, cx, panel_size);
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

    fn usage_text(usage: u32, is_vertical: bool) -> String {
        style::compact_percent(usage, is_vertical)
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
        let cpu_text = Self::usage_text(cpu_usage, is_vertical);
        let memory_text = Self::usage_text(memory_usage, is_vertical);
        let cpu_color = theme.status.from_percentage(cpu_usage);
        let memory_color = theme.status.from_percentage(memory_usage);
        let config = &cx.config().bar.modules.sysinfo;

        // Pre-compute colors for closures
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let icon_size = style::icon(is_vertical);
        let text_size = style::label(is_vertical);

        div()
            .id("sysinfo-widget")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(style::CHIP_GAP))
            .px(px(style::chip_padding_x(is_vertical)))
            .py(px(style::CHIP_PADDING_Y))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .bg(interactive_default)
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| {
                    this.toggle_panel(event, window, cx);
                }),
            )
            .children({
                let mut sections: Vec<gpui::AnyElement> = Vec::new();

                let stat = |icon: &'static str, text: String, color: gpui::Hsla| {
                    div()
                        .flex()
                        .when(is_vertical, |this| this.flex_col())
                        .items_center()
                        .gap(px(style::CHIP_GAP))
                        .child(div().text_size(px(icon_size)).text_color(color).child(icon))
                        .child(div().text_size(px(text_size)).text_color(color).child(text))
                        .into_any_element()
                };

                if config.show_cpu {
                    sections.push(stat(cpu_icon, cpu_text, cpu_color));
                }

                if config.show_memory {
                    sections.push(stat(memory_icon, memory_text, memory_color));
                }

                if config.show_temp {
                    if let Some(temp) = self.data.temperature {
                        let temp_icon = if temp >= 70 {
                            icons::TEMP_HOT
                        } else {
                            icons::TEMP
                        };
                        let temp_text = if is_vertical {
                            format!("{temp}")
                        } else {
                            format!("{temp}°C")
                        };
                        let temp_color = theme.status.from_temperature(temp);
                        sections.push(stat(temp_icon, temp_text, temp_color));
                    }
                }

                sections
            })
    }
}
