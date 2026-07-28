use crate::prelude::*;

pub struct ButtonStory;

impl Render for ButtonStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Styles",
                vec![
                    example(
                        "Filled",
                        Button::new("filled", "Filled")
                            .style(ButtonStyle::Filled)
                            .into_any_element(),
                    ),
                    example(
                        "Subtle",
                        Button::new("subtle", "Subtle")
                            .style(ButtonStyle::Subtle)
                            .into_any_element(),
                    ),
                    example(
                        "Outlined",
                        Button::new("outlined", "Outlined")
                            .style(ButtonStyle::Outlined)
                            .into_any_element(),
                    ),
                    example(
                        "OutlinedGhost",
                        Button::new("outlined-ghost", "Outlined ghost")
                            .style(ButtonStyle::OutlinedGhost)
                            .into_any_element(),
                    ),
                    example(
                        "Transparent",
                        Button::new("transparent", "Transparent")
                            .style(ButtonStyle::Transparent)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Tints",
                vec![
                    example(
                        "Accent",
                        Button::new("tint-accent", "Accent")
                            .style(ButtonStyle::Tinted(TintColor::Accent))
                            .into_any_element(),
                    ),
                    example(
                        "Success",
                        Button::new("tint-success", "Success")
                            .style(ButtonStyle::Tinted(TintColor::Success))
                            .into_any_element(),
                    ),
                    example(
                        "Warning",
                        Button::new("tint-warning", "Warning")
                            .style(ButtonStyle::Tinted(TintColor::Warning))
                            .into_any_element(),
                    ),
                    example(
                        "Error",
                        Button::new("tint-error", "Error")
                            .style(ButtonStyle::Tinted(TintColor::Error))
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Sizes",
                vec![
                    example(
                        "Compact",
                        Button::new("compact", "Compact")
                            .size(ButtonSize::Compact)
                            .into_any_element(),
                    ),
                    example(
                        "Default",
                        Button::new("default", "Default").into_any_element(),
                    ),
                    example(
                        "Large",
                        Button::new("large", "Large")
                            .size(ButtonSize::Large)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "States & extras",
                vec![
                    example(
                        "With icon",
                        Button::new("icon", "Save")
                            .icon(IconName::Check)
                            .into_any_element(),
                    ),
                    example(
                        "Disabled",
                        Button::new("disabled", "Disabled")
                            .disabled(true)
                            .into_any_element(),
                    ),
                    example(
                        "Selected",
                        Button::new("selected", "Selected")
                            .style(ButtonStyle::Subtle)
                            .toggle_state(true)
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| ButtonStory).into()
}
