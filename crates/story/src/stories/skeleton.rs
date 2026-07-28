use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct SkeletonStory;

impl Render for SkeletonStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Shapes",
                vec![
                    example("Rectangle (default)", Skeleton::new().into_any_element()),
                    example(
                        "Wide bar",
                        Skeleton::new()
                            .width(px(200.0))
                            .height(px(24.0))
                            .into_any_element(),
                    ),
                    example(
                        "Circle (avatar placeholder)",
                        Skeleton::circle(px(40.0)).into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Compositions",
                vec![
                    example("Text block", skeleton_text(4, px(220.0)).into_any_element()),
                    example(
                        "Card placeholder",
                        v_flex()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                h_flex()
                                    .gap(Spacing::Small.pixels())
                                    .child(Skeleton::circle(px(32.0)))
                                    .child(
                                        v_flex()
                                            .gap(px(4.0))
                                            .child(
                                                Skeleton::new().width(px(100.0)).height(px(12.0)),
                                            )
                                            .child(
                                                Skeleton::new().width(px(60.0)).height(px(10.0)),
                                            ),
                                    ),
                            )
                            .child(skeleton_text(3, px(200.0)))
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| SkeletonStory).into()
}
