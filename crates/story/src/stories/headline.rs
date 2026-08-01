use crate::prelude::*;

pub struct HeadlineStory;

impl Render for HeadlineStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Sizes",
                vec![
                    example(
                        "XSmall",
                        Headline::new("XSmall headline")
                            .size(HeadlineSize::XSmall)
                            .into_any_element(),
                    ),
                    example(
                        "Small",
                        Headline::new("Small headline")
                            .size(HeadlineSize::Small)
                            .into_any_element(),
                    ),
                    example(
                        "Medium (default)",
                        Headline::new("Medium headline").into_any_element(),
                    ),
                    example(
                        "Large",
                        Headline::new("Large headline")
                            .size(HeadlineSize::Large)
                            .into_any_element(),
                    ),
                    example(
                        "XLarge",
                        Headline::new("XLarge headline")
                            .size(HeadlineSize::XLarge)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Colors",
                vec![
                    example("Default", Headline::new("Default").into_any_element()),
                    example(
                        "Muted",
                        Headline::new("Muted")
                            .color(Color::Muted)
                            .into_any_element(),
                    ),
                    example(
                        "Accent",
                        Headline::new("Accent")
                            .color(Color::Accent)
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| HeadlineStory).into()
}
