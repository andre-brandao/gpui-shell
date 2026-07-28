//! Help view showing available launcher commands and system information.

pub mod config;

use gpui::{AnyElement, App, div, prelude::*, px};
use ui::{
    ActiveTheme, Color, IconSize, Label, LabelCommon, ListItem, ListItemSpacing, Spacing, TextSize,
};

use self::config::HelpConfig;
use crate::bar::modules::sysinfo::icons;
use crate::launcher::view::{LauncherView, ViewContext};
use crate::state::AppState;

/// Help view - shows available commands and system status.
pub struct HelpView {
    prefix: String,
    entries: Vec<HelpEntry>,
}

struct HelpEntry {
    prefix: String,
    icon: String,
    name: String,
    description: String,
}

impl HelpView {
    pub fn new(config: &HelpConfig, views: &[Box<dyn LauncherView>]) -> Self {
        let entries = views
            .iter()
            .filter(|v| v.show_in_help())
            .map(|v| HelpEntry {
                prefix: v.prefix().to_string(),
                icon: v.icon().to_string(),
                name: v.name().to_string(),
                description: v.description().to_string(),
            })
            .collect();

        HelpView {
            prefix: config.prefix.clone(),
            entries,
        }
    }

    fn render_system_info(&self, _vx: &ViewContext, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let sysinfo = AppState::sysinfo(cx).get();
        let upower = AppState::upower(cx).get();

        let cpu_usage = sysinfo.cpu_usage;
        let memory_usage = sysinfo.memory_usage;
        let cpu_color = theme.colors.status.from_percentage(cpu_usage);
        let memory_color = theme.colors.status.from_percentage(memory_usage);

        let cpu_icon = if cpu_usage >= 90 {
            icons::CPU_HIGH
        } else {
            icons::CPU
        };

        let temp_text = sysinfo
            .temperature
            .map(|t| format!("{}°C", t))
            .unwrap_or_else(|| "—".to_string());

        let battery_icon = if let Some(ref battery) = upower.battery {
            battery.icon()
        } else {
            "󰂃"
        };

        let battery_text = if let Some(ref battery) = upower.battery {
            format!("{}%", battery.percentage)
        } else {
            String::new()
        };

        let text_muted = theme.colors.text_muted;

        div()
            .w_full()
            .px(Spacing::Large.pixels())
            .py(Spacing::Medium.pixels())
            .bg(theme.colors.surface_background)
            .rounded(px(8.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(IconSize::Small.pixels())
                            .text_color(cpu_color)
                            .child(cpu_icon),
                    )
                    .child(
                        div()
                            .text_size(TextSize::Small.rems())
                            .text_color(cpu_color)
                            .child(format!("{}%", cpu_usage)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(IconSize::Small.pixels())
                            .text_color(memory_color)
                            .child(icons::MEMORY),
                    )
                    .child(
                        div()
                            .text_size(TextSize::Small.rems())
                            .text_color(memory_color)
                            .child(format!("{}%", memory_usage)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(IconSize::Small.pixels())
                            .text_color(text_muted)
                            .child(icons::TEMP),
                    )
                    .child(
                        div()
                            .text_size(TextSize::Small.rems())
                            .text_color(text_muted)
                            .child(temp_text),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(IconSize::Small.pixels())
                            .text_color(text_muted)
                            .child(battery_icon),
                    )
                    .when(!battery_text.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(TextSize::Small.rems())
                                .text_color(text_muted)
                                .child(battery_text.clone()),
                        )
                    }),
            )
            .into_any_element()
    }

    pub fn selected_prefix(&self, index: usize, query: &str) -> Option<&str> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                if query.is_empty() {
                    return true;
                }
                entry.prefix.to_lowercase().contains(&query_lower)
                    || entry.name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .nth(index)
            .map(|e| e.prefix.as_str())
    }

    fn filtered_count(&self, query: &str) -> usize {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                if query.is_empty() {
                    return true;
                }
                entry.prefix.to_lowercase().contains(&query_lower)
                    || entry.name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .count()
    }
}

impl LauncherView for HelpView {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn name(&self) -> &'static str {
        "Help"
    }

    fn icon(&self) -> &'static str {
        "󰋗"
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn show_in_help(&self) -> bool {
        false
    }

    fn match_count(&self, vx: &ViewContext, _cx: &App) -> usize {
        self.filtered_count(vx.query)
    }

    fn render_item(&self, index: usize, selected: bool, vx: &ViewContext, cx: &App) -> AnyElement {
        let query_lower = vx.query.to_lowercase();
        let filtered: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| {
                if vx.query.is_empty() {
                    return true;
                }
                entry.prefix.to_lowercase().contains(&query_lower)
                    || entry.name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .collect();

        let Some(entry) = filtered.get(index) else {
            return div().into_any_element();
        };

        let theme = cx.theme();
        let interactive_default = theme.colors.element_background;

        ListItem::new(format!("cmd-{}", entry.prefix))
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(selected)
            .start_slot(
                div()
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .bg(interactive_default)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(TextSize::Large.rems())
                    .child(entry.icon.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(interactive_default)
                                    .child(
                                        Label::new(entry.prefix.clone())
                                            .size(TextSize::Small)
                                            .color(Color::Muted),
                                    ),
                            )
                            .child(Label::new(entry.name.clone()).size(TextSize::Default)),
                    )
                    .child(
                        Label::new(entry.description.clone())
                            .size(TextSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .into_any_element()
    }

    fn render_header(&self, vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        Some(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XLarge.pixels())
                .p(Spacing::Medium.pixels())
                .child(self.render_system_info(vx, cx))
                .child(
                    div().px(Spacing::Medium.pixels()).child(
                        Label::new("COMMANDS")
                            .size(TextSize::XSmall)
                            .color(Color::Disabled),
                    ),
                )
                .into_any_element(),
        )
    }

    fn render_footer(&self, _vx: &ViewContext, _cx: &App) -> Option<AnyElement> {
        Some(
            div()
                .px(Spacing::Medium.pixels())
                .pt(Spacing::Medium.pixels())
                .pb(Spacing::Medium.pixels())
                .flex()
                .flex_col()
                .gap(Spacing::XSmall.pixels())
                .child(
                    Label::new("USAGE")
                        .size(TextSize::XSmall)
                        .color(Color::Disabled),
                )
                .child(
                    Label::new("• Type a prefix (like @, $, !) to switch to that view")
                        .size(TextSize::Small)
                        .color(Color::Muted),
                )
                .child(
                    Label::new("• Type without prefix to search apps directly")
                        .size(TextSize::Small)
                        .color(Color::Muted),
                )
                .child(
                    Label::new("• Press ? anytime to return to this help")
                        .size(TextSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
        )
    }

    fn on_select(&self, _index: usize, _vx: &ViewContext, _cx: &mut App) -> bool {
        false
    }
}
