use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct HoverCardStory;

impl Render for HoverCardStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card = cx.new(|_| {
            HoverCard::new(|_, _| {
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(Label::new("Alice Smith").size(TextSize::Default))
                    .child(Label::new("Senior Engineer at Acme").color(Color::Muted))
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .child(
                                Icon::new(IconName::Mail)
                                    .size(IconSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(
                                Label::new("alice@example.com")
                                    .size(TextSize::Small)
                                    .color(Color::Muted),
                            ),
                    )
                    .into_any_element()
            })
            .title("Preview")
            .min_width(px(220.0))
        });

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "HoverCard",
            vec![
                example("Rich content card", card.into_any_element()),
                example(
                    "Minimal card",
                    cx.new(|_| {
                        HoverCard::new(|_, _| {
                            Label::new("https://example.com")
                                .color(Color::Accent)
                                .into_any_element()
                        })
                        .title("Link Preview")
                    })
                    .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| HoverCardStory).into()
}
