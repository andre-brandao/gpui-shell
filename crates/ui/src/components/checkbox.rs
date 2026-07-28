//! Checkbox - tri-state (unchecked / checked / indeterminate) interactive box
//! with optional inline label.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Radius, Spacing};
use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px};

use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::{Disableable, ToggleHandler, ToggleState, Toggleable};

/// Size of a [`Checkbox`]. Affects the box side length and label text size.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxSize {
    Small,
    #[default]
    Default,
    Large,
}

impl CheckboxSize {
    const fn box_size(self) -> gpui::Pixels {
        match self {
            Self::Small => px(12.0),
            Self::Default => px(14.0),
            Self::Large => px(16.0),
        }
    }

    const fn icon_size(self) -> IconSize {
        match self {
            Self::Small => IconSize::Indicator,
            Self::Default => IconSize::XSmall,
            Self::Large => IconSize::Small,
        }
    }

    const fn label_size(self) -> TextSize {
        match self {
            Self::Small | Self::Default => TextSize::XSmall,
            Self::Large => TextSize::Small,
        }
    }
}

/// A checkbox with optional inline label. Supports the full [`ToggleState`]
/// (including `Indeterminate`), a disabled state, and a click handler that
/// receives the **new** state after the click flipped it.
#[derive(IntoElement)]
#[must_use = "Checkbox does nothing unless rendered"]
pub struct Checkbox {
    id: ElementId,
    state: ToggleState,
    size: CheckboxSize,
    disabled: bool,
    label: Option<SharedString>,
    on_click: Option<ToggleHandler>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>, state: impl Into<ToggleState>) -> Self {
        Self {
            id: id.into(),
            state: state.into(),
            size: CheckboxSize::default(),
            disabled: false,
            label: None,
            on_click: None,
        }
    }

    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Register a click handler. The handler receives the new [`ToggleState`]
    /// produced by flipping the current state (indeterminate flips to
    /// selected, matching typical checkbox semantics).
    pub fn on_click(
        mut self,
        handler: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for Checkbox {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.state = state.into();
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let selected = matches!(self.state, ToggleState::Selected);
        let indeterminate = matches!(self.state, ToggleState::Indeterminate);
        let active = selected || indeterminate;

        let (bg, border, glyph) = if self.disabled {
            (
                colors.element_disabled,
                colors.border_variant,
                active.then_some(if indeterminate {
                    IconName::Dash
                } else {
                    IconName::Check
                }),
            )
        } else if active {
            (
                colors.accent,
                colors.accent,
                Some(if indeterminate {
                    IconName::Dash
                } else {
                    IconName::Check
                }),
            )
        } else {
            (colors.element_background, colors.border, None)
        };

        let box_size = self.size.box_size();
        let icon_size = self.size.icon_size();
        let label_size = self.size.label_size();
        let label_color = if self.disabled {
            Color::Disabled
        } else {
            Color::Default
        };
        // On the filled accent background we need a contrasting glyph.
        let glyph_color = if self.disabled {
            Color::Disabled
        } else {
            Color::Custom(colors.background)
        };

        let checkbox_box = div()
            .id(self.id)
            .size(box_size)
            .flex()
            .items_center()
            .justify_center()
            .rounded(Radius::XSmall.pixels())
            .border_1()
            .border_color(border)
            .bg(bg)
            .when(!self.disabled, |this| {
                this.cursor_pointer()
                    .hover(|s| s.border_color(colors.border_focused))
            })
            .when_some(glyph, |this, icon| {
                this.child(Icon::new(icon).size(icon_size).color(glyph_color))
            })
            .when_some(
                (!self.disabled).then_some(self.on_click).flatten(),
                |this, handler| {
                    let next_state = self.state.inverse();
                    this.on_click(move |_event, window, cx| handler(&next_state, window, cx))
                },
            );

        h_flex()
            .gap(Spacing::Small.pixels())
            .child(checkbox_box)
            .when_some(self.label, |this, label| {
                this.child(Label::new(label).size(label_size).color(label_color))
            })
    }
}
