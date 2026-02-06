//! Theme view — browse and apply bundled Zed themes.

use gpui::{AnyElement, App, Context, EventEmitter, SharedString};
use settings::{ParseStatus, SettingsStore};
use theme::{Appearance, SystemAppearance, ThemeRegistry};
use ui::{ActiveTheme, Color, Icon, Label, LabelSize, ListItem, ListItemSpacing, prelude::*};

use crate::launcher::view::{FooterAction, LauncherView, ViewEvent};

pub struct ThemesView {
    entries: Vec<ThemeEntry>,
    filtered: Vec<usize>,
    query: String,
}

#[derive(Clone)]
struct ThemeEntry {
    name: SharedString,
    appearance: Appearance,
}

impl EventEmitter<ViewEvent> for ThemesView {}

impl ThemesView {
    pub fn new(cx: &App) -> Self {
        let entries = load_theme_entries(cx);
        let filtered = (0..entries.len()).collect();
        Self {
            entries,
            filtered,
            query: String::new(),
        }
    }

    fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if query.is_empty() {
                    return true;
                }

                let appearance_label = appearance_label(entry.appearance).to_lowercase();
                entry.name.to_lowercase().contains(&query) || appearance_label.contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
    }
}

impl LauncherView for ThemesView {
    fn id(&self) -> &'static str {
        "themes"
    }

    fn prefix(&self) -> &'static str {
        "~"
    }

    fn name(&self) -> &'static str {
        "Themes"
    }

    fn icon(&self) -> IconName {
        IconName::Sparkle
    }

    fn description(&self) -> &'static str {
        "Browse and apply Zed themes"
    }

    fn match_count(&self) -> usize {
        self.filtered.len()
    }

    fn set_query(&mut self, query: &str, _cx: &mut Context<Self>) {
        self.query = query.to_string();
        self.refilter();
    }

    fn render_item(&self, index: usize, selected: bool, cx: &App) -> AnyElement {
        let Some(&entry_index) = self.filtered.get(index) else {
            return gpui::Empty.into_any_element();
        };
        let entry = &self.entries[entry_index];
        let colors = cx.theme().colors();
        let current_theme_name = cx.theme().name.clone();
        let is_active = current_theme_name == entry.name;

        let theme_name_for_click = entry.name.clone();
        let appearance_for_click = entry.appearance;

        ListItem::new(format!("theme-{index}"))
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
                    .child(Icon::new(IconName::Sparkle).color(Color::Muted)),
            )
            .on_click(move |_, _, cx| {
                apply_theme(theme_name_for_click.clone(), appearance_for_click, cx);
            })
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
                            .child(Label::new(entry.name.clone()).size(LabelSize::Default))
                            .when(is_active, |element| {
                                element.child(
                                    div()
                                        .px(px(6.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(colors.ghost_element_selected)
                                        .child(
                                            Label::new("Active")
                                                .size(LabelSize::XSmall)
                                                .color(Color::Muted),
                                        ),
                                )
                            }),
                    )
                    .child(
                        Label::new(format!("{} theme", appearance_label(entry.appearance)))
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .into_any_element()
    }

    fn confirm(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(&entry_index) = self.filtered.get(index) else {
            return;
        };
        let entry = &self.entries[entry_index];
        apply_theme(entry.name.clone(), entry.appearance, cx);
    }

    fn footer_actions(&self) -> Vec<FooterAction> {
        vec![
            FooterAction {
                label: "Apply",
                key: "Enter",
            },
            FooterAction {
                label: "Close",
                key: "Esc",
            },
        ]
    }
}

fn load_theme_entries(cx: &App) -> Vec<ThemeEntry> {
    let mut entries: Vec<ThemeEntry> = ThemeRegistry::global(cx)
        .list()
        .into_iter()
        .map(|theme| ThemeEntry {
            name: theme.name,
            appearance: theme.appearance,
        })
        .collect();

    entries.sort_by(|left, right| {
        let left_rank = appearance_rank(left.appearance);
        let right_rank = appearance_rank(right.appearance);
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.name.cmp(&right.name))
    });

    entries
}

fn appearance_rank(appearance: Appearance) -> u8 {
    match appearance {
        Appearance::Dark => 0,
        Appearance::Light => 1,
    }
}

fn appearance_label(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::Dark => "Dark",
        Appearance::Light => "Light",
    }
}

fn apply_theme(theme_name: SharedString, theme_appearance: Appearance, cx: &mut App) {
    cx.update_global::<SettingsStore, _>(move |store, cx| {
        let mut user_settings = store.raw_user_settings().cloned().unwrap_or_default();

        theme::set_theme(
            user_settings.content.as_mut(),
            theme_name.to_string(),
            theme_appearance,
            *SystemAppearance::global(cx),
        );

        let serialized = match serde_json::to_string(&user_settings) {
            Ok(serialized) => serialized,
            Err(error) => {
                tracing::error!("Failed to serialize user settings: {}", error);
                return;
            }
        };

        let parse_result = store.set_user_settings(&serialized, cx);
        if let ParseStatus::Failed { error } = parse_result.parse_status {
            tracing::error!("Failed to apply theme settings: {}", error);
        }
    });
}
