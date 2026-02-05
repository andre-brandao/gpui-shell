//! SysInfo panel showing detailed system information.
//!
//! This panel displays CPU, memory, swap, temperature, network, and disk information.

use super::icons;
use futures_signals::signal::SignalExt;
use gpui::{
    App, Context, FontWeight, Hsla, ScrollHandle, Window, div, prelude::*, px, relative, rems,
};
use services::{SysInfoData, SysInfoSubscriber};
use ui::prelude::*;

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

        SysInfoPanel {
            data: initial_data,
            scroll_handle: ScrollHandle::new(),
        }
    }

    fn status_color(usage: u32, cx: &App) -> Hsla {
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

    fn temp_color(temp_c: Option<i32>, cx: &App) -> Option<Hsla> {
        temp_c.map(|t| {
            let status = cx.theme().status();
            if t >= 80 {
                status.error
            } else if t >= 65 {
                status.warning
            } else {
                status.success
            }
        })
    }

    fn render_info_row(
        icon: &str,
        label: &str,
        value: &str,
        color: Option<Hsla>,
        cx: &App,
    ) -> impl IntoElement {
        let colors = cx.theme().colors();
        let text_color = color.unwrap_or(colors.text);

        div()
            .w_full()
            .flex()
            .items_center()
            .py(px(8.0))
            .child(
                div()
                    .w(px(32.0))
                    .text_size(rems(1.05))
                    .text_color(text_color)
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(rems(0.95))
                    .text_color(colors.text)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_size(rems(0.95))
                    .text_color(text_color)
                    .font_weight(FontWeight::MEDIUM)
                    .child(value.to_string()),
            )
    }

    fn render_progress_bar(usage: u32, cx: &App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let bar_color = Self::status_color(usage, cx);
        let width_percent = usage.min(100) as f32;

        div()
            .w_full()
            .h(px(4.0))
            .rounded(px(2.0))
            .bg(colors.element_background)
            .child(
                div()
                    .h_full()
                    .rounded(px(2.0))
                    .bg(bar_color)
                    .w(relative(width_percent / 100.0)),
            )
    }

    fn render_usage_section(
        icon: &str,
        title: &str,
        usage: u32,
        details: &str,
        cx: &App,
    ) -> impl IntoElement {
        let colors = cx.theme().colors();
        let bar_color = Self::status_color(usage, cx);

        div()
            .w_full()
            .p(px(14.0))
            .bg(colors.surface_background)
            .rounded(px(10.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(rems(1.1))
                                    .text_color(colors.text)
                                    .child(icon.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(rems(0.95))
                                    .text_color(colors.text)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(rems(1.05))
                            .font_weight(FontWeight::BOLD)
                            .text_color(bar_color)
                            .child(format!("{}%", usage)),
                    ),
            )
            .child(Self::render_progress_bar(usage, cx))
            .child(
                div()
                    .text_size(rems(0.85))
                    .text_color(colors.text_muted)
                    .child(details.to_string()),
            )
    }
}

impl Render for SysInfoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();

        let cpu_usage = self.data.cpu_usage;
        let memory_usage = self.data.memory_usage;
        let swap_usage = self.data.swap_usage;
        let memory_details = format!(
            "{:.1} GB / {:.1} GB used",
            self.data.memory_used_gb, self.data.memory_total_gb
        );

        let (temp_str, temp_color) = match self.data.temperature {
            Some(t) => (format!("{}°C", t), Self::temp_color(Some(t), cx)),
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

        div()
            .id("sysinfo-panel")
            .w_full()
            .h_full()
            .p(px(16.0))
            .bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .rounded(px(12.0))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(rems(1.2))
                            .text_color(colors.text)
                            .child(icons::SYSTEM),
                    )
                    .child(
                        div()
                            .text_size(rems(1.05))
                            .text_color(colors.text)
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
            .child(div().w_full().h(px(1.0)).bg(colors.border))
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
                    .p(px(12.0))
                    .bg(colors.surface_background)
                    .rounded(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(rems(1.05))
                                    .text_color(colors.text)
                                    .child(icons::NETWORK),
                            )
                            .child(
                                div()
                                    .text_size(rems(0.95))
                                    .text_color(colors.text)
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
                        .p(px(12.0))
                        .bg(colors.surface_background)
                        .rounded(px(10.0))
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .text_size(rems(1.05))
                                        .text_color(colors.text)
                                        .child(icons::DISK),
                                )
                                .child(
                                    div()
                                        .text_size(rems(0.95))
                                        .text_color(colors.text)
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("Disks"),
                                ),
                        )
                        .children(disks.iter().map(|disk| {
                            let details =
                                format!("{:.1} GB / {:.1} GB", disk.used_gb, disk.total_gb);
                            let disk_color = Self::status_color(disk.usage_percent, cx);
                            let width_percent = disk.usage_percent.min(100) as f32;

                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .py(px(8.0))
                                        .child(
                                            div()
                                                .w(px(32.0))
                                                .text_size(rems(1.0))
                                                .text_color(disk_color)
                                                .child(icons::DISK_FOLDER),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(rems(0.95))
                                                .text_color(colors.text)
                                                .child(disk.mount_point.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(rems(0.95))
                                                .text_color(disk_color)
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(format!("{}%", disk.usage_percent)),
                                        ),
                                )
                                .child(
                                    div().pl(px(32.0)).child(
                                        div()
                                            .w_full()
                                            .h(px(4.0))
                                            .rounded(px(2.0))
                                            .bg(colors.element_background)
                                            .child(
                                                div()
                                                    .h_full()
                                                    .rounded(px(2.0))
                                                    .bg(disk_color)
                                                    .w(relative(width_percent / 100.0)),
                                            ),
                                    ),
                                )
                                .child(
                                    div()
                                        .pl(px(32.0))
                                        .text_size(rems(0.85))
                                        .text_color(colors.text_muted)
                                        .child(details),
                                )
                        })),
                )
            })
    }
}
