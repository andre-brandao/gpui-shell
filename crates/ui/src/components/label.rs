use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

use crate::{ActiveTheme, Color, TextSize};

/// Trait for label-like elements that support size and color.
pub trait LabelCommon: Sized {
    fn size(self, size: TextSize) -> Self;
    fn color(self, color: Color) -> Self;
}

/// A text label component.
#[derive(IntoElement)]
pub struct Label {
    label: SharedString,
    size: TextSize,
    color: Color,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            size: TextSize::Default,
            color: Color::Default,
        }
    }
}

impl LabelCommon for Label {
    fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_size(self.size.rems())
            .text_color(self.color.hsla(cx.theme().colors()))
            .child(self.label)
    }
}
