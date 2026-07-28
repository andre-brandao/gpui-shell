use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct SliderStory {
    basic: f32,
    stepped: f32,
    small: f32,
}

impl SliderStory {
    fn new() -> Self {
        Self {
            basic: 50.0,
            stepped: 30.0,
            small: 0.5,
        }
    }
}

impl Render for SliderStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Interactive sliders",
                vec![
                    example("Continuous", {
                        let w = weak.clone();
                        Slider::new("sl-basic", self.basic)
                            .label("Volume")
                            .show_value(true)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.basic = val;
                                    cx.notify();
                                })
                                .ok();
                            })
                            .into_any_element()
                    }),
                    example("Stepped (10)", {
                        let w = weak.clone();
                        Slider::new("sl-stepped", self.stepped)
                            .min(0.0)
                            .max(100.0)
                            .step(10.0)
                            .label("Brightness")
                            .show_value(true)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.stepped = val;
                                    cx.notify();
                                })
                                .ok();
                            })
                            .into_any_element()
                    }),
                ],
            ))
            .child(example_group(
                "States",
                vec![
                    example(
                        "Disabled",
                        Slider::new("sl-dis", 60.0)
                            .label("Locked")
                            .show_value(true)
                            .disabled(true)
                            .into_any_element(),
                    ),
                    example("Custom range (0-1)", {
                        let w = weak.clone();
                        Slider::new("sl-small", self.small)
                            .min(0.0)
                            .max(1.0)
                            .show_value(true)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.small = val;
                                    cx.notify();
                                })
                                .ok();
                            })
                            .into_any_element()
                    }),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| SliderStory::new()).into()
}
