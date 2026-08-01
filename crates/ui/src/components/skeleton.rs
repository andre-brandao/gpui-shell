//! Skeleton - an animated placeholder that signals content is loading.

use std::time::Duration;

use crate::theme::{ActiveTheme, Radius, Spacing};
use gpui::{
    Animation, AnimationExt, App, IntoElement, Pixels, RenderOnce, Styled, Window, div, prelude::*,
    px,
};

use crate::components::stack::v_flex;

/// Shape preset for a [`Skeleton`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonShape {
    /// Rounded rectangle (default).
    #[default]
    Rectangle,
    /// Full circle - use with equal width and height.
    Circle,
}

/// An animated loading placeholder.
#[derive(IntoElement)]
#[must_use = "Skeleton does nothing unless rendered"]
pub struct Skeleton {
    width: Pixels,
    height: Pixels,
    shape: SkeletonShape,
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            width: px(120.0),
            height: px(16.0),
            shape: SkeletonShape::default(),
        }
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    pub fn shape(mut self, shape: SkeletonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Convenience: a square circle skeleton (e.g. avatar placeholder).
    pub fn circle(size: Pixels) -> Self {
        Self {
            width: size,
            height: size,
            shape: SkeletonShape::Circle,
        }
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();

        let base = div()
            .w(self.width)
            .h(self.height)
            .bg(colors.surface_background);

        let shaped = match self.shape {
            SkeletonShape::Rectangle => base.rounded(Radius::Small.pixels()),
            SkeletonShape::Circle => base.rounded_full(),
        };

        // Pulse animation: oscillate opacity between 0.4 and 1.0.
        shaped.with_animation(
            "skeleton-pulse",
            Animation::new(Duration::from_millis(1500)).repeat(),
            |el, delta| {
                // Sine wave: 0->1->0 over one cycle, mapped to 0.4-1.0
                let t = (delta * std::f32::consts::PI * 2.0).sin() * 0.5 + 0.5;
                let opacity = 0.4 + t * 0.6;
                el.opacity(opacity)
            },
        )
    }
}

/// Convenience: a common "text block" skeleton - several lines stacked.
pub fn skeleton_text(lines: usize, line_width: Pixels) -> impl IntoElement {
    v_flex()
        .gap(Spacing::Medium.pixels())
        .children((0..lines).map(move |i| {
            // Last line is shorter for visual interest.
            let w = if i == lines - 1 {
                line_width * 0.6
            } else {
                line_width
            };
            Skeleton::new().width(w).height(px(12.0))
        }))
}
