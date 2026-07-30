//! Thin separator line used to delineate regions.

use crate::theme::ActiveTheme;
use gpui::{App, Hsla, IntoElement, Pixels, RenderOnce, Window, div, prelude::*, px};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerOrientation {
    Horizontal,
    Vertical,
}

/// A 1px line rendered in the theme's `border_variant` color.
#[derive(IntoElement)]
#[must_use = "Divider does nothing unless rendered"]
pub struct Divider {
    orientation: DividerOrientation,
    color: Option<Hsla>,
    length: Option<Pixels>,
}

impl Divider {
    pub fn horizontal() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            color: None,
            length: None,
        }
    }

    pub fn vertical() -> Self {
        Self {
            orientation: DividerOrientation::Vertical,
            color: None,
            length: None,
        }
    }

    /// Override the line color. Needed where the divider's visibility is a
    /// config decision (a bar widget's border can be turned off) rather than
    /// a theme one.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Give the line a fixed extent instead of spanning its parent. A
    /// vertical divider inside an `items_center` row has no parent height to
    /// span, so it needs one.
    pub fn length(mut self, length: Pixels) -> Self {
        self.length = Some(length);
        self
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self
            .color
            .unwrap_or_else(|| cx.theme().colors().border_variant);
        match self.orientation {
            DividerOrientation::Horizontal => div()
                .h(px(1.0))
                .map(|el| match self.length {
                    Some(length) => el.w(length),
                    None => el.w_full(),
                })
                .bg(color),
            DividerOrientation::Vertical => div()
                .w(px(1.0))
                .map(|el| match self.length {
                    Some(length) => el.h(length),
                    None => el.h_full(),
                })
                .bg(color),
        }
    }
}
