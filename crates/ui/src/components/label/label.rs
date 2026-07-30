//! [`Label`] - single-string text label built on [`LabelLike`].

use crate::theme::Color;
use gpui::{App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Window};

use crate::components::label::label_like::{LabelCommon, LabelLike, LineHeightStyle};
use crate::theme::TextSize;

/// A single-string text label.
#[derive(IntoElement)]
#[must_use = "Label does nothing unless rendered"]
pub struct Label {
    base: LabelLike,
    text: SharedString,
}

impl Label {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            base: LabelLike::new(),
            text: text.into(),
        }
    }
}

impl LabelCommon for Label {
    fn size(mut self, size: TextSize) -> Self {
        self.base = self.base.size(size);
        self
    }

    fn weight(mut self, weight: FontWeight) -> Self {
        self.base = self.base.weight(weight);
        self
    }

    fn line_height_style(mut self, line_height_style: LineHeightStyle) -> Self {
        self.base = self.base.line_height_style(line_height_style);
        self
    }

    fn color(mut self, color: Color) -> Self {
        self.base = self.base.color(color);
        self
    }

    fn strikethrough(mut self) -> Self {
        self.base = self.base.strikethrough();
        self
    }

    fn italic(mut self) -> Self {
        self.base = self.base.italic();
        self
    }

    fn underline(mut self) -> Self {
        self.base = self.base.underline();
        self
    }

    fn alpha(mut self, alpha: f32) -> Self {
        self.base = self.base.alpha(alpha);
        self
    }

    fn truncate(mut self) -> Self {
        self.base = self.base.truncate();
        self
    }

    fn single_line(mut self) -> Self {
        // Match zed: when a label is forced to one line, surface any embedded
        // newlines as a visible "return" glyph instead of silently swallowing
        // them.
        self.text = SharedString::from(self.text.replace('\n', "⏎"));
        self.base = self.base.single_line();
        self
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.base.child(self.text)
    }
}
