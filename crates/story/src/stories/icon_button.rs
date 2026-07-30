use crate::prelude::*;

pub struct IconButtonStory;

impl Render for IconButtonStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Styles",
                vec![
                    example(
                        "Filled",
                        IconButton::new("filled", IconName::Settings).into_any_element(),
                    ),
                    example(
                        "Subtle",
                        IconButton::new("subtle", IconName::MagnifyingGlass)
                            .style(ButtonStyle::Subtle)
                            .into_any_element(),
                    ),
                    example(
                        "Outlined",
                        IconButton::new("outlined", IconName::Plus)
                            .style(ButtonStyle::Outlined)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "States",
                vec![
                    example(
                        "Selected",
                        IconButton::new("selected", IconName::Star)
                            .style(ButtonStyle::Subtle)
                            .toggle_state(true)
                            .into_any_element(),
                    ),
                    example(
                        "Disabled",
                        IconButton::new("disabled", IconName::Close)
                            .disabled(true)
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| IconButtonStory).into()
}
