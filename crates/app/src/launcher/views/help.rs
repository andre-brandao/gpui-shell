//! Help view — lists available launcher commands and system information.

use gpui::{AnyElement, App, Context, EventEmitter};
use services::Services;
use ui::{ActiveTheme, Color, Icon, Label, LabelSize, ListItem, ListItemSpacing, prelude::*};

use crate::launcher::view::{LauncherView, ViewEvent, ViewMeta};
use crate::widgets::sysinfo::icons;

pub struct HelpView {
    services: Services,
    entries: Vec<ViewMeta>,
    query: String,
    filtered_indices: Vec<usize>,
}

impl EventEmitter<ViewEvent> for HelpView {}

impl HelpView {
    pub fn new(entries: Vec<ViewMeta>, services: Services) -> Self {
        let filtered_indices = (0..entries.len()).collect();
        Self {
            services,
            entries,
            query: String::new(),
            filtered_indices,
        }
    }

    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if q.is_empty() {
                    return e.show_in_help;
                }
                e.show_in_help
                    && (e.prefix.to_lowercase().contains(&q)
                        || e.name.to_lowercase().contains(&q)
                        || e.description.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect();
    }
}

impl LauncherView for HelpView {
    fn id(&self) -> &'static str {
        "help"
    }

    fn prefix(&self) -> &'static str {
        "?"
    }

    fn name(&self) -> &'static str {
        "Help"
    }

    fn icon(&self) -> IconName {
        IconName::CircleHelp
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn show_in_help(&self) -> bool {
        false
    }

    fn match_count(&self) -> usize {
        self.filtered_indices.len()
    }

    fn set_query(&mut self, query: &str, _cx: &mut Context<Self>) {
        self.query = query.to_string();
        self.refilter();
    }

    fn render_header(&self, cx: &App) -> Option<AnyElement> {
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        let sysinfo = self.services.sysinfo.get();
        let upower = self.services.upower.get();

        let cpu = sysinfo.cpu_usage;
        let mem = sysinfo.memory_usage;

        let cpu_color = if cpu >= 90 {
            status.error
        } else if cpu >= 70 {
            status.warning
        } else {
            status.success
        };

        let mem_color = if mem >= 90 {
            status.error
        } else if mem >= 70 {
            status.warning
        } else {
            status.success
        };

        let cpu_icon = if cpu >= 90 {
            icons::CPU_HIGH
        } else {
            icons::CPU
        };

        let temp_text = sysinfo
            .temperature
            .map(|t| format!("{}°C", t))
            .unwrap_or_else(|| "\u{2014}".to_string());

        let battery_icon = upower
            .battery
            .as_ref()
            .map(|b| b.icon())
            .unwrap_or("\u{f0079}");
        let battery_text = upower
            .battery
            .as_ref()
            .map(|b| format!("{}%", b.percentage))
            .unwrap_or_default();

        Some(
            div()
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .child(
                    div()
                        .w_full()
                        .px(px(12.))
                        .py(px(8.))
                        .bg(colors.surface_background)
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
                                        .text_size(px(16.))
                                        .text_color(cpu_color)
                                        .child(cpu_icon),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(cpu_color)
                                        .child(format!("{}%", cpu)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .text_color(mem_color)
                                        .child(icons::MEMORY),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(mem_color)
                                        .child(format!("{}%", mem)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .text_color(colors.text_muted)
                                        .child(icons::TEMP),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(colors.text_muted)
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
                                        .text_size(px(16.))
                                        .text_color(colors.text_muted)
                                        .child(battery_icon),
                                )
                                .when(!battery_text.is_empty(), |el| {
                                    el.child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(colors.text_muted)
                                            .child(battery_text.clone()),
                                    )
                                }),
                        ),
                )
                .into_any_element(),
        )
    }

    fn render_item(&self, index: usize, selected: bool, cx: &App) -> AnyElement {
        let Some(&entry_idx) = self.filtered_indices.get(index) else {
            return gpui::Empty.into_any_element();
        };
        let entry = &self.entries[entry_idx];
        let colors = cx.theme().colors();

        ListItem::new(format!("cmd-{}", entry.prefix))
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(selected)
            .start_slot(
                div()
                    .w(px(32.))
                    .h(px(32.))
                    .rounded(px(6.))
                    .bg(colors.element_background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(16.))
                    .child(Icon::new(entry.icon).color(Color::Default)),
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
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(colors.element_background)
                                    .child(
                                        Label::new(entry.prefix)
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    ),
                            )
                            .child(Label::new(entry.name).size(LabelSize::Default)),
                    )
                    .child(
                        Label::new(entry.description)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .into_any_element()
    }

    fn render_footer(&self, _cx: &App) -> Option<AnyElement> {
        Some(
            div()
                .w_full()
                .px(px(12.))
                .pt(px(8.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    Label::new("USAGE")
                        .size(LabelSize::XSmall)
                        .color(Color::Disabled),
                )
                .child(
                    Label::new("\u{2022} Type a prefix (like @, $, !) to switch to that view")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .child(
                    Label::new("\u{2022} Type without prefix to search apps directly")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .child(
                    Label::new("\u{2022} Press ? anytime to return to this help")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
        )
    }

    fn confirm(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(&entry_idx) = self.filtered_indices.get(index) {
            let prefix = self.entries[entry_idx].prefix;
            cx.emit(ViewEvent::SwitchTo(prefix.to_string()));
        }
    }
}
