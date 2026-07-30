//! SysInfo panel showing detailed system information.
//!
//! This panel displays CPU, memory, swap, temperature, network, and disk information.

use crate::state::watch;
use gpui::{App, Context, FontWeight, Hsla, ScrollHandle, Window, div, prelude::*, px};
use services::{SysInfoData, SysInfoSubscriber};
use ui::patterns::PanelSurface;
use ui::{
    ActiveTheme, Color, Divider, Icon, IconName, IconSize, Label, LabelCommon, ProgressBar,
    Spacing, TextSize, h_flex, v_flex,
};

/// SysInfo panel content showing detailed system information.
pub struct SysInfoPanel {
    data: SysInfoData,
    scroll_handle: ScrollHandle,
}

/// Width of the icon gutter every info row aligns to.
const ROW_GUTTER: f32 = 32.0;

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
    ) -> impl IntoElement {
        let value_color = color.map_or(Color::Default, Color::Custom);

        h_flex()
            .w_full()
            .py(Spacing::Medium.pixels())
            .child(
                div()
                    .w(px(ROW_GUTTER))
                    .child(Icon::new(icon).size(IconSize::Large).color(value_color)),
            )
            .child(h_flex().flex_1().child(Label::new(label.to_string())))
            .child(
                Label::new(value.to_string())
                    .color(value_color)
                    .weight(FontWeight::MEDIUM),
            )
    }

    fn render_progress_bar(usage: u32, cx: &App) -> impl IntoElement {
        let colors = cx.theme().colors();

        ProgressBar::new(usage.min(100) as f32, 100.)
            .fg_color(colors.status.from_percentage(usage))
            .bg_color(colors.elevated_surface_background)
    }

    /// A card header: icon plus title, the shape every section in this panel
    /// opens with.
    fn render_section_header(icon: IconName, title: &str) -> impl IntoElement {
        h_flex()
            .gap(Spacing::Medium.pixels())
            .child(Icon::new(icon).size(IconSize::Medium).color(Color::Default))
            .child(Label::new(title.to_string()).weight(FontWeight::MEDIUM))
    }

    fn render_usage_section(
        icon: IconName,
        title: &str,
        usage: u32,
        details: &str,
        cx: &App,
    ) -> impl IntoElement {
        let color = cx.theme().colors.status.from_percentage(usage);

        v_flex()
            .w_full()
            .p(Spacing::Large.pixels())
            .panel_card(cx)
            .gap(Spacing::Medium.pixels())
            .child(
                h_flex()
                    .justify_between()
                    .child(Self::render_section_header(icon, title))
                    .child(
                        Label::new(format!("{}%", usage))
                            .size(TextSize::Medium)
                            .color(Color::Custom(color))
                            .weight(FontWeight::BOLD),
                    ),
            )
            .child(Self::render_progress_bar(usage, cx))
            .child(Label::new(details.to_string()).size(TextSize::Small))
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
            Some(t) if t >= 70 => IconName::Flame,
            _ => IconName::Thermometer,
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
            IconName::Flame
        } else {
            IconName::Cpu
        };

        let disks = self.data.disks.clone();

        v_flex()
            .id("sysinfo-panel")
            .w_full()
            .h_full()
            .p(Spacing::XLarge.pixels())
            .panel_surface(cx)
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .gap(Spacing::Large.pixels())
            // Header
            .child(
                h_flex()
                    .gap(Spacing::Medium.pixels())
                    .child(
                        Icon::new(IconName::Server)
                            .size(IconSize::Large)
                            .color(Color::Default),
                    )
                    .child(
                        Label::new("System Information")
                            .size(TextSize::Large)
                            .weight(FontWeight::BOLD),
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
                IconName::MemoryStick,
                "Memory Usage",
                memory_usage,
                &memory_details,
                cx,
            ))
            // Swap Section (only show if swap is being used)
            .when(swap_usage > 0, |el| {
                el.child(Self::render_usage_section(
                    IconName::ArrowDownUp,
                    "Swap Usage",
                    swap_usage,
                    "Swap memory",
                    cx,
                ))
            })
            .child(Divider::horizontal())
            // Temperature
            .child(Self::render_info_row(
                temp_icon,
                "Temperature",
                &temp_str,
                temp_color,
            ))
            // Network section
            .child(
                v_flex()
                    .w_full()
                    .p(Spacing::Large.pixels())
                    .panel_card(cx)
                    .gap(Spacing::Medium.pixels())
                    .child(Self::render_section_header(IconName::Network, "Network"))
                    .child(Self::render_info_row(
                        IconName::Globe,
                        "IP Address",
                        &ip_str,
                        None,
                    ))
                    .child(Self::render_info_row(
                        IconName::Download,
                        "Download",
                        &download_str,
                        None,
                    ))
                    .child(Self::render_info_row(
                        IconName::Upload,
                        "Upload",
                        &upload_str,
                        None,
                    )),
            )
            // Disks section
            .when(!disks.is_empty(), |el| {
                el.child(
                    v_flex()
                        .w_full()
                        .p(Spacing::Large.pixels())
                        .panel_card(cx)
                        .gap(Spacing::Medium.pixels())
                        .child(Self::render_section_header(IconName::HardDrive, "Disks"))
                        .children(disks.iter().map(|disk| {
                            let details =
                                format!("{:.1} GB / {:.1} GB", disk.used_gb, disk.total_gb);
                            let disk_color =
                                theme.colors.status.from_percentage(disk.usage_percent);

                            v_flex()
                                .gap(Spacing::XSmall.pixels())
                                .child(Self::render_info_row(
                                    IconName::Folder,
                                    &disk.mount_point,
                                    &format!("{}%", disk.usage_percent),
                                    Some(disk_color),
                                ))
                                .child(
                                    div()
                                        .pl(px(ROW_GUTTER))
                                        .child(Self::render_progress_bar(disk.usage_percent, cx)),
                                )
                                .child(
                                    div()
                                        .pl(px(ROW_GUTTER))
                                        .child(Label::new(details).size(TextSize::Small)),
                                )
                        })),
                )
            })
    }
}
