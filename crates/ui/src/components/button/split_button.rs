//! [`SplitButton`] - a two-part button with a primary action on the left
//! and a secondary action (e.g. dropdown trigger) on the right, visually
//! separated by a divider.
//!
//! The outer `SplitButtonStyle` drives the chrome: `Filled` paints a shared
//! element background, `Outlined` paints a shared border, `Transparent`
//! strips both. Either side is forced to [`ButtonStyle::Transparent`] at
//! render time so the inner buttons never double-paint the chrome.

use crate::theme::ActiveTheme;
use gpui::{
    App, BoxShadow, IntoElement, ParentElement, RenderOnce, Styled, Window, div, hsla, point,
    prelude::*, px, relative,
};

use crate::components::button::button_like::{ButtonCommon, ButtonLike, ButtonStyle};
use crate::components::button::icon_button::IconButton;
use crate::components::stack::h_flex;

/// What kind of element either side of the split button contains.
pub enum SplitButtonKind {
    ButtonLike(ButtonLike),
    IconButton(IconButton),
}

impl From<ButtonLike> for SplitButtonKind {
    fn from(b: ButtonLike) -> Self {
        Self::ButtonLike(b)
    }
}

impl From<IconButton> for SplitButtonKind {
    fn from(b: IconButton) -> Self {
        Self::IconButton(b)
    }
}

impl SplitButtonKind {
    fn with_style(self, style: ButtonStyle) -> Self {
        match self {
            Self::ButtonLike(b) => Self::ButtonLike(b.style(style)),
            Self::IconButton(b) => Self::IconButton(b.style(style)),
        }
    }
}

/// Visual style of the split button container.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SplitButtonStyle {
    Filled,
    Outlined,
    Transparent,
}

/// A button with two parts separated by a thin divider.
#[derive(IntoElement)]
#[must_use = "SplitButton does nothing unless rendered"]
pub struct SplitButton {
    left: SplitButtonKind,
    right: SplitButtonKind,
    style: SplitButtonStyle,
}

impl SplitButton {
    pub fn new(left: impl Into<SplitButtonKind>, right: impl Into<SplitButtonKind>) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
            style: SplitButtonStyle::Filled,
        }
    }

    pub fn style(mut self, style: SplitButtonStyle) -> Self {
        self.style = style;
        self
    }
}

impl RenderOnce for SplitButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let is_filled = self.style == SplitButtonStyle::Filled;
        let is_outlined = self.style == SplitButtonStyle::Outlined;
        let divider_color = match self.style {
            SplitButtonStyle::Filled => colors.border.opacity(0.5),
            SplitButtonStyle::Outlined => colors.border.opacity(0.6),
            SplitButtonStyle::Transparent => colors.border.opacity(0.3),
        };

        // Both sides render with transparent chrome - the outer container
        // is responsible for background + border so the two sides read as
        // a single surface.
        let left = self.left.with_style(ButtonStyle::Transparent);
        let right = self.right.with_style(ButtonStyle::Transparent);

        let render_kind = |k: SplitButtonKind| match k {
            SplitButtonKind::ButtonLike(b) => b.into_any_element(),
            SplitButtonKind::IconButton(b) => b.into_any_element(),
        };

        h_flex()
            .rounded_sm()
            .overflow_hidden()
            .when(is_filled, |this| {
                this.bg(colors.element_hover)
                    .border_1()
                    .border_color(colors.border)
                    .shadow(vec![BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.16),
                        offset: point(px(0.), px(1.)),
                        blur_radius: px(0.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
            })
            .when(is_outlined, |this| {
                this.border_1().border_color(colors.border_selected)
            })
            .child(div().flex_grow(1.).child(render_kind(left)))
            .child(div().h(relative(0.8)).w_px().bg(divider_color))
            .child(render_kind(right))
    }
}
