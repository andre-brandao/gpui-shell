use crate::launcher::view::{InputResult, LauncherView, ViewContext, ViewInput};
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

    fn render_system_info(&self, vx: &ViewContext, cx: &App) -> AnyElement {
        let upower = vx.services.upower.read(cx);
        let audio = vx.services.audio.read(cx);
        let network = vx.services.network.read(cx);
        let compositor = vx.services.compositor.read(cx);

        let (battery_percent, battery_icon) = if let Some(ref battery) = upower.data.battery {
            let icon = match battery.status {
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
            };
            (battery.percentage, icon)
        } else {
            (0, "󰂃")
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

        div()
            .w_full()
            .p(px(16.))
            .bg(rgba(0x2a2a2aff))
            .rounded(px(8.))
            .flex()
            .justify_between()
            .child(
                // Battery
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.))
                    .child(div().text_size(px(24.)).child(battery_icon))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgba(0x888888ff))
                            .child(format!("{}%", battery_percent)),
                    ),
            )
            .child(
                // Volume
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.))
                    .child(div().text_size(px(24.)).child(volume_icon))
                    .child(div().text_size(px(12.)).text_color(rgba(0x888888ff)).child(
                        if audio.sink_muted {
                            "Muted".to_string()
                        } else {
                            format!("{}%", audio.sink_volume)
                        },
                    )),
            )
            .child(
                // WiFi
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.))
                    .child(div().text_size(px(24.)).child(wifi_icon))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgba(0x888888ff))
                            .child(if network.wifi_enabled { "On" } else { "Off" }),
                    ),
            )
            .child(
                // Workspaces
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.))
                    .child(div().text_size(px(24.)).child(""))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgba(0x888888ff))
                            .child(format!("{} ws", compositor.workspaces.len())),
                    ),
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
            .overflow_hidden()
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
