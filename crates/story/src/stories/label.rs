use crate::prelude::*;

pub struct LabelStory;

impl Render for LabelStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Sizes",
                vec![
                    example(
                        "XSmall",
                        Label::new("XSmall")
                            .size(TextSize::XSmall)
                            .into_any_element(),
                    ),
                    example(
                        "Small",
                        Label::new("Small").size(TextSize::Small).into_any_element(),
                    ),
                    example("Default", Label::new("Default").into_any_element()),
                    example(
                        "Large",
                        Label::new("Large").size(TextSize::Large).into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Colors",
                vec![
                    example("Default", Label::new("Default").into_any_element()),
                    example(
                        "Muted",
                        Label::new("Muted").color(Color::Muted).into_any_element(),
                    ),
                    example(
                        "Accent",
                        Label::new("Accent").color(Color::Accent).into_any_element(),
                    ),
                    example(
                        "Success",
                        Label::new("Success")
                            .color(Color::Success)
                            .into_any_element(),
                    ),
                    example(
                        "Warning",
                        Label::new("Warning")
                            .color(Color::Warning)
                            .into_any_element(),
                    ),
                    example(
                        "Error",
                        Label::new("Error").color(Color::Error).into_any_element(),
                    ),
                    example(
                        "Disabled",
                        Label::new("Disabled")
                            .color(Color::Disabled)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Modifiers",
                vec![
                    example(
                        "Bold",
                        Label::new("Bold")
                            .weight(gpui::FontWeight::BOLD)
                            .into_any_element(),
                    ),
                    example("Italic", Label::new("Italic").italic().into_any_element()),
                    example(
                        "Underline",
                        Label::new("Underline").underline().into_any_element(),
                    ),
                    example(
                        "Strikethrough",
                        Label::new("Strikethrough")
                            .strikethrough()
                            .into_any_element(),
                    ),
                    example(
                        "Faded 50%",
                        Label::new("Faded 50%").alpha(0.5).into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| LabelStory).into()
}
