use crate::prelude::*;

pub struct IndicatorStory;

impl Render for IndicatorStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Dot",
                vec![example(
                    "Colors",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .items_center()
                        .child(Indicator::dot())
                        .child(Indicator::dot().color(Color::Success))
                        .child(Indicator::dot().color(Color::Warning))
                        .child(Indicator::dot().color(Color::Error))
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "Bar",
                vec![example(
                    "Accent bar",
                    v_flex()
                        .w(px(48.0))
                        .child(Indicator::bar().color(Color::Accent))
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "Icon",
                vec![example(
                    "Status icons",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .items_center()
                        .child(Indicator::icon(Icon::new(IconName::Check)).color(Color::Success))
                        .child(Indicator::icon(Icon::new(IconName::Close)).color(Color::Error))
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| IndicatorStory).into()
}
