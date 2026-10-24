//! Help view listing every other view by prefix.

pub mod config;

use gpui::{AnyElement, App, Context, EventEmitter, SharedString, Window, div, prelude::*, px};
use ui::{
    ActiveTheme, Color, HighlightedLabel, Icon, IconName, IconSize, Label, LabelCommon, ListItem,
    ListItemSpacing, Spacing, TextSize, Toggleable,
};

use self::config::HelpConfig;
use crate::launcher::item::{Item, ItemIcon};
use crate::launcher::view::{Items, LauncherView, Outcome, ViewDescriptor, ViewEvent, ViewMeta};

pub fn descriptor(config: &HelpConfig, views: Vec<ViewMeta>) -> ViewDescriptor {
    let entries: Vec<ViewMeta> = views.into_iter().filter(|meta| meta.in_help).collect();
    ViewDescriptor::new(
        ViewMeta {
            id: "help",
            prefix: config.prefix.clone().into(),
            name: "Help",
            icon: IconName::CircleHelp,
            description: "Show available commands",
            default: false,
            in_help: false,
        },
        move |_| HelpView {
            entries: entries.clone(),
        },
    )
}

pub struct HelpView {
    entries: Vec<ViewMeta>,
}

impl EventEmitter<ViewEvent> for HelpView {}

impl LauncherView for HelpView {
    fn items(&mut self, _query: &str, _cx: &mut Context<Self>) -> Items {
        Items::Ready(
            self.entries
                .iter()
                .map(|meta| {
                    Item::new(meta.prefix.clone(), meta.name)
                        .subtitle(meta.description)
                        .keywords(meta.prefix.clone())
                        .icon(ItemIcon::Icon(meta.icon.into()))
                        .action("open", "Open")
                })
                .collect(),
        )
    }

    fn activate(&mut self, item: &Item, _action: &str, _cx: &mut Context<Self>) -> Outcome {
        Outcome::SetQuery(format!("{} ", item.id))
    }

    fn render_row(
        &self,
        item: &Item,
        selected: bool,
        _window: &Window,
        cx: &App,
    ) -> Option<AnyElement> {
        let slot = cx.theme().colors.element_background;
        let icon = self
            .entries
            .iter()
            .find(|meta| meta.prefix == item.id)
            .map_or(IconName::CircleHelp, |meta| meta.icon);
        let pill = |text: SharedString| {
            div()
                .px(px(6.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(slot)
                .child(Label::new(text).size(TextSize::Small).color(Color::Muted))
        };

        Some(
            ListItem::new(item.id.clone())
                .spacing(ListItemSpacing::Sparse)
                .toggle_state(selected)
                .start_slot(
                    div()
                        .size(px(28.))
                        .rounded(px(6.))
                        .bg(slot)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(icon).size(IconSize::Medium)),
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
                                .child(pill(item.id.clone()))
                                .child(
                                    HighlightedLabel::new(item.title.clone(), Vec::new())
                                        .size(TextSize::Default),
                                ),
                        )
                        .children(item.subtitle.clone().map(|subtitle| {
                            Label::new(subtitle)
                                .size(TextSize::Small)
                                .color(Color::Muted)
                        })),
                )
                .into_any_element(),
        )
    }

    fn render_header(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
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
}
