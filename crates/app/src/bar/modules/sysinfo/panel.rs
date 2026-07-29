//! SysInfo panel showing detailed system information.
//!
//! This panel displays CPU, memory, swap, temperature, network, and disk information.

use super::icons;
use crate::state::watch;
use gpui::{App, Context, FontWeight, Hsla, ScrollHandle, Window, div, prelude::*, px};
use services::{SysInfoData, SysInfoSubscriber};
use ui::patterns::PanelSurface;
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

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
        icon: IconName,
        label: &str,
        value: &str,
        color: Option<Hsla>,
        cx: &App,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let text_color = color.unwrap_or(theme.colors.text);

        div()
            .w_full()
            .flex()
            .items_center()
            .py(Spacing::Medium.pixels())
            .child(
                div().w(px(32.)).child(
                    Icon::new(icon)
                        .size(IconSize::Large)
                        .color(Color::Custom(text_color)),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(TextSize::Default.rems())
                    .text_color(theme.colors.text)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_size(TextSize::Default.rems())
                    .text_color(text_color)
                    .font_weight(FontWeight::MEDIUM)
                    .child(value.to_string()),
            )
    }

    fn render_progress_bar(usage: u32, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let color = theme.colors.status.from_percentage(usage);
        let width_percent = usage.min(100) as f32;

        div()
            .w_full()
            .h(px(4.))
            .rounded(px(2.))
            .bg(theme.colors.elevated_surface_background)
            .child(
                div()
                    .h_full()
                    .rounded(px(2.))
                    .bg(color)
                    .w(gpui::relative(width_percent / 100.0)),
            )
    }

    fn render_usage_section(
        icon: IconName,
        title: &str,
        usage: u32,
        details: &str,
        cx: &App,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let color = theme.colors.status.from_percentage(usage);

        div()
            .w_full()
            .p(Spacing::Large.pixels())
            .bg(theme.colors.surface_background)
            .rounded(Radius::Medium.pixels())
            .flex()
            .flex_col()
            .gap(Spacing::Medium.pixels())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                Icon::new(icon)
                                    .size(IconSize::Medium)
                                    .color(Color::Custom(theme.colors.text)),
                            )
                            .child(
                                div()
                                    .text_size(TextSize::Default.rems())
                                    .text_color(theme.colors.text)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(TextSize::Medium.rems())
                            .font_weight(FontWeight::BOLD)
                            .text_color(color)
                            .child(format!("{}%", usage)),
                    ),
            )
            .child(Self::render_progress_bar(usage, cx))
            .child(
                div()
                    .text_size(TextSize::Small.rems())
                    .text_color(theme.colors.text)
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
            Some(t) => (
                format!("{}°C", t),
                Some(theme.colors.status.from_temperature(t)),
            ),
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
        let text_primary = theme.colors.text;
        let text_secondary = theme.colors.text;
        let bg_secondary = theme.colors.surface_background;
        let bg_tertiary = theme.colors.elevated_surface_background;

        div()
            .id("sysinfo-panel")
            .w_full()
            .h_full()
            .p(Spacing::XLarge.pixels())
            .panel_surface(cx)
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .flex()
            .flex_col()
            .gap(Spacing::Large.pixels())
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::Medium.pixels())
                    .child(
                        Icon::new(icons::SYSTEM)
                            .size(IconSize::Large)
                            .color(Color::Custom(theme.colors.text)),
                    )
                    .child(
                        div()
                            .text_size(TextSize::Large.rems())
                            .text_color(theme.colors.text)
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
            .child(div().w_full().h(px(1.)).bg(theme.colors.border))
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
                    .p(Spacing::Large.pixels())
                    .bg(theme.colors.surface_background)
                    .rounded(Radius::Medium.pixels())
                    .flex()
                    .flex_col()
                    .gap(Spacing::Medium.pixels())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                Icon::new(icons::NETWORK)
                                    .size(IconSize::Medium)
                                    .color(Color::Custom(theme.colors.text)),
                            )
                            .child(
                                div()
                                    .text_size(TextSize::Default.rems())
                                    .text_color(theme.colors.text)
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
                        .p(Spacing::Large.pixels())
                        .bg(bg_secondary)
                        .rounded(Radius::Medium.pixels())
                        .flex()
                        .flex_col()
                        .gap(Spacing::Medium.pixels())
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(Spacing::Medium.pixels())
                                .child(
                                    Icon::new(icons::DISK)
                                        .size(IconSize::Medium)
                                        .color(Color::Custom(text_primary)),
                                )
                                .child(
                                    div()
                                        .text_size(TextSize::Default.rems())
                                        .text_color(text_primary)
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("Disks"),
                                ),
                        )
                        .children(disks.iter().map(|disk| {
                            let details =
                                format!("{:.1} GB / {:.1} GB", disk.used_gb, disk.total_gb);
                            let disk_color =
                                theme.colors.status.from_percentage(disk.usage_percent);
                            let width_percent = disk.usage_percent.min(100) as f32;

                            div()
                                .flex()
                                .flex_col()
                                .gap(Spacing::XSmall.pixels())
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .py(Spacing::Medium.pixels())
                                        .child(
                                            div().w(px(32.)).child(
                                                Icon::new(icons::DISK_FOLDER)
                                                    .size(IconSize::Large)
                                                    .color(Color::Custom(disk_color)),
                                            ),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(TextSize::Default.rems())
                                                .text_color(text_primary)
                                                .child(disk.mount_point.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(TextSize::Default.rems())
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
                                        .text_size(TextSize::Small.rems())
                                        .text_color(text_secondary)
                                        .child(details),
                                )
                        })),
                )
            })
    }
}
