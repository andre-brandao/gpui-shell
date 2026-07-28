use crate::prelude::*;

pub struct DividerStory;

impl Render for DividerStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Orientation",
            vec![
                example(
                    "Horizontal",
                    v_flex()
                        .gap(Spacing::Medium.pixels())
                        .w(px(320.0))
                        .child(Label::new("Above").color(Color::Muted))
                        .child(Divider::horizontal())
                        .child(Label::new("Below").color(Color::Muted))
                        .into_any_element(),
                ),
                example(
                    "Vertical",
                    h_flex()
                        .gap(Spacing::Medium.pixels())
                        .h(px(48.0))
                        .items_center()
                        .child(Label::new("Left").color(Color::Muted))
                        .child(Divider::vertical())
                        .child(Label::new("Right").color(Color::Muted))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| DividerStory).into()
}
