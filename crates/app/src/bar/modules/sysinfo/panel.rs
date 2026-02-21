//! SysInfo panel showing detailed system information.
//!
//! This panel displays CPU, memory, swap, temperature, network, and disk information.

use super::icons;
use crate::state::watch;
use gpui::{div, prelude::*, px, App, Context, FontWeight, Hsla, ScrollHandle, Window};
use services::{SysInfoData, SysInfoSubscriber};
use ui::{icon_size, radius, spacing, ActiveTheme};

/// SysInfo panel content showing detailed system information.
pub struct SysInfoPanel {
    data: SysInfoData,
    scroll_handle: ScrollHandle,
}

impl SysInfoPanel {
    /// Create a new SysInfo panel with the given subscriber.
    pub fn new(subscriber: SysInfoSubscriber, cx: &mut Context<Self>) -> Self {
        let initial_data = subscriber.get();

        // Subscribe to updates from the sysinfo service
        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        SysInfoPanel {
            data: initial_data,
            scroll_handle: ScrollHandle::new(),
        }
    }

    fn render_info_row(
        icon: &str,
        label: &str,
        value: &str,
        color: Option<Hsla>,
        cx: &App,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let text_color = color.unwrap_or(theme.text.primary);

        div()
            .w_full()
            .flex()
            .items_center()
            .py(px(spacing::SM))
            .child(
                div()
                    .w(px(32.))
                    .text_size(px(icon_size::XL))
                    .text_color(text_color)
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(theme.font_sizes.base)
                    .text_color(theme.text.primary)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_size(theme.font_sizes.base)
                    .text_color(text_color)
                    .font_weight(FontWeight::MEDIUM)
                    .child(value.to_string()),
            )
    }

    fn render_progress_bar(usage: u32, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let color = theme.status.from_percentage(usage);
        let width_percent = usage.min(100) as f32;

        div()
            .w_full()
            .h(px(4.))
            .rounded(px(2.))
            .bg(theme.bg.tertiary)
            .child(
                div()
                    .h_full()
                    .rounded(px(2.))
                    .bg(color)
                    .w(gpui::relative(width_percent / 100.0)),
            )
    }

    fn render_usage_section(
        icon: &str,
        title: &str,
        usage: u32,
        details: &str,
        cx: &App,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let color = theme.status.from_percentage(usage);

        div()
            .w_full()
            .p(px(spacing::MD))
            .bg(theme.bg.secondary)
            .rounded(px(radius::MD))
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(icon_size::LG))
                                    .text_color(theme.text.primary)
                                    .child(icon.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(theme.font_sizes.base)
                                    .text_color(theme.text.primary)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(theme.font_sizes.md)
                            .font_weight(FontWeight::BOLD)
                            .text_color(color)
                            .child(format!("{}%", usage)),
                    ),
            )
            .child(Self::render_progress_bar(usage, cx))
            .child(
                div()
                    .text_size(theme.font_sizes.sm)
                    .text_color(theme.text.secondary)
                    .child(details.to_string()),
            )
    }
}

