//! Spinner - an animated loading indicator.
//!
//! Uses GPUI's `Animation` + `AnimationExt` to continuously rotate an icon
//! (default: [`IconName::LoaderCircle`]). Useful anywhere a background
//! operation is in progress. Pair with `Color::Success` / `Color::Warning` /
//! `Color::Error` / `Color::Info` when the spinner's tone should reflect the
//! status of the work it represents.

use std::time::Duration;

use crate::theme::{ActiveTheme, Color};
use gpui::{
    Animation, AnimationExt, App, IntoElement, RenderOnce, Styled, Transformation, Window,
    percentage,
};

use crate::components::icon::{IconName, IconSize};

/// An animated spinner icon.
#[derive(IntoElement)]
#[must_use = "Spinner does nothing unless rendered"]
pub struct Spinner {
    icon: IconName,
    size: IconSize,
    color: Color,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            icon: IconName::LoaderCircle,
            size: IconSize::Medium,
            color: Color::Muted,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let hsla = match self.color {
            Color::Default => colors.icon,
            Color::Muted => colors.icon_muted,
            Color::Disabled => colors.icon_disabled,
            Color::Accent => colors.icon_accent,
            other => other.hsla(colors),
        };

        let size = self.size.rems();

        gpui::svg()
            .size(size)
            .flex_none()
            .path(self.icon.path())
            .text_color(hsla)
            .with_animation(
                "spinner-rotate",
                Animation::new(Duration::from_secs(1)).repeat(),
                |svg, delta| svg.with_transformation(Transformation::rotate(percentage(delta))),
            )
    }
}
