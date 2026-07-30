use crate::prelude::*;

pub struct GradientFadeStory;

impl Render for GradientFadeStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "GradientFade",
            vec![
                example(
                    "Right edge fade (hides overflowing content)",
                    gpui::div()
                        .relative()
                        .w(px(360.0))
                        .h(px(48.0))
                        .overflow_hidden()
                        .bg(colors.surface_background)
                        .child(
                            h_flex()
                                .gap(Spacing::Small.pixels())
                                .children((0..12).map(|i| {
                                    gpui::div()
                                        .flex_none()
                                        .px(Spacing::Small.pixels())
                                        .py(Spacing::XSmall.pixels())
                                        .mt(Spacing::Small.pixels())
                                        .ml(Spacing::XSmall.pixels())
                                        .rounded_sm()
                                        .bg(colors.element_background)
                                        .child(
                                            Label::new(format!("Tab {}", i + 1))
                                                .size(TextSize::Small),
                                        )
                                })),
                        )
                        .child(GradientFade::new(
                            colors.surface_background,
                            colors.surface_background,
                            colors.surface_background,
                        ))
                        .into_any_element(),
                ),
                example(
                    "Wide gradient",
                    gpui::div()
                        .relative()
                        .w(px(360.0))
                        .h(px(48.0))
                        .overflow_hidden()
                        .bg(colors.surface_background)
                        .child(
                            h_flex()
                                .gap(Spacing::Small.pixels())
                                .children((0..12).map(|i| {
                                    gpui::div()
                                        .flex_none()
                                        .px(Spacing::Small.pixels())
                                        .py(Spacing::XSmall.pixels())
                                        .mt(Spacing::Small.pixels())
                                        .ml(Spacing::XSmall.pixels())
                                        .rounded_sm()
                                        .bg(colors.element_background)
                                        .child(
                                            Label::new(format!("Item {}", i + 1))
                                                .size(TextSize::Small),
                                        )
                                })),
                        )
                        .child(
                            GradientFade::new(
                                colors.surface_background,
                                colors.surface_background,
                                colors.surface_background,
                            )
                            .width(px(120.0)),
                        )
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| GradientFadeStory).into()
}
