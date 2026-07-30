//! SysInfo widget showing CPU and memory usage in the bar.
//!
//! Clicking the widget opens a detailed system information panel.

use crate::panel::{PanelConfig, toggle_panel};
use gpui::{AnyElement, App, ClickEvent, Context, Pixels, Point, Size, Window, prelude::*, px};
use services::SysInfoData;
use ui::{ActiveTheme, ButtonCommon, ButtonLike, ButtonStyle, Clickable, IconName};

mod config;
pub use config::SysInfoConfig;

use super::{BarWidget, style};
use crate::config::{ActiveConfig, Config};
use crate::panel::panel_placement_from_event;
use crate::state::AppState;
use crate::state::watch;

mod panel;
pub use panel::SysInfoPanel;

/// SysInfo widget showing CPU and memory usage in the bar.
pub struct SysInfo {
    subscriber: services::SysInfoSubscriber,
    data: SysInfoData,
}

struct SysInfoStat {
    icon: IconName,
    text: String,
    color: gpui::Hsla,
}

impl SysInfo {
    /// Create a new SysInfo widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::sysinfo(cx).clone();
        let initial_data = subscriber.get();

        // Subscribe to updates from the sysinfo service
        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        SysInfo {
            subscriber,
            data: initial_data,
        }
    }

    fn toggle_panel(&mut self, at: Point<Pixels>, window: &Window, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        let config = Config::global(cx);
        let panel_size = Size::new(px(350.0), px(450.0));
        let (anchor, margin) =
            panel_placement_from_event(config.bar.position, at, window, cx, panel_size);
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

    fn cpu_icon(&self) -> IconName {
        if self.data.cpu_usage >= 90 {
            IconName::Flame
        } else {
            IconName::Cpu
        }
    }

    fn memory_icon(&self) -> IconName {
        if self.data.memory_usage >= 90 {
            IconName::ArrowDownUp
        } else {
            IconName::MemoryStick
        }
    }

    fn usage_text(usage: u32, is_vertical: bool) -> String {
        style::compact_percent(usage, is_vertical)
    }

    fn stats(
        &self,
        theme: &ui::Theme,
        config: &SysInfoConfig,
        is_vertical: bool,
    ) -> Vec<SysInfoStat> {
        let mut stats = Vec::new();

        if config.show_cpu {
            let cpu_usage = self.data.cpu_usage;
            stats.push(SysInfoStat {
                icon: self.cpu_icon(),
                text: Self::usage_text(cpu_usage, is_vertical),
                color: theme.colors.status.from_percentage(cpu_usage),
            });
        }

        if config.show_memory {
            let memory_usage = self.data.memory_usage;
            stats.push(SysInfoStat {
                icon: self.memory_icon(),
                text: Self::usage_text(memory_usage, is_vertical),
                color: theme.colors.status.from_percentage(memory_usage),
            });
        }

        if config.show_temp
            && let Some(temp) = self.data.temperature
        {
            stats.push(SysInfoStat {
                icon: if temp >= 70 {
                    IconName::Flame
                } else {
                    IconName::Thermometer
                },
                text: if is_vertical {
                    temp.to_string()
                } else {
                    format!("{temp}°C")
                },
                color: theme.colors.status.from_temperature(temp),
            });
        }

        stats
    }

    fn render_stats(
        &mut self,
        cx: &mut Context<Self>,
        stats: Vec<SysInfoStat>,
        is_vertical: bool,
    ) -> AnyElement {
        ButtonLike::new("sysinfo-widget")
            .style(ButtonStyle::Transparent)
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                this.toggle_panel(event.position(), window, cx);
            }))
            .child(
                style::stack(is_vertical)
                    .justify_center()
                    .gap(px(style::CHIP_GAP))
                    .children(stats.into_iter().map(|stat| {
                        style::bar_stat(is_vertical, stat.icon, stat.text, stat.color)
                    })),
            )
            .into_any_element()
    }
}

impl BarWidget for SysInfo {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let config = &cx.config().bar.modules.sysinfo;
        let stats = self.stats(cx.theme(), config, true);
        self.render_stats(cx, stats, true)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let config = &cx.config().bar.modules.sysinfo;
        let stats = self.stats(cx.theme(), config, false);
        self.render_stats(cx, stats, false)
    }
}

impl Render for SysInfo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}
