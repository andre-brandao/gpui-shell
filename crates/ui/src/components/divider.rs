//! Thin separator line used to delineate regions.

use crate::theme::ActiveTheme;
use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*, px};

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
}

impl Divider {
    pub fn horizontal() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
        }
    }

    pub fn vertical() -> Self {
        Self {
            orientation: DividerOrientation::Vertical,
        }
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = cx.theme().colors().border_variant;
        match self.orientation {
            DividerOrientation::Horizontal => div().h(px(1.0)).w_full().bg(color),
            DividerOrientation::Vertical => div().w(px(1.0)).h_full().bg(color),
        }
    }
}
