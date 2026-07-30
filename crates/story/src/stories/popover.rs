use std::cell::Cell;
use std::rc::Rc;

use crate::prelude::*;
use gpui::{Bounds, FocusHandle, Pixels, canvas};

use crate::layout::{example, example_group};

pub struct PopoverStory {
    open: bool,
    focus: FocusHandle,
    trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl PopoverStory {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            open: false,
            focus: cx.focus_handle(),
            trigger_bounds: Rc::new(Cell::new(None)),
        }
    }
}

impl Render for PopoverStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let bounds_slot = self.trigger_bounds.clone();

        let trigger = Button::new("btn-popover", "Toggle popover")
            .style(ButtonStyle::Outlined)
            .on_click({
                let weak = weak.clone();
                move |_event, window, cx| {
                    weak.update(cx, |this, cx| {
                        this.open = !this.open;
                        if this.open {
                            window.focus(&this.focus, cx);
                        }
                        cx.notify();
                    })
                    .ok();
                }
            });

        let trigger_with_capture = gpui::div().relative().child(trigger).child(
            canvas(
                move |bounds, _window, _cx| {
                    bounds_slot.set(Some(bounds));
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0()
            .size_full(),
        );

        let is_open = self.open;
        let trigger_bounds = self.trigger_bounds.get();
        let focus = self.focus.clone();
        let weak_dismiss = weak.clone();

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Popover (click to toggle)",
            vec![example(
                "Anchored popover",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(trigger_with_capture)
                    .when(is_open, |this| {
                        let Some(bounds) = trigger_bounds else {
                            return this;
                        };
                        this.child(anchored_popover(
                            focus,
                            gpui::Anchor::TopLeft,
                            bounds,
                            Popover::new().child(
                                v_flex()
                                    .gap(Spacing::Small.pixels())
                                    .child(Label::new("Popover content"))
                                    .child(
                                        Label::new("This is a basic popover.")
                                            .color(Color::Muted)
                                            .size(TextSize::Small),
                                    ),
                            ),
                            move |_window, cx| {
                                weak_dismiss
                                    .update(cx, |this, cx| {
                                        this.open = false;
                                        cx.notify();
                                    })
                                    .ok();
                            },
                        ))
                    })
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(PopoverStory::new).into()
}
