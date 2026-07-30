//! [`LabelLike`] - the shared chrome behind every label.

use crate::theme::{ActiveTheme, Color, TextSize};
use gpui::{
    AnyElement, App, Div, FontWeight, IntoElement, ParentElement, Rems, RenderOnce, UnderlineStyle,
    Window, div, prelude::*, px, relative,
};
use smallvec::SmallVec;

/// Sets the line height behavior of a label.
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub enum LineHeightStyle {
    /// Natural line height for the resolved [`TextSize`].
    #[default]
    TextLabel,
    /// Tight line height (1.0) for compact UI labels.
    UiLabel,
}

/// Common builder methods every label-like component implements.
///
/// Naming uniformity across label types, not a generic bound.
pub trait LabelCommon {
    /// Set the size of the label.
    fn size(self, size: TextSize) -> Self;

    /// Set the font weight of the label.
    fn weight(self, weight: FontWeight) -> Self;

    /// Set the line height behavior.
    fn line_height_style(self, line_height_style: LineHeightStyle) -> Self;

    /// Set the semantic color of the label.
    fn color(self, color: Color) -> Self;

    /// Render the label with a strikethrough.
    fn strikethrough(self) -> Self;

    /// Render the label in italics.
    fn italic(self) -> Self;

    /// Render an underline beneath the label.
    fn underline(self) -> Self;

    /// Multiply the resolved color's alpha by `alpha`.
    fn alpha(self, alpha: f32) -> Self;

    /// Truncate overflowing text with a trailing ellipsis (`...`).
    fn truncate(self) -> Self;

    /// Force single-line layout, collapsing any embedded newlines.
    fn single_line(self) -> Self;
}

/// A flexible base from which the prebuilt label types
/// ([`Label`](super::label::Label), [`Headline`](super::headline::Headline))
/// are composed.
#[derive(IntoElement)]
#[must_use = "LabelLike does nothing unless rendered"]
pub struct LabelLike {
    pub(super) base: Div,
    size: TextSize,
    custom_size: Option<Rems>,
    weight: Option<FontWeight>,
    line_height_style: LineHeightStyle,
    pub(crate) color: Color,
    strikethrough: bool,
    italic: bool,
    underline: bool,
    alpha: Option<f32>,
    single_line: bool,
    truncate: bool,
    children: SmallVec<[AnyElement; 2]>,
}

impl Default for LabelLike {
    fn default() -> Self {
        Self::new()
    }
}

impl LabelLike {
    pub fn new() -> Self {
        Self {
            base: div(),
            size: TextSize::Small,
            custom_size: None,
            weight: None,
            line_height_style: LineHeightStyle::default(),
            color: Color::Default,
            strikethrough: false,
            italic: false,
            underline: false,
            alpha: None,
            single_line: false,
            truncate: false,
            children: SmallVec::new(),
        }
    }
}

impl LabelLike {
    /// Set an arbitrary size in rems, overriding the [`TextSize`] token.
    pub fn size_rems(mut self, size: Rems) -> Self {
        self.custom_size = Some(size);
        self
    }
}

impl LabelCommon for LabelLike {
    fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    fn line_height_style(mut self, line_height_style: LineHeightStyle) -> Self {
        self.line_height_style = line_height_style;
        self
    }

    fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = Some(alpha);
        self
    }

    fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    fn single_line(mut self) -> Self {
        self.single_line = true;
        self
    }
}

impl ParentElement for LabelLike {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for LabelLike {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let mut color = self.color.hsla(colors);
        if let Some(alpha) = self.alpha {
            // Mirrors zed's behaviour: rescale the resolved alpha so the
            // label fades over its semantic color rather than overwriting the
            // alpha channel outright.
            color.fade_out(1.0 - alpha.clamp(0.0, 1.0));
        }
        let underline_color = colors.text_muted.opacity(0.4);

        self.base
            .map(|this| this.text_size(self.custom_size.unwrap_or_else(|| self.size.rems())))
            .when(self.line_height_style == LineHeightStyle::UiLabel, |this| {
                this.line_height(relative(1.0))
            })
            .when(self.italic, |this| this.italic())
            .when(self.underline, |mut this| {
                this.text_style().underline = Some(UnderlineStyle {
                    thickness: px(1.0),
                    color: Some(underline_color),
                    wavy: false,
                });
                this
            })
            .when(self.strikethrough, |this| this.line_through())
            .when(self.single_line, |this| this.whitespace_nowrap())
            .when(self.truncate, |this| {
                this.min_w_0()
                    .overflow_x_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
            })
            .text_color(color)
            .when_some(self.weight, |this, weight| this.font_weight(weight))
            .children(self.children)
    }
}
