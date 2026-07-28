//! Radio - a two-state circle toggle for mutually exclusive choices.
//!
//! Follows the same idiom as [`Checkbox`](super::checkbox) and
//! [`Switch`](super::switch): stateless `RenderOnce`, parent owns the
//! selected state, handler receives the *new* state after the click. The
//! visual is a filled circle inside a bordered ring.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px};

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::{Disableable, ToggleHandler, ToggleState, Toggleable};

/// A radio button with optional inline label.
#[derive(IntoElement)]
#[must_use = "Radio does nothing unless rendered"]
pub struct Radio {
    id: ElementId,
    state: ToggleState,
    disabled: bool,
    label: Option<SharedString>,
    on_click: Option<ToggleHandler>,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>, state: impl Into<ToggleState>) -> Self {
        Self {
            id: id.into(),
            state: state.into(),
            disabled: false,
            label: None,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Register a click handler. The handler receives the new [`ToggleState`],
    /// always [`ToggleState::Selected`], since a radio can only be turned
    /// *on* by clicking; the parent is responsible for deselecting siblings.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Disableable for Radio {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for Radio {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.state = state.into();
        self
    }
}

impl RenderOnce for Radio {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let selected = self.state.selected();

        let ring_border = if self.disabled {
            colors.border_variant
        } else if selected {
            colors.accent
        } else {
            colors.border
        };

        let ring_bg = if self.disabled {
            colors.element_disabled
        } else {
            colors.element_background
        };

        let dot_color = if self.disabled {
            colors.text_disabled
        } else {
            colors.accent
        };

        let label_color = if self.disabled {
            Color::Disabled
        } else {
            Color::Default
        };

        let outer = px(16.0);
        let dot = px(8.0);

        let radio_circle = div()
            .id(self.id.clone())
            .size(outer)
            .rounded_full()
            .border_1()
            .border_color(ring_border)
            .bg(ring_bg)
            .flex()
            .items_center()
            .justify_center()
            .when(!self.disabled, |this| {
                this.cursor_pointer()
                    .hover(|s| s.border_color(colors.border_focused))
            })
            .when(selected, |this| {
                this.child(div().size(dot).rounded_full().bg(dot_color))
            })
            .when_some(
                (!self.disabled).then_some(self.on_click).flatten(),
                |this, handler| {
                    let next = ToggleState::Selected;
                    this.on_click(move |_event, window, cx| handler(&next, window, cx))
                },
            );

        h_flex()
            .gap(Spacing::Small.pixels())
            .child(radio_circle)
            .when_some(self.label, |this, label| {
                this.child(Label::new(label).size(TextSize::Small).color(label_color))
            })
    }
}
