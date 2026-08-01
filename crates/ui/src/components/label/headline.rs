//! [`Headline`] - a typographic step above [`Label`](super::label::Label).

use crate::theme::{Color, TextSize};
use gpui::{
    App, FontWeight, IntoElement, ParentElement, Rems, RenderOnce, SharedString, Window, rems,
};

use crate::components::label::label_like::{LabelCommon, LabelLike, LineHeightStyle};

/// The size of a [`Headline`] element.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum HeadlineSize {
    /// An extra small headline - `~14px` @16px/rem.
    XSmall,
    /// A small headline - `16px` @16px/rem.
    Small,
    /// A medium headline - `~18px` @16px/rem. The default.
    #[default]
    Medium,
    /// A large headline - `~20px` @16px/rem.
    Large,
    /// An extra large headline - `~22px` @16px/rem.
    XLarge,
}

impl HeadlineSize {
    /// Returns the headline size as rems.
    pub fn rems(self) -> Rems {
        match self {
            Self::XSmall => rems(0.88),
            Self::Small => rems(1.0),
            Self::Medium => rems(1.125),
            Self::Large => rems(1.27),
            Self::XLarge => rems(1.43),
        }
    }
}

/// A headline element used to emphasize text and create visual hierarchy.
#[derive(IntoElement)]
#[must_use = "Headline does nothing unless rendered"]
pub struct Headline {
    base: LabelLike,
    size: HeadlineSize,
    text: SharedString,
}

impl Headline {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            base: LabelLike::new().weight(FontWeight::SEMIBOLD),
            size: HeadlineSize::default(),
            text: text.into(),
        }
    }

    /// Set the size of the headline.
    pub fn size(mut self, size: HeadlineSize) -> Self {
        self.size = size;
        self
    }
}

impl LabelCommon for Headline {
    fn size(self, _size: TextSize) -> Self {
        // `Headline` is parameterised by its own `HeadlineSize` scale, not by
        // `TextSize`. Use the inherent [`Headline::size`] instead.
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
        self.text = SharedString::from(self.text.replace('\n', "⏎"));
        self.base = self.base.single_line();
        self
    }
}

impl RenderOnce for Headline {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // Apply the headline-scale font size via `LabelLike::size_rems` so
        // all the other LabelLike modifiers (weight, italic, color, truncate,
        // etc) compose without needing a parallel render path.
        self.base.size_rems(self.size.rems()).child(self.text)
    }
}
