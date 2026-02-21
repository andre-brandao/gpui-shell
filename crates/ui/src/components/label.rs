use gpui::{div, prelude::*, App, Hsla, IntoElement, Pixels, RenderOnce, SharedString, Window};

use crate::{ActiveTheme, Theme};

/// Semantic color for text elements.
///
/// Maps to local theme colors via `Color::hsla()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    #[default]
    Default,
    Muted,
    Disabled,
    Placeholder,
    Accent,
    Error,
    Warning,
    Success,
    Info,
}

impl Color {
    pub fn hsla(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Color::Default => theme.text.primary,
            Color::Muted => theme.text.muted,
            Color::Disabled => theme.text.disabled,
            Color::Placeholder => theme.text.placeholder,
            Color::Accent => theme.accent.primary,
            Color::Error => theme.status.error,
            Color::Warning => theme.status.warning,
            Color::Success => theme.status.success,
            Color::Info => theme.status.info,
        }
    }
}

/// Label text size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelSize {
    XSmall,
    Small,
    #[default]
    Default,
    Large,
}

impl LabelSize {
    pub fn pixels(self, theme: &Theme) -> Pixels {
        match self {
            LabelSize::XSmall => theme.font_sizes.xs,
            LabelSize::Small => theme.font_sizes.sm,
            LabelSize::Default => theme.font_sizes.base,
            LabelSize::Large => theme.font_sizes.lg,
        }
    }
}

/// Trait for label-like elements that support size and color.
pub trait LabelCommon: Sized {
    fn size(self, size: LabelSize) -> Self;
    fn color(self, color: Color) -> Self;
}

/// A text label component.
#[derive(IntoElement)]
pub struct Label {
    label: SharedString,
    size: LabelSize,
    color: Color,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            size: LabelSize::Default,
            color: Color::Default,
        }
    }
}

impl LabelCommon for Label {
    fn size(mut self, size: LabelSize) -> Self {
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
        let theme = cx.theme();

        div()
            .text_size(self.size.pixels(theme))
            .text_color(self.color.hsla(cx))
            .child(self.label)
    }
}
