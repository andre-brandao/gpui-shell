//! Slider - a draggable range input for selecting a numeric value.
//!
//! The slider is stateless (`RenderOnce`): the parent owns the value and
//! receives changes via a handler. Drag interaction uses a `canvas` overlay
//! with `on_mouse_down` / `on_mouse_up` / `on_mouse_move` to track pointer
//! position, computing the value from the horizontal offset within the
//! track bounds.

use std::cell::Cell;
use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{
    App, Bounds, BoxShadow, ElementId, IntoElement, MouseButton, MouseMoveEvent, Pixels,
    RenderOnce, SharedString, Styled, Window, canvas, div, point, prelude::*, px, relative, size,
};

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::Disableable;
use crate::traits::handlers::F32Handler;

/// A horizontal slider for selecting a numeric value within a range.
#[derive(IntoElement)]
#[must_use = "Slider does nothing unless rendered"]
pub struct Slider {
    id: ElementId,
    value: f32,
    min: f32,
    max: f32,
    step: Option<f32>,
    disabled: bool,
    label: Option<SharedString>,
    show_value: bool,
    on_change: Option<F32Handler>,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>, value: f32) -> Self {
        Self {
            id: id.into(),
            value,
            min: 0.0,
            max: 100.0,
            step: None,
            disabled: false,
            label: None,
            show_value: false,
            on_change: None,
        }
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show the current numeric value next to the slider.
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Register a change handler, invoked with the new value when dragged.
    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Disableable for Slider {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let range = self.max - self.min;
        let fraction = if range > 0.0 {
            ((self.value - self.min) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let track_bg = if self.disabled {
            colors.element_disabled
        } else {
            colors.element_background
        };
        let fill_bg = if self.disabled {
            colors.text_disabled
        } else {
            colors.accent
        };
        let thumb_bg = if self.disabled {
            colors.text_disabled
        } else {
            colors.background
        };
        let thumb_border = if self.disabled {
            colors.border_variant
        } else {
            colors.border_focused
        };
        let label_color = if self.disabled {
            Color::Disabled
        } else {
            Color::Default
        };

        let track_height = px(4.0);
        let thumb_size = px(12.0);
        let ring_color = colors.border_focused.opacity(0.25);

        // Capture the track bounds during paint so the click handler can
        // compute a value from the pointer's X position.
        let track_bounds: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(1.0), px(1.0)),
        }));

        let paint_bounds = track_bounds.clone();

        // The interaction area is intentionally taller than the visual
        // track so the user can drag vertically without losing the cursor
        // - same approach as HTML <input type="range">. The visible 6px
        // track is centered inside via flex + items_center.
        let track = div()
            .id(self.id.clone())
            .w_full()
            .h(px(40.0))
            .flex()
            .items_center()
            .child(
                div()
                    .w_full()
                    .h(track_height)
                    .rounded_full()
                    .bg(track_bg)
                    .relative()
                    // Invisible canvas that captures its own bounds during paint.
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                paint_bounds.set(bounds);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    // Filled portion
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .h_full()
                            .w(relative(fraction))
                            .rounded_full()
                            .bg(fill_bg),
                    )
                    // Thumb - shadcn-style: small white circle with a ring
                    // halo on hover/active for affordance.
                    .child(
                        div()
                            .absolute()
                            .top(-(thumb_size - track_height) / 2.0)
                            .left(relative(fraction))
                            .ml(-thumb_size / 2.0)
                            .size(thumb_size)
                            .rounded_full()
                            .bg(thumb_bg)
                            .border_1()
                            .border_color(thumb_border)
                            .when(!self.disabled, |this| {
                                this.cursor_pointer().hover(|s| {
                                    s.border_color(colors.accent).shadow(vec![BoxShadow {
                                        color: ring_color,
                                        offset: point(px(0.), px(0.)),
                                        blur_radius: px(0.),
                                        spread_radius: px(3.),
                                        inset: false,
                                    }])
                                })
                            }),
                    ),
            )
            .when_some(
                (!self.disabled).then_some(self.on_change).flatten(),
                |this, handler| {
                    let min = self.min;
                    let max = self.max;
                    let step = self.step;
                    let bounds_for_click = track_bounds.clone();
                    let bounds_for_move = track_bounds;
                    let move_handler = handler.clone();
                    this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        let b = bounds_for_click.get();
                        let w = b.size.width.max(px(1.0));
                        let x = event.position.x - b.origin.x;
                        let frac = (x / w).clamp(0.0, 1.0);
                        let val = value_from_fraction(frac, min, max, step);
                        handler(val, window, cx);
                    })
                    .on_mouse_move(
                        move |event: &MouseMoveEvent, window, cx| {
                            if event.pressed_button == Some(MouseButton::Left) {
                                let b = bounds_for_move.get();
                                let w = b.size.width.max(px(1.0));
                                let x = event.position.x - b.origin.x;
                                let frac = (x / w).clamp(0.0, 1.0);
                                let val = value_from_fraction(frac, min, max, step);
                                move_handler(val, window, cx);
                            }
                        },
                    )
                },
            );

        let value_label = self.show_value.then(|| {
            let text = if self.step.is_some_and(|s| s == s.round()) {
                format!("{}", self.value as i64)
            } else {
                format!("{:.1}", self.value)
            };
            Label::new(text).size(TextSize::Small).color(label_color)
        });

        h_flex()
            .gap(Spacing::Small.pixels())
            .w_full()
            .when_some(self.label, |this, label| {
                this.child(Label::new(label).size(TextSize::Small).color(label_color))
            })
            .child(track)
            .when_some(value_label, |this, label| this.child(label))
    }
}

