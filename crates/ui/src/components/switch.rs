//! Switch - a two-state toggle (on / off) with optional inline label.
//!
//! Modeled on Zed's `Switch`, trimmed to the essentials: state, disabled,
//! label, and click handler. Skips Zed's `SwitchColor`, `SwitchLabelPosition`,
//! `key_binding`, and tab-index plumbing - these can come back as needed.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px};

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::{Disableable, ToggleHandler, ToggleState, Toggleable};

#[derive(IntoElement)]
#[must_use = "Switch does nothing unless rendered"]
pub struct Switch {
    id: ElementId,
    state: ToggleState,
    disabled: bool,
    label: Option<SharedString>,
    on_click: Option<ToggleHandler>,
}

impl Switch {
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

    /// Register a click handler. The handler receives the **new** state
    /// produced by flipping the current one.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for Switch {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.state = state.into();
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let is_on = self.state.selected();

        let track_bg = if self.disabled {
            colors.element_disabled
        } else if is_on {
            colors.accent
        } else {
            colors.element_background
        };

        let track_border = if is_on { colors.accent } else { colors.border };

        let thumb_bg = if self.disabled {
            colors.text_disabled
        } else if is_on {
            colors.background
        } else {
            colors.text_muted
        };

        let label_color = if self.disabled {
            Color::Disabled
        } else {
            Color::Default
        };

        // Track / thumb dimensions: 28x16 track with a 12px thumb leaves
        // ~2px of padding on every side.
        let track_width = px(28.0);
        let track_height = px(16.0);
        let thumb_size = px(12.0);

        let switch = div()
            .id((self.id.clone(), "switch-track"))
            .w(track_width)
            .h(track_height)
            .rounded_full()
            .bg(track_bg)
            .border_1()
            .border_color(track_border)
            .flex()
            .items_center()
            .when(is_on, |this| this.justify_end())
            .when(!is_on, |this| this.justify_start())
            .px(px(1.0))
            .child(div().size(thumb_size).rounded_full().bg(thumb_bg));

        h_flex()
            .id(self.id)
            .gap(Spacing::Small.pixels())
            .when(!self.disabled, |this| this.cursor_pointer())
            .child(switch)
            .when_some(self.label, |this, label| {
                this.child(Label::new(label).size(TextSize::Small).color(label_color))
            })
            .when_some(
                (!self.disabled).then_some(self.on_click).flatten(),
                |this, handler| {
                    let next = self.state.inverse();
                    this.on_click(move |_event, window, cx| handler(&next, window, cx))
                },
            )
    }
}
