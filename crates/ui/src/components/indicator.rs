//! Indicator - small status marker (dot, bar, or icon).

use crate::theme::{ActiveTheme, Color};
use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*, px};

use crate::components::icon::{Icon, IconSize};

enum IndicatorKind {
    Dot,
    Bar,
    Icon(Icon),
}

#[derive(IntoElement)]
#[must_use = "Indicator does nothing unless rendered"]
pub struct Indicator {
    kind: IndicatorKind,
    color: Color,
    border_color: Option<Color>,
}

impl Indicator {
    pub fn dot() -> Self {
        Self {
            kind: IndicatorKind::Dot,
            color: Color::Default,
            border_color: None,
        }
    }

    pub fn bar() -> Self {
        Self {
            kind: IndicatorKind::Bar,
            color: Color::Default,
            border_color: None,
        }
    }

    pub fn icon(icon: Icon) -> Self {
        Self {
            kind: IndicatorKind::Icon(icon),
            color: Color::Default,
            border_color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }
}

impl RenderOnce for Indicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let fg = self.color.hsla(colors);

        let container = div().flex_none();
        let container = match self.border_color {
            Some(border) if !matches!(self.kind, IndicatorKind::Icon(_)) => {
                container.border_1().border_color(border.hsla(colors))
            }
            _ => container,
        };

        match self.kind {
            IndicatorKind::Dot => container
                .size(px(6.0))
                .rounded_full()
                .bg(fg)
                .into_any_element(),
            IndicatorKind::Bar => container
                .h(px(6.0))
                .w_full()
                .rounded(px(2.0))
                .bg(fg)
                .into_any_element(),
            IndicatorKind::Icon(icon) => container
                .child(icon.size(IconSize::Indicator).color(self.color))
                .into_any_element(),
        }
    }
}
