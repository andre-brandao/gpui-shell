use crate::prelude::*;

pub struct ButtonLinkStory;

impl Render for ButtonLinkStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "ButtonLink",
            vec![
                example(
                    "Default (with icon)",
                    ButtonLink::new("Learn more", "https://example.com").into_any_element(),
                ),
                example(
                    "Without icon",
                    ButtonLink::new("Privacy policy", "https://example.com/privacy")
                        .no_icon()
                        .into_any_element(),
                ),
                example(
                    "Colors",
                    v_flex()
                        .gap(Spacing::Small.pixels())
                        .child(ButtonLink::new("Default", "https://example.com"))
                        .child(
                            ButtonLink::new("Accent", "https://example.com")
                                .label_color(Color::Accent),
                        )
                        .child(
                            ButtonLink::new("Muted", "https://example.com")
                                .label_color(Color::Muted),
                        )
                        .into_any_element(),
                ),
                example(
                    "Sizes",
                    v_flex()
                        .gap(Spacing::Small.pixels())
                        .child(
                            ButtonLink::new("XSmall", "https://example.com")
                                .label_size(TextSize::XSmall),
                        )
                        .child(
                            ButtonLink::new("Small", "https://example.com")
                                .label_size(TextSize::Small),
                        )
                        .child(ButtonLink::new("Default", "https://example.com"))
                        .child(
                            ButtonLink::new("Large", "https://example.com")
                                .label_size(TextSize::Large),
                        )
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| ButtonLinkStory).into()
}
