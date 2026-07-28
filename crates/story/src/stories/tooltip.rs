use crate::prelude::*;

pub struct TooltipStory;

impl Render for TooltipStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Tooltip variants (hover the buttons)",
            vec![
                example(
                    "Text only",
                    Button::new("tip-text", "Hover me")
                        .tooltip(Tooltip::text("This is a tooltip"))
                        .into_any_element(),
                ),
                example(
                    "With meta",
                    Button::new("tip-meta", "With meta")
                        .style(ButtonStyle::Subtle)
                        .tooltip(Tooltip::with_meta("Save file", "Ctrl+S"))
                        .into_any_element(),
                ),
                example(
                    "On IconButton",
                    IconButton::new("tip-icon", IconName::Settings)
                        .style(ButtonStyle::Subtle)
                        .tooltip(Tooltip::text("Settings"))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| TooltipStory).into()
}