/// Map a 0..=1 track fraction to a clamped, step-snapped slider value.
///
/// `frac` is pre-clamped by the caller; this fn handles step rounding and a
/// final min/max clamp so rounding can never push the value past the range.
fn value_from_fraction(frac: f32, min: f32, max: f32, step: Option<f32>) -> f32 {
    let range = max - min;
    let mut val = min + frac * range;
    if let Some(s) = step
        && s > 0.0
    {
        val = (val / s).round() * s;
    }
    val.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_fraction_returns_min() {
        assert_eq!(value_from_fraction(0.0, 0.0, 100.0, None), 0.0);
    }

    #[test]
    fn full_fraction_returns_max() {
        assert_eq!(value_from_fraction(1.0, 0.0, 100.0, None), 100.0);
    }

    #[test]
    fn mid_fraction_returns_midpoint() {
        assert_eq!(value_from_fraction(0.5, 0.0, 100.0, None), 50.0);
    }

    #[test]
    fn step_snaps_to_nearest_increment() {
        assert_eq!(value_from_fraction(0.27, 0.0, 100.0, Some(10.0)), 30.0);
        assert_eq!(value_from_fraction(0.22, 0.0, 100.0, Some(10.0)), 20.0);
    }

    #[test]
    fn step_rounding_stays_within_range() {
        // A step that doesn't divide the range evenly: rounding could push
        // the value past `max`, so clamp catches it.
        let val = value_from_fraction(1.0, 0.0, 100.0, Some(30.0));
        assert!(val <= 100.0);
    }

    #[test]
    fn negative_range_reversed_bounds_still_clamp() {
        let val = value_from_fraction(0.5, -50.0, 50.0, None);
        assert_eq!(val, 0.0);
    }

    #[test]
    fn zero_range_returns_min() {
        assert_eq!(value_from_fraction(0.5, 42.0, 42.0, None), 42.0);
    }

    #[test]
    fn invalid_step_ignored() {
        // Step <= 0 should not divide-by-zero; it's simply ignored.
        assert_eq!(value_from_fraction(0.5, 0.0, 100.0, Some(0.0)), 50.0);
    }
}
