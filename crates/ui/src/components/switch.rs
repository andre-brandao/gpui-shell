use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, App, ElementId, InteractiveElement, IntoElement,
    ParentElement as _, Pixels, RenderOnce, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::super::theme::{bg, interactive, text};
use super::h_flex;

/// Which side the label appears on
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LabelSide {
    Left,
    #[default]
    Right,
}

/// Size variants for the switch
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SwitchSize {
    Small,
    #[default]
    Medium,
}

impl SwitchSize {
    fn dimensions(&self) -> (Pixels, Pixels, Pixels) {
        match self {
            SwitchSize::Small => (px(28.), px(16.), px(12.)),
            SwitchSize::Medium => (px(36.), px(20.), px(16.)),
        }
    }
}

/// A Switch element that can be toggled on or off.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    label: Option<SharedString>,
    label_side: LabelSide,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    size: SwitchSize,
    tooltip: Option<SharedString>,
}

impl Switch {
    /// Create a new Switch element.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            label: None,
            on_click: None,
            label_side: LabelSide::Right,
            size: SwitchSize::Medium,
            tooltip: None,
        }
    }

    /// Set the checked state of the switch.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the disabled state of the switch.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the label of the switch.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set which side the label appears on.
    pub fn label_side(mut self, side: LabelSide) -> Self {
        self.label_side = side;
        self
    }

    /// Set the size of the switch.
    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    /// Add a click handler for the switch.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set tooltip for the switch.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let on_click = self.on_click.clone();
        let toggle_state = window.use_keyed_state(self.id.clone(), cx, |_, _| checked);

        let (bg_color, toggle_bg) = match checked {
            true => (interactive::toggle_on(), bg::primary()),
            false => (bg::tertiary(), bg::elevated()),
        };

        let (bg_color, toggle_bg) = if self.disabled {
            (
                if checked {
                    gpui::Hsla { a: 0.5, ..bg_color }
                } else {
                    bg_color
                },
                gpui::Hsla {
                    a: 0.35,
                    ..toggle_bg
                },
            )
        } else {
            (bg_color, toggle_bg)
        };

        let (bg_width, bg_height, bar_width) = self.size.dimensions();
        let inset = px(2.);
        let radius = bg_height;

        div().child(
            h_flex()
                .id(self.id.clone())
                .gap(px(8.))
                .items_center()
                .when(self.label_side == LabelSide::Left, |this| {
                    this.flex_row_reverse()
                })
                .child(
                    // Switch Bar
                    div()
                        .id("switch-bar")
                        .w(bg_width)
                        .h(bg_height)
                        .rounded(radius)
                        .flex()
                        .items_center()
                        .border(inset)
                        .border_color(gpui::transparent_black())
                        .bg(bg_color)
                        .when(!self.disabled, |this| this.cursor_pointer())
                        .child(
                            // Switch Toggle (the sliding knob)
                            div()
                                .rounded(radius)
                                .bg(toggle_bg)
                                .shadow_md()
                                .size(bar_width)
                                .map(|this| {
                                    let prev_checked = toggle_state.read(cx);
                                    if !self.disabled && *prev_checked != checked {
                                        let duration = Duration::from_secs_f64(0.15);
                                        cx.spawn({
                                            let toggle_state = toggle_state.clone();
                                            async move |cx| {
                                                cx.background_executor().timer(duration).await;
                                                _ = toggle_state
                                                    .update(cx, |this, _| *this = checked);
                                            }
                                        })
                                        .detach();

                                        this.with_animation(
                                            ElementId::NamedInteger("move".into(), checked as u64),
                                            Animation::new(duration),
                                            move |this, delta| {
                                                let max_x = bg_width - bar_width - inset * 2;
                                                let x = if checked {
                                                    max_x * delta
                                                } else {
                                                    max_x - max_x * delta
                                                };
                                                this.left(x)
                                            },
                                        )
                                        .into_any_element()
                                    } else {
                                        let max_x = bg_width - bar_width - inset * 2;
                                        let x = if checked { max_x } else { px(0.) };
                                        this.left(x).into_any_element()
                                    }
                                }),
                        ),
                )
                .when_some(self.label.clone(), |this, label| {
                    this.child(
                        div()
                            .line_height(bg_height)
                            .text_color(if self.disabled {
                                text::disabled()
                            } else {
                                text::primary()
                            })
                            .map(|this| match self.size {
                                SwitchSize::Small => this.text_sm(),
                                SwitchSize::Medium => this.text_base(),
                            })
                            .child(label),
                    )
                })
                .when_some(
                    on_click
                        .as_ref()
                        .map(|c| c.clone())
                        .filter(|_| !self.disabled),
                    |this, on_click| {
                        let toggle_state = toggle_state.clone();
                        this.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            _ = toggle_state.update(cx, |this, _| *this = checked);
                            on_click(&!checked, window, cx);
                        })
                    },
                ),
        )
    }
}
