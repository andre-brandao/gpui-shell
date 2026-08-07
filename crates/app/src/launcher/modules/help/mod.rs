//! Help view showing available launcher commands.

pub mod config;

use gpui::{AnyElement, App, div, prelude::*, px};
use ui::{
    ActiveTheme, Color, Icon, IconName, IconSize, Label, LabelCommon, ListItem, ListItemSpacing,
    Spacing, TextSize, Toggleable,
};

use self::config::HelpConfig;
use crate::launcher::view::{LauncherView, ViewContext};

/// Help view - shows available commands.
pub struct HelpView {
    prefix: String,
    entries: Vec<HelpEntry>,
    /// Indices into `entries` matching the current query, refreshed once
    /// per frame by `update_matches`.
    matches: Vec<usize>,
}

struct HelpEntry {
    prefix: String,
    icon: IconName,
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
                icon: v.icon(),
                name: v.name().to_string(),
                description: v.description().to_string(),
            })
            .collect();

        HelpView {
            prefix: config.prefix.clone(),
            entries,
            matches: Vec::new(),
        }
    }

    /// Prefix of the entry at `index` in the current match list.
    pub fn selected_prefix(&self, index: usize) -> Option<&str> {
        self.entry(index).map(|e| e.prefix.as_str())
    }

    fn entry(&self, index: usize) -> Option<&HelpEntry> {
        self.entries.get(*self.matches.get(index)?)
    }
}

impl LauncherView for HelpView {
    fn prefix(&self) -> &str {
        &self.prefix
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

    fn update_matches(&mut self, vx: &ViewContext, _cx: &App) {
        let query_lower = vx.query.to_lowercase();
        self.matches = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query_lower.is_empty()
                    || entry.prefix.to_lowercase().contains(&query_lower)
                    || entry.name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .map(|(ix, _)| ix)
            .collect();
    }

    fn match_count(&self, _vx: &ViewContext, _cx: &App) -> usize {
        self.matches.len()
    }

    fn render_item(&self, index: usize, selected: bool, _vx: &ViewContext, cx: &App) -> AnyElement {
        let Some(entry) = self.entry(index) else {
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
                    .child(Icon::new(entry.icon).size(IconSize::Medium)),
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

    fn render_header(&self, _vx: &ViewContext, _cx: &App) -> Option<AnyElement> {
        Some(
            div()
                .p(Spacing::Medium.pixels())
                .px(Spacing::Large.pixels())
                .child(
                    Label::new("COMMANDS")
                        .size(TextSize::XSmall)
                        .color(Color::Disabled),
                )
                .into_any_element(),
        )
    }

    fn on_select(&self, _index: usize, _vx: &ViewContext, _cx: &mut App) -> bool {
        false
    }
}
