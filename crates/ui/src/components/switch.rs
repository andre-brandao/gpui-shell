//! Switch - a two-state toggle (on / off) with optional inline label.

use std::rc::Rc;
use std::time::Duration;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{
    Animation, AnimationExt as _, App, ElementId, IntoElement, Pixels, RenderOnce, SharedString,
    Window, div, prelude::*, px,
};

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::{Disableable, ToggleHandler, ToggleState, Toggleable};

/// How long the knob takes to slide between positions.
const SLIDE_DURATION: Duration = Duration::from_millis(150);

/// Size variants for the switch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SwitchSize {
    /// 28x16 track with a 12px knob.
    Small,
    /// 36x20 track with a 16px knob.
    #[default]
    Medium,
}

impl SwitchSize {
    /// `(track width, track height, knob size)`.
    const fn dimensions(self) -> (Pixels, Pixels, Pixels) {
        match self {
            Self::Small => (px(28.0), px(16.0), px(12.0)),
            Self::Medium => (px(36.0), px(20.0), px(16.0)),
        }
    }
}

#[derive(IntoElement)]
#[must_use = "Switch does nothing unless rendered"]
pub struct Switch {
    id: ElementId,
    state: ToggleState,
    size: SwitchSize,
    disabled: bool,
    label: Option<SharedString>,
    on_click: Option<ToggleHandler>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>, state: impl Into<ToggleState>) -> Self {
        Self {
            id: id.into(),
            state: state.into(),
            size: SwitchSize::default(),
            disabled: false,
            label: None,
            on_click: None,
        }
    }

    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Register a click handler.
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_on = self.state.selected();

        // Remembers the state the knob was last painted at, so a render that
        // flips it can animate instead of teleporting.
        let previous = window.use_keyed_state((self.id.clone(), "switch-state"), cx, |_, _| is_on);
        let colors = *cx.theme().colors();

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

        let (track_width, track_height, thumb_size) = self.size.dimensions();
        let inset = px(2.0);
        let travel = track_width - thumb_size - inset * 2.0;

        let knob = div()
            .absolute()
            .size(thumb_size)
            .rounded_full()
            .bg(thumb_bg)
            .shadow_md();

        let was_on = *previous.read(cx);
        let animating = !self.disabled && was_on != is_on;

        if animating {
            // Record the new position once the slide has played out, so the
            // next render treats it as settled rather than replaying.
            let previous = previous.clone();
            cx.spawn(async move |cx| {
                cx.background_executor().timer(SLIDE_DURATION).await;
                previous.update(cx, |state, _| *state = is_on);
            })
            .detach();
        }

        let knob = if animating {
            knob.with_animation(
                ElementId::NamedInteger("switch-slide".into(), is_on as u64),
                Animation::new(SLIDE_DURATION),
                move |this, delta| {
                    let x = if is_on {
                        travel * delta
                    } else {
                        travel * (1.0 - delta)
                    };
                    this.left(x)
                },
            )
            .into_any_element()
        } else {
            knob.left(if is_on { travel } else { px(0.0) })
                .into_any_element()
        };

        let switch = div()
            .id((self.id.clone(), "switch-track"))
            .w(track_width)
            .h(track_height)
            .rounded_full()
            .bg(track_bg)
            .border(inset)
            .border_color(track_border)
            .relative()
            .flex()
            .items_center()
            .child(knob);

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
                    this.on_click(move |_event, window, cx| {
                        previous.update(cx, |state, _| *state = is_on);
                        handler(&next, window, cx)
                    })
                },
            )
    }
}
