use crate::prelude::*;

pub struct TabStory {
    selected_tab: usize,
}

impl TabStory {
    fn new() -> Self {
        Self { selected_tab: 0 }
    }

    fn tab(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        index: usize,
        weak: &gpui::WeakEntity<Self>,
    ) -> Tab {
        let weak = weak.clone();
        Tab::new(id, label)
            .icon(icon)
            .toggle_state(self.selected_tab == index)
            .on_click(move |_event, _window, cx| {
                weak.update(cx, |this, cx| {
                    this.selected_tab = index;
                    cx.notify();
                })
                .ok();
            })
    }
}

impl Render for TabStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "TabBar (click to select)",
            vec![example(
                "Interactive tabs",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        TabBar::new()
                            .child(self.tab("tab-overview", IconName::Eye, "Overview", 0, &weak))
                            .child(self.tab("tab-files", IconName::File, "Files", 1, &weak))
                            .child(self.tab(
                                "tab-settings",
                                IconName::Settings,
                                "Settings",
                                2,
                                &weak,
                            )),
                    )
                    .child(
                        Label::new(match self.selected_tab {
                            0 => "Overview content",
                            1 => "Files content",
                            _ => "Settings content",
                        })
                        .color(Color::Muted),
                    )
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| TabStory::new()).into()
}
