use crate::prelude::*;

pub struct ProgressStory;

impl Render for ProgressStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "ProgressBar",
                vec![
                    example(
                        "Empty",
                        v_flex()
                            .w(px(300.0))
                            .child(ProgressBar::new(0.0, 100.0))
                            .into_any_element(),
                    ),
                    example(
                        "Half",
                        v_flex()
                            .w(px(300.0))
                            .child(ProgressBar::new(50.0, 100.0))
                            .into_any_element(),
                    ),
                    example(
                        "Full",
                        v_flex()
                            .w(px(300.0))
                            .child(ProgressBar::new(100.0, 100.0))
                            .into_any_element(),
                    ),
                    example(
                        "Over capacity",
                        v_flex()
                            .w(px(300.0))
                            .child(ProgressBar::new(120.0, 100.0))
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "CircularProgress",
                vec![
                    example(
                        "Quarter",
                        CircularProgress::new(0.25, 1.0, px(32.0)).into_any_element(),
                    ),
                    example(
                        "Half",
                        CircularProgress::new(0.5, 1.0, px(32.0)).into_any_element(),
                    ),
                    example(
                        "Three quarters",
                        CircularProgress::new(0.75, 1.0, px(32.0)).into_any_element(),
                    ),
                    example(
                        "Full",
                        CircularProgress::new(1.0, 1.0, px(32.0)).into_any_element(),
                    ),
                    example(
                        "Large",
                        CircularProgress::new(0.6, 1.0, px(48.0)).into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| ProgressStory).into()
}