impl Render for SysInfoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let cpu_usage = self.data.cpu_usage;
        let memory_usage = self.data.memory_usage;
        let swap_usage = self.data.swap_usage;
        let memory_details = format!(
            "{:.1} GB / {:.1} GB used",
            self.data.memory_used_gb, self.data.memory_total_gb
        );

        let (temp_str, temp_color) = match self.data.temperature {
            Some(t) => (format!("{}°C", t), Some(theme.status.from_temperature(t))),
            None => ("N/A".to_string(), None),
        };

        let temp_icon = match self.data.temperature {
            Some(t) if t >= 70 => icons::TEMP_HOT,
            _ => icons::TEMP,
        };

        let ip_str = self
            .data
            .network
            .ip
            .clone()
            .unwrap_or_else(|| "No IP".to_string());

        let download_str = if self.data.network.download_speed >= 1000 {
            format!("{} MB/s", self.data.network.download_speed / 1000)
        } else {
            format!("{} KB/s", self.data.network.download_speed)
        };

        let upload_str = if self.data.network.upload_speed >= 1000 {
            format!("{} MB/s", self.data.network.upload_speed / 1000)
        } else {
            format!("{} KB/s", self.data.network.upload_speed)
        };

        let cpu_icon = if cpu_usage >= 90 {
            icons::CPU_HIGH
        } else {
            icons::CPU
        };

        let disks = self.data.disks.clone();

        // Pre-compute theme colors for closures
        let text_primary = theme.text.primary;
        let text_secondary = theme.text.secondary;
        let bg_secondary = theme.bg.secondary;
        let bg_tertiary = theme.bg.tertiary;

        div()
            .id("sysinfo-panel")
            .w_full()
            .h_full()
            .p(px(spacing::LG))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.default)
            .rounded(px(radius::LG))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .flex()
            .flex_col()
            .gap(px(spacing::MD))
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::SM))
                    .child(
                        div()
                            .text_size(px(icon_size::XL))
                            .text_color(theme.text.primary)
                            .child(icons::SYSTEM),
                    )
                    .child(
                        div()
                            .text_size(theme.font_sizes.lg)
                            .text_color(theme.text.primary)
                            .font_weight(FontWeight::BOLD)
                            .child("System Information"),
                    ),
            )
            // CPU Section
            .child(Self::render_usage_section(
                cpu_icon,
                "CPU Usage",
                cpu_usage,
                "Processor load",
                cx,
            ))
            // Memory Section
            .child(Self::render_usage_section(
                icons::MEMORY,
                "Memory Usage",
                memory_usage,
                &memory_details,
                cx,
            ))
            // Swap Section (only show if swap is being used)
            .when(swap_usage > 0, |el| {
                el.child(Self::render_usage_section(
                    icons::SWAP,
                    "Swap Usage",
                    swap_usage,
                    "Swap memory",
                    cx,
                ))
            })
            // Divider
            .child(div().w_full().h(px(1.)).bg(theme.border.default))
            // Temperature
            .child(Self::render_info_row(
                temp_icon,
                "Temperature",
                &temp_str,
                temp_color,
                cx,
            ))
            // Network section
            .child(
                div()
                    .w_full()
                    .p(px(spacing::MD))
                    .bg(theme.bg.secondary)
                    .rounded(px(radius::MD))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::SM))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(icon_size::LG))
                                    .text_color(theme.text.primary)
                                    .child(icons::NETWORK),
                            )
                            .child(
                                div()
                                    .text_size(theme.font_sizes.base)
                                    .text_color(theme.text.primary)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Network"),
                            ),
                    )
                    .child(Self::render_info_row(
                        icons::IP,
                        "IP Address",
                        &ip_str,
                        None,
                        cx,
                    ))
                    .child(Self::render_info_row(
                        icons::DOWNLOAD,
                        "Download",
                        &download_str,
                        None,
                        cx,
                    ))
                    .child(Self::render_info_row(
                        icons::UPLOAD,
                        "Upload",
                        &upload_str,
                        None,
                        cx,
                    )),
            )
            // Disks section
            .when(!disks.is_empty(), |el| {
                el.child(
                    div()
                        .w_full()
                        .p(px(spacing::MD))
                        .bg(bg_secondary)
                        .rounded(px(radius::MD))
                        .flex()
                        .flex_col()
                        .gap(px(spacing::SM))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(spacing::SM))
                                .child(
                                    div()
                                        .text_size(px(icon_size::LG))
                                        .text_color(text_primary)
                                        .child(icons::DISK),
                                )
                                .child(
                                    div()
                                        .text_size(theme.font_sizes.base)
                                        .text_color(text_primary)
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("Disks"),
                                ),
                        )
                        .children(disks.iter().map(|disk| {
                            let details =
                                format!("{:.1} GB / {:.1} GB", disk.used_gb, disk.total_gb);
                            let disk_color = theme.status.from_percentage(disk.usage_percent);
                            let width_percent = disk.usage_percent.min(100) as f32;

                            div()
                                .flex()
                                .flex_col()
                                .gap(px(spacing::XS))
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .py(px(spacing::SM))
                                        .child(
                                            div()
                                                .w(px(32.))
                                                .text_size(px(icon_size::XL))
                                                .text_color(disk_color)
                                                .child(icons::DISK_FOLDER),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(theme.font_sizes.base)
                                                .text_color(text_primary)
                                                .child(disk.mount_point.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(theme.font_sizes.base)
                                                .text_color(disk_color)
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(format!("{}%", disk.usage_percent)),
                                        ),
                                )
                                .child(
                                    div().pl(px(32.)).child(
                                        div()
                                            .w_full()
                                            .h(px(4.))
                                            .rounded(px(2.))
                                            .bg(bg_tertiary)
                                            .child(
                                                div()
                                                    .h_full()
                                                    .rounded(px(2.))
                                                    .bg(disk_color)
                                                    .w(gpui::relative(width_percent / 100.0)),
                                            ),
                                    ),
                                )
                                .child(
                                    div()
                                        .pl(px(32.))
                                        .text_size(theme.font_sizes.sm)
                                        .text_color(text_secondary)
                                        .child(details),
                                )
                        })),
                )
            })
    }
}
