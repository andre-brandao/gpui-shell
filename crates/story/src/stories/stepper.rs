use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct StepperStory {
    quantity: f64,
    decimal: f64,
}

impl StepperStory {
    fn new() -> Self {
        Self {
            quantity: 3.0,
            decimal: 1.5,
        }
    }
}

impl Render for StepperStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Interactive steppers",
                vec![
                    example("Integer", {
                        let w = weak.clone();
                        Stepper::new("stp-int", self.quantity)
                            .label("Quantity")
                            .min(0.0)
                            .max(20.0)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.quantity = val;
                                    cx.notify();
                                })
                                .ok();
                            })
                            .into_any_element()
                    }),
                    example("Decimal (step 0.5)", {
                        let w = weak.clone();
                        Stepper::new("stp-dec", self.decimal)
                            .label("Amount")
                            .min(0.0)
                            .max(10.0)
                            .step(0.5)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.decimal = val;
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
                        "At minimum",
                        Stepper::new("stp-min", 0.0)
                            .min(0.0)
                            .max(10.0)
                            .into_any_element(),
                    ),
                    example(
                        "At maximum",
                        Stepper::new("stp-max", 10.0)
                            .min(0.0)
                            .max(10.0)
                            .into_any_element(),
                    ),
                    example(
                        "Disabled",
                        Stepper::new("stp-dis", 5.0)
                            .disabled(true)
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| StepperStory::new()).into()
}
