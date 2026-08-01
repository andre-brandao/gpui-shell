//! DecoratedIcon - an icon with an optional overlay decoration (badge dot).

use crate::theme::Radius;
use gpui::{
    App, Hsla, IntoElement, ParentElement, Pixels, Point, RenderOnce, Styled, Window, div, px,
};

use crate::components::icon::{Icon, IconSize};

/// A small overlay decoration rendered on top of an icon.
#[derive(IntoElement)]
#[must_use = "IconDecoration does nothing unless rendered"]
pub struct IconDecoration {
    color: Hsla,
    size: Pixels,
    position: Point<Pixels>,
}

impl IconDecoration {
    /// Create a dot decoration with the given color.
    pub fn dot(color: Hsla) -> Self {
        Self {
            color,
            size: px(8.0),
            position: Point {
                x: px(-1.0),
                y: px(-1.0),
            },
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn position(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }
}

impl RenderOnce for IconDecoration {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .size(self.size)
            .flex_none()
            .absolute()
            .bottom(self.position.y)
            .right(self.position.x)
            .rounded(Radius::Full.pixels())
            .bg(self.color)
    }
}

/// An icon with an optional decoration overlay.
#[derive(IntoElement)]
#[must_use = "DecoratedIcon does nothing unless rendered"]
pub struct DecoratedIcon {
    icon: Icon,
    icon_size: IconSize,
    decoration: Option<IconDecoration>,
}

impl DecoratedIcon {
    pub fn new(icon: Icon, icon_size: IconSize, decoration: Option<IconDecoration>) -> Self {
        Self {
            icon,
            icon_size,
            decoration,
        }
    }
}

impl RenderOnce for DecoratedIcon {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .relative()
            .size(self.icon_size.pixels())
            .child(self.icon.size(self.icon_size))
            .children(self.decoration)
    }
}
