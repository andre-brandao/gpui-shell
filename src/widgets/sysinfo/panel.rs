use crate::services::Services;
use gpui::{Context, FontWeight, ScrollHandle, Window, div, prelude::*, px, rgba};

use super::icons;

/// SysInfo panel content showing detailed system information.
pub struct SysInfoPanelContent {
    services: Services,
    scroll_handle: ScrollHandle,
}

impl SysInfoPanelContent {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        // Observe sysinfo service for updates
        cx.observe(&services.sysinfo, |_, _, cx| cx.notify())
            .detach();

        SysInfoPanelContent {
            services,
            scroll_handle: ScrollHandle::new(),
        }
    }

    fn render_info_row(
        icon: &str,
        label: &str,
        value: &str,
        color: Option<gpui::Hsla>,
    ) -> impl IntoElement {
        let text_color = color.unwrap_or_else(|| rgba(0xffffffff).into());

        div()
            .w_full()
            .flex()
            .items_center()
            .py(px(8.))
            .child(
                div()
                    .w(px(32.))
                    .text_size(px(18.))
                    .text_color(text_color)
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(13.))
                    .text_color(rgba(0xffffffff))
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(text_color)
                    .font_weight(FontWeight::MEDIUM)
                    .child(value.to_string()),
            )
    }

    fn usage_color(usage: u32) -> gpui::Hsla {
        if usage >= 90 {
            gpui::rgb(0xf87171).into() // red - critical (brighter)
        } else if usage >= 70 {
            gpui::rgb(0xfbbf24).into() // amber - warning (brighter)
        } else {
            gpui::rgb(0x4ade80).into() // green - normal (brighter)
        }
    }

    fn temp_color(temp: i32) -> gpui::Hsla {
        if temp >= 85 {
            gpui::rgb(0xf87171).into() // red - critical (brighter)
        } else if temp >= 70 {
            gpui::rgb(0xfbbf24).into() // amber - warning (brighter)
        } else {
            gpui::rgb(0x4ade80).into() // green - normal (brighter)
        }
    }

    fn render_progress_bar(usage: u32) -> impl IntoElement {
        let color = Self::usage_color(usage);
        let width_percent = usage.min(100) as f32;

        div()
            .w_full()
            .h(px(4.))
            .rounded(px(2.))
            .bg(rgba(0x404040ff))
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
    ) -> impl IntoElement {
        let color = Self::usage_color(usage);

        div()
            .w_full()
            .p(px(12.))
            .bg(rgba(0x333333ff))
            .rounded(px(8.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .text_color(rgba(0xffffffff))
                                    .child(icon.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgba(0xffffffff))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(color)
                            .child(format!("{}%", usage)),
                    ),
            )
            .child(Self::render_progress_bar(usage))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgba(0xccccccff))
                    .child(details.to_string()),
            )
    }
}

impl Render for SysInfoPanelContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sysinfo = self.services.sysinfo.read(cx);

        let cpu_usage = sysinfo.cpu_usage;
        let memory_usage = sysinfo.memory_usage;
        let swap_usage = sysinfo.swap_usage;
        let memory_details = format!(
            "{:.1} GB / {:.1} GB used",
            sysinfo.memory_used_gb, sysinfo.memory_total_gb
        );

        let (temp_str, temp_color) = match sysinfo.temperature {
            Some(t) => (format!("{}°C", t), Some(Self::temp_color(t))),
            None => ("N/A".to_string(), None),
        };

        let temp_icon = match sysinfo.temperature {
            Some(t) if t >= 70 => icons::TEMP_HOT,
            _ => icons::TEMP,
        };

        let ip_str = sysinfo
            .network
            .ip
            .clone()
            .unwrap_or_else(|| "No IP".to_string());

        let download_str = if sysinfo.network.download_speed >= 1000 {
            format!("{} MB/s", sysinfo.network.download_speed / 1000)
        } else {
            format!("{} KB/s", sysinfo.network.download_speed)
        };

        let upload_str = if sysinfo.network.upload_speed >= 1000 {
            format!("{} MB/s", sysinfo.network.upload_speed / 1000)
        } else {
            format!("{} KB/s", sysinfo.network.upload_speed)
        };

        let cpu_icon = if cpu_usage >= 90 {
            icons::CPU_HIGH
        } else {
            icons::CPU
        };

        div()
            .id("sysinfo-panel")
            .w_full()
            .h_full()
            .p(px(16.))
            .bg(rgba(0x242424ff))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .flex()
            .flex_col()
            .gap(px(12.))
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(rgba(0xffffffff))
                            .child(icons::SYSTEM),
                    )
                    .child(
                        div()
                            .text_size(px(16.))
                            .text_color(rgba(0xffffffff))
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
            ))
            // Memory Section
            .child(Self::render_usage_section(
                icons::MEMORY,
                "Memory Usage",
                memory_usage,
                &memory_details,
            ))
            // Swap Section (only show if swap is being used)
            .when(swap_usage > 0, |el| {
                el.child(Self::render_usage_section(
                    icons::SWAP,
                    "Swap Usage",
                    swap_usage,
                    "Swap memory",
                ))
            })
            // Divider
            .child(div().w_full().h(px(1.)).bg(rgba(0x444444ff)))
            // Temperature
            .child(Self::render_info_row(
                temp_icon,
                "Temperature",
                &temp_str,
                temp_color,
            ))
            // Network section
            .child(
                div()
                    .w_full()
                    .p(px(12.))
                    .bg(rgba(0x333333ff))
                    .rounded(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .text_color(rgba(0xffffffff))
                                    .child(icons::NETWORK),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgba(0xffffffff))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Network"),
                            ),
                    )
                    .child(Self::render_info_row(
                        icons::IP,
                        "IP Address",
                        &ip_str,
                        None,
                    ))
                    .child(Self::render_info_row(
                        icons::DOWNLOAD,
                        "Download",
                        &download_str,
                        None,
                    ))
                    .child(Self::render_info_row(
                        icons::UPLOAD,
                        "Upload",
                        &upload_str,
                        None,
                    )),
            )
            // Disks section
            .when(!sysinfo.disks.is_empty(), |el| {
                el.child(
                    div()
                        .w_full()
                        .p(px(12.))
                        .bg(rgba(0x333333ff))
                        .rounded(px(8.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .text_color(rgba(0xffffffff))
                                        .child(icons::DISK),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(rgba(0xffffffff))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("Disks"),
                                ),
                        )
                        .children(sysinfo.disks.iter().map(|disk| {
                            let details =
                                format!("{:.1} GB / {:.1} GB", disk.used_gb, disk.total_gb);
                            let color = Some(Self::usage_color(disk.usage_percent));

                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(Self::render_info_row(
                                    icons::DISK_FOLDER,
                                    &disk.mount_point,
                                    &format!("{}%", disk.usage_percent),
                                    color,
                                ))
                                .child(
                                    div()
                                        .pl(px(32.))
                                        .child(Self::render_progress_bar(disk.usage_percent)),
                                )
                                .child(
                                    div()
                                        .pl(px(32.))
                                        .text_size(px(11.))
                                        .text_color(rgba(0xccccccff))
                                        .child(details),
                                )
                        })),
                )
            })
    }
}
