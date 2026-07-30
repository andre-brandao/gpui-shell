use crate::prelude::*;

pub struct ListStory {
    selected_nav: SharedString,
}

impl ListStory {
    fn new() -> Self {
        Self {
            selected_nav: SharedString::from("nav-search"),
        }
    }

    fn nav_item(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        weak: &gpui::WeakEntity<Self>,
    ) -> ListItem {
        let id_owned = SharedString::from(id);
        let is_selected = self.selected_nav == id_owned;
        let weak = weak.clone();
        ListItem::new(id)
            .start_slot(Icon::new(icon))
            .child(Label::new(label))
            .toggle_state(is_selected)
            .on_click(move |_event, _window, cx| {
                let id = id_owned.clone();
                weak.update(cx, |this, cx| {
                    this.selected_nav = id;
                    cx.notify();
                })
                .ok();
            })
    }
}

impl Render for ListStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Selectable list (click to select)",
                vec![example(
                    "Navigation",
                    v_flex()
                        .w(px(280.0))
                        .child(
                            List::new()
                                .header("Navigation")
                                .child(self.nav_item(
                                    "nav-home",
                                    IconName::ChevronRight,
                                    "Home",
                                    &weak,
                                ))
                                .child(self.nav_item(
                                    "nav-search",
                                    IconName::MagnifyingGlass,
                                    "Search",
                                    &weak,
                                ))
                                .child(self.nav_item(
                                    "nav-settings",
                                    IconName::Settings,
                                    "Settings",
                                    &weak,
                                ))
                                .child(
                                    ListItem::new("nav-disabled")
                                        .start_slot(Icon::new(IconName::Close))
                                        .child(Label::new("Unavailable"))
                                        .disabled(true),
                                ),
                        )
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "Empty list",
                vec![example(
                    "With empty message",
                    v_flex()
                        .w(px(280.0))
                        .child(
                            List::new()
                                .header("Recent")
                                .empty_message("No recent items"),
                        )
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "Tree list (indent, spacing, inset)",
                vec![example(
                    "File tree",
                    v_flex()
                        .w(px(320.0))
                        .child(
                            List::new()
                                .header("Project")
                                .child(
                                    ListItem::new("tree-src")
                                        .spacing(ListItemSpacing::Dense)
                                        .inset(true)
                                        .indent_level(0)
                                        .start_slot(Icon::new(IconName::Folder))
                                        .child(Label::new("src")),
                                )
                                .child(
                                    ListItem::new("tree-main")
                                        .spacing(ListItemSpacing::Dense)
                                        .inset(true)
                                        .indent_level(1)
                                        .start_slot(Icon::new(IconName::File))
                                        .child(Label::new("main.rs")),
                                )
                                .child(
                                    ListItem::new("tree-lib")
                                        .spacing(ListItemSpacing::Dense)
                                        .inset(true)
                                        .indent_level(1)
                                        .start_slot(Icon::new(IconName::Folder))
                                        .child(Label::new("lib")),
                                )
                                .child(
                                    ListItem::new("tree-mod")
                                        .spacing(ListItemSpacing::Dense)
                                        .inset(true)
                                        .indent_level(2)
                                        .start_slot(Icon::new(IconName::File))
                                        .child(Label::new("mod.rs"))
                                        .end_slot(Icon::new(IconName::Trash))
                                        .show_end_slot_on_hover(),
                                ),
                        )
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| ListStory::new()).into()
}
