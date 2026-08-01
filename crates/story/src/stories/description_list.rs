use crate::prelude::*;

pub struct DescriptionListStory;

impl Render for DescriptionListStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Basic",
                vec![example(
                    "Key-value pairs",
                    DescriptionList::new()
                        .label_width(120.0)
                        .entry("Name", Label::new("Alice"))
                        .entry("Email", Label::new("alice@example.com"))
                        .entry("Role", Label::new("Engineer"))
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "Bordered",
                vec![example(
                    "With separators",
                    DescriptionList::new()
                        .bordered(true)
                        .label_width(100.0)
                        .entry("Status", Chip::new("Active").style(ChipStyle::Success))
                        .entry("Plan", Label::new("Pro"))
                        .entry("Created", Label::new("2026-01-15"))
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| DescriptionListStory).into()
}
