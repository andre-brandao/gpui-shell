//! Motion - the shared springs and entrances, side by side so their feel can
//! be compared and tuned in one place.

use gpui::{AnimationExt, AnyElement, ElementId, SpringConfig, div};

use crate::prelude::*;

/// How far the spring demo travels.
const TRAVEL: f32 = 200.0;

pub struct MotionStory {
    /// Which end the spring demo targets.
    at_end: bool,
    /// Bumped to remount the entrance demo, so it plays again.
    generation: u64,
}

impl MotionStory {
    fn spring_row(&self, name: &'static str, config: SpringConfig, cx: &App) -> AnyElement {
        let colors = *cx.theme().colors();
        h_flex()
            .gap(Spacing::Medium.pixels())
            .items_center()
            .child(
                div()
                    .w(px(64.))
                    .child(Label::new(name).size(TextSize::Small).color(Color::Muted)),
            )
            .child(
                div()
                    .w(px(TRAVEL + 16.))
                    .h(px(16.))
                    .relative()
                    .rounded(Radius::Full.pixels())
                    .bg(colors.element_background)
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .size(px(16.))
                            .rounded(Radius::Full.pixels())
                            .bg(colors.accent)
                            .with_spring(
                                (ElementId::from("motion-spring"), name),
                                config.to(self.at_end),
                                |knob, phase| knob.left(phase.interpolate(px(0.), px(TRAVEL))),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn entrance(&self, name: &'static str, from: Enter, cx: &App) -> AnyElement {
        let colors = *cx.theme().colors();
        div()
            .w(px(96.))
            .h(px(56.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(Radius::Medium.pixels())
            .bg(colors.elevated_surface_background)
            .border_1()
            .border_color(colors.border)
            .child(Label::new(name).size(TextSize::Small))
            .enter(
                (ElementId::from(("motion-enter", self.generation)), name),
                from,
            )
            .into_any_element()
    }
}

impl Render for MotionStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let toggle_springs = {
            let weak = weak.clone();
            move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                weak.update(cx, |this, cx| {
                    this.at_end = !this.at_end;
                    cx.notify();
                })
                .ok();
            }
        };
        let replay = move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
            weak.update(cx, |this, cx| {
                this.generation += 1;
                cx.notify();
            })
            .ok();
        };

        v_flex()
            .gap(Spacing::XLarge.pixels())
            .child(example_group(
                "Springs",
                vec![
                    example(
                        "Presets",
                        v_flex()
                            .gap(Spacing::Medium.pixels())
                            .child(self.spring_row("Snappy", Motion::SNAPPY, cx))
                            .child(self.spring_row("Gentle", Motion::GENTLE, cx))
                            .child(self.spring_row("Lively", Motion::LIVELY, cx))
                            .child(
                                Button::new("motion-toggle", "Send across")
                                    .on_click(toggle_springs),
                            )
                            .into_any_element(),
                    )
                    .description(
                        "Press again mid-flight: a spring bends toward the new target \
                         instead of restarting. The OSD level and the dock hover use these.",
                    ),
                ],
            ))
            .child(example_group(
                "Entrances",
                vec![
                    example(
                        "Directions",
                        v_flex()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                h_flex()
                                    .gap(Spacing::Medium.pixels())
                                    .child(self.entrance("In place", Enter::InPlace, cx))
                                    .child(self.entrance("From above", Enter::FromAbove, cx))
                                    .child(self.entrance("From below", Enter::FromBelow, cx))
                                    .child(self.entrance("From left", Enter::FromLeft, cx))
                                    .child(self.entrance("From right", Enter::FromRight, cx)),
                            )
                            .child(Button::new("motion-replay", "Replay").on_click(replay))
                            .into_any_element(),
                    )
                    .description(
                        "Fade and settle over 180ms, once per id. The OSD enters this way.",
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| MotionStory {
        at_end: false,
        generation: 0,
    })
    .into()
}
