use crate::prelude::*;

pub struct SplitButtonStory;

impl Render for SplitButtonStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "SplitButton styles",
            vec![
                example(
                    "Filled (default)",
                    SplitButton::new(
                        IconButton::new("sb-left-1", IconName::Play),
                        IconButton::new("sb-right-1", IconName::ChevronDown),
                    )
                    .into_any_element(),
                ),
                example(
                    "Outlined",
                    SplitButton::new(
                        IconButton::new("sb-left-2", IconName::Save),
                        IconButton::new("sb-right-2", IconName::ChevronDown),
                    )
                    .style(SplitButtonStyle::Outlined)
                    .into_any_element(),
                ),
                example(
                    "Transparent",
                    SplitButton::new(
                        IconButton::new("sb-left-3", IconName::Settings),
                        IconButton::new("sb-right-3", IconName::ChevronDown),
                    )
                    .style(SplitButtonStyle::Transparent)
                    .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| SplitButtonStory).into()
}
