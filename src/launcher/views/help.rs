use crate::launcher::view::{InputResult, LIST_ITEM_HEIGHT, LauncherView, ViewContext, ViewInput};
use crate::widgets::sysinfo::icons;
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px, rgba};

pub struct HelpView {
    prefix_char: char,
    views: Vec<HelpEntry>,
}

struct HelpEntry {
    prefix: String,
    icon: String,
    description: String,
}

impl HelpView {
    pub fn new(prefix_char: char, views: &[Box<dyn LauncherView>]) -> Self {
        let entries = views
            .iter()
            .map(|v| HelpEntry {
                prefix: v.prefix().to_string(),
                icon: v.icon().to_string(),
                description: v.description().to_string(),
            })
            .collect();

        HelpView {
            prefix_char,
            views: entries,
        }
    }

    fn usage_color(usage: u32) -> gpui::Hsla {
        if usage >= 90 {
            gpui::rgb(0xef4444).into() // red - critical
        } else if usage >= 70 {
            gpui::rgb(0xf59e0b).into() // amber - warning
        } else {
            gpui::rgb(0x22c55e).into() // green - normal
        }
    }

    fn render_system_info(&self, vx: &ViewContext, cx: &App) -> AnyElement {
        let upower = vx.services.upower.read(cx);
        let audio = vx.services.audio.read(cx);
        let network = vx.services.network.read(cx);
        let sysinfo = vx.services.sysinfo.read(cx);

        let battery_icon = if let Some(ref battery) = upower.data.battery {
            match battery.status {
                crate::services::upower::BatteryStatus::Charging => "󰂄",
                _ => match battery.percentage {
                    0..=10 => "󰁺",
                    11..=20 => "󰁻",
                    21..=30 => "󰁼",
                    31..=40 => "󰁽",
                    41..=50 => "󰁾",
                    51..=60 => "󰁿",
                    61..=70 => "󰂀",
                    71..=80 => "󰂁",
                    81..=90 => "󰂂",
                    _ => "󰁹",
                },
            }
        } else {
            "󰂃"
        };

        let volume_icon = if audio.sink_muted {
            "󰝟"
        } else {
            match audio.sink_volume {
                0 => "󰕿",
                1..=50 => "󰖀",
                _ => "󰕾",
            }
        };

        let wifi_icon = if network.wifi_enabled { "󰤨" } else { "󰤭" };

        let cpu_usage = sysinfo.cpu_usage;
        let memory_usage = sysinfo.memory_usage;
        let cpu_color = Self::usage_color(cpu_usage);
        let memory_color = Self::usage_color(memory_usage);

        let cpu_icon = if cpu_usage >= 90 {
            icons::CPU_HIGH
        } else {
            icons::CPU
        };

        let temp_text = sysinfo
            .temperature
            .map(|t| format!("{}°C", t))
            .unwrap_or_else(|| "—".to_string());

        // Single compact row with all info
        div()
            .w_full()
            .px(px(12.))
            .py(px(8.))
            .bg(rgba(0x2a2a2aff))
            .rounded(px(8.))
            .flex()
            .items_center()
            .justify_between()
            // CPU
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(cpu_color)
                            .child(cpu_icon),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(cpu_color)
                            .child(format!("{}%", cpu_usage)),
                    ),
            )
            // RAM
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(memory_color)
                            .child(icons::MEMORY),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(memory_color)
                            .child(format!("{}%", memory_usage)),
                    ),
            )
            // Temp
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(rgba(0x888888ff))
                            .child(icons::TEMP),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgba(0x888888ff))
                            .child(temp_text),
                    ),
            )
            // Battery
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(rgba(0x888888ff))
                    .child(battery_icon),
            )
            // Volume
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(rgba(0x888888ff))
                    .child(volume_icon),
            )
            // WiFi
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(rgba(0x888888ff))
                    .child(wifi_icon),
            )
            .into_any_element()
    }

    fn render_commands(&self, vx: &ViewContext, selected_index: usize) -> AnyElement {
        let query_lower = vx.query.to_lowercase();

        let filtered: Vec<_> = self
            .views
            .iter()
            .filter(|entry| {
                if vx.query.is_empty() {
                    return true;
                }
                entry.prefix.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(filtered.into_iter().enumerate().map(|(i, entry)| {
                let is_selected = i == selected_index;
                let prefix_char = self.prefix_char;

                div()
                    .id(format!("cmd-{}", entry.prefix))
                    .w_full()
                    .h(px(LIST_ITEM_HEIGHT))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(is_selected, |el| el.bg(rgba(0x3b82f6ff)))
                    .when(!is_selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .w(px(32.))
                                    .h(px(32.))
                                    .rounded(px(6.))
                                    .bg(rgba(0x444444ff))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(16.))
                                    .child(entry.icon.clone()),
                            )
                            .child(
                                div().flex().flex_col().gap(px(2.)).child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(rgba(0x555555ff))
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(format!("{}{}", prefix_char, entry.prefix)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(rgba(0x888888ff))
                                                .child(entry.description.clone()),
                                        ),
                                ),
                            ),
                    )
            }))
            .into_any_element()
    }
}

impl LauncherView for HelpView {
    fn prefix(&self) -> &'static str {
        "help"
    }

    fn name(&self) -> &'static str {
        "Help"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let query_lower = vx.query.to_lowercase();

        let filtered_count = self
            .views
            .iter()
            .filter(|entry| {
                if vx.query.is_empty() {
                    return true;
                }
                entry.prefix.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .count();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(16.))
            // System info header
            .child(self.render_system_info(vx, cx))
            // Section title
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgba(0x666666ff))
                    .font_weight(FontWeight::MEDIUM)
                    .child("COMMANDS"),
            )
            // Commands list
            .child(self.render_commands(vx, vx.selected_index))
            .into_any_element();

        (element, filtered_count)
    }

    fn handle_input(&self, _input: &ViewInput, _vx: &ViewContext, _cx: &mut App) -> InputResult {
        // Help view uses default input handling
        InputResult::Unhandled
    }

    fn on_select(&self, index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let query_lower = vx.query.to_lowercase();

        let filtered: Vec<_> = self
            .views
            .iter()
            .filter(|entry| {
                if vx.query.is_empty() {
                    return true;
                }
                entry.prefix.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .collect();

        if let Some(_entry) = filtered.get(index) {
            // Return false - the launcher will handle SwitchView action
            false
        } else {
            false
        }
    }
}
