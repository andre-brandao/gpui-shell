use crate::prelude::*;

pub struct SpinnerStory;

impl Render for SpinnerStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Spinner",
            vec![
                example(
                    "Sizes",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .items_center()
                        .child(Spinner::new().size(IconSize::XSmall))
                        .child(Spinner::new().size(IconSize::Small))
                        .child(Spinner::new())
                        .child(Spinner::new().size(IconSize::XLarge))
                        .into_any_element(),
                ),
                example(
                    "Colors",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .items_center()
                        .child(Spinner::new().color(Color::Default))
                        .child(Spinner::new().color(Color::Muted))
                        .child(Spinner::new().color(Color::Accent))
                        .into_any_element(),
                ),
                example(
                    "With label",
                    h_flex()
                        .gap(Spacing::Small.pixels())
                        .items_center()
                        .child(Spinner::new().size(IconSize::Small))
                        .child(Label::new("Loading...").color(Color::Muted))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| SpinnerStory).into()
}
