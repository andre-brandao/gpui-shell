//! Help view showing available launcher commands and system information.

use crate::launcher::view::{LIST_ITEM_HEIGHT, LauncherView, ViewContext};
use crate::widgets::sysinfo::icons;
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px, rgba};
use ui::{bg, font_size, icon_size, spacing, status, text};

/// Help view - shows available commands and system status.
pub struct HelpView {
    entries: Vec<HelpEntry>,
}

struct HelpEntry {
    prefix: String,
    icon: String,
    name: String,
    description: String,
}

impl HelpView {
    /// Create a new help view from the available launcher views.
    pub fn new(views: &[Box<dyn LauncherView>]) -> Self {
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

        HelpView { entries }
    }

    fn render_system_info(&self, vx: &ViewContext) -> AnyElement {
        let sysinfo = vx.services.sysinfo.get();
        let upower = vx.services.upower.get();

        let cpu_usage = sysinfo.cpu_usage;
        let memory_usage = sysinfo.memory_usage;
        let cpu_color = status::from_percentage(cpu_usage);
        let memory_color = status::from_percentage(memory_usage);

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
            "󰂃" // No battery icon
        };

        let battery_text = if let Some(ref battery) = upower.battery {
            format!("{}%", battery.percentage)
        } else {
            String::new()
        };

        // Single compact row with system info
        div()
            .w_full()
            .px(px(spacing::MD))
            .py(px(spacing::SM))
            .bg(bg::secondary())
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
                            .text_size(px(icon_size::MD))
                            .text_color(cpu_color)
                            .child(cpu_icon),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
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
                            .text_size(px(icon_size::MD))
                            .text_color(memory_color)
                            .child(icons::MEMORY),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
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
                            .text_size(px(icon_size::MD))
                            .text_color(text::muted())
                            .child(icons::TEMP),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child(temp_text),
                    ),
            )
            // Battery
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(icon_size::MD))
                            .text_color(text::muted())
                            .child(battery_icon),
                    )
                    .when(!battery_text.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(px(font_size::SM))
                                .text_color(text::muted())
                                .child(battery_text.clone()),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_commands(&self, vx: &ViewContext) -> AnyElement {
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

        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(filtered.into_iter().enumerate().map(|(i, entry)| {
                let is_selected = i == vx.selected_index;

                div()
                    .id(format!("cmd-{}", entry.prefix))
                    .w_full()
                    .h(px(LIST_ITEM_HEIGHT))
                    .px(px(spacing::MD))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .when(is_selected, |el| el.bg(rgba(0x3b82f6ff)))
                    .when(!is_selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::MD))
                            .child(
                                div()
                                    .w(px(32.))
                                    .h(px(32.))
                                    .rounded(px(6.))
                                    .bg(rgba(0x444444ff))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(font_size::LG))
                                    .child(entry.icon.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(spacing::SM))
                                            .child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(2.))
                                                    .rounded(px(4.))
                                                    .bg(rgba(0x555555ff))
                                                    .text_size(px(font_size::SM))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(entry.prefix.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(font_size::BASE))
                                                    .text_color(text::primary())
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(entry.name.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(font_size::SM))
                                            .text_color(text::muted())
                                            .child(entry.description.clone()),
                                    ),
                            ),
                    )
            }))
            .into_any_element()
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

    /// Get a filtered entry by index.
    fn get_filtered_entry(&self, query: &str, index: usize) -> Option<&HelpEntry> {
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
    }
}

impl LauncherView for HelpView {
    fn prefix(&self) -> &'static str {
        "?"
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

    fn show_in_help(&self) -> bool {
        // Don't show help in its own list (it would be redundant)
        false
    }

    fn render(&self, vx: &ViewContext, _cx: &App) -> (AnyElement, usize) {
        let count = self.filtered_count(vx.query);

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(spacing::LG))
            .p(px(spacing::SM))
            // System info header
            .child(self.render_system_info(vx))
            // Section title
            .child(
                div()
                    .px(px(spacing::SM))
                    .text_size(px(font_size::XS))
                    .text_color(text::disabled())
                    .font_weight(FontWeight::MEDIUM)
                    .child("COMMANDS"),
            )
            // Commands list
            .child(self.render_commands(vx))
            // Usage hint
            .child(
                div()
                    .px(px(spacing::SM))
                    .pt(px(spacing::SM))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(text::disabled())
                            .font_weight(FontWeight::MEDIUM)
                            .child("USAGE"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Type a prefix (like @, $, !) to switch to that view"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Type without prefix to search apps directly"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Press ? anytime to return to this help"),
                    ),
            )
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, _vx: &ViewContext, _cx: &mut App) -> bool {
        // When selecting a command, we don't close - the launcher will handle
        // switching to the selected view's prefix
        if let Some(_entry) = self.get_filtered_entry(_vx.query, index) {
            // Return false to not close; the launcher's execute_selected
            // will handle switching to the view
        }
        false
    }
}
