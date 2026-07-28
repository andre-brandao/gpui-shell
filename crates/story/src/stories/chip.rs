use crate::prelude::*;

pub struct ChipStory;

impl Render for ChipStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Chip styles",
                vec![
                    example("Default", Chip::new("Default").into_any_element()),
                    example(
                        "Accent",
                        Chip::new("Accent")
                            .style(ChipStyle::Accent)
                            .into_any_element(),
                    ),
                    example(
                        "Success",
                        Chip::new("Success")
                            .style(ChipStyle::Success)
                            .into_any_element(),
                    ),
                    example(
                        "Warning",
                        Chip::new("Warning")
                            .style(ChipStyle::Warning)
                            .into_any_element(),
                    ),
                    example(
                        "Error",
                        Chip::new("Error")
                            .style(ChipStyle::Error)
                            .into_any_element(),
                    ),
                    example(
                        "Info",
                        Chip::new("Info").style(ChipStyle::Info).into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Chip sizes",
                vec![
                    example(
                        "Small",
                        Chip::new("Small").size(ChipSize::Small).into_any_element(),
                    ),
                    example(
                        "Medium (default)",
                        Chip::new("Medium")
                            .size(ChipSize::Medium)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Outline mode",
                vec![
                    example(
                        "Success",
                        Chip::new("Success")
                            .style(ChipStyle::Success)
                            .outline(true)
                            .into_any_element(),
                    ),
                    example(
                        "Error",
                        Chip::new("Error")
                            .style(ChipStyle::Error)
                            .outline(true)
                            .into_any_element(),
                    ),
                    example(
                        "Accent",
                        Chip::new("Accent")
                            .style(ChipStyle::Accent)
                            .outline(true)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "CountBadge",
                vec![example(
                    "Various counts",
                    h_flex()
                        .gap(Spacing::Medium.pixels())
                        .items_center()
                        .child(CountBadge::new(3))
                        .child(CountBadge::new(42))
                        .child(CountBadge::new(150))
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| ChipStory).into()
}
