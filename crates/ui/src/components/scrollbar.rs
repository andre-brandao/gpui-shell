//! Minimal scrollbar indicator for [`gpui::ScrollHandle`]-driven content.

use crate::theme::{ActiveTheme, Radius};
use gpui::{
    App, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, ScrollHandle, Styled, Window,
    div, prelude::*, px,
};

/// Which axis the scrollbar tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarAxis {
    Vertical,
    Horizontal,
}

/// A styled scroll indicator for a [`gpui::ScrollHandle`].
#[derive(IntoElement)]
#[must_use = "Scrollbar does nothing unless rendered"]
pub struct Scrollbar {
    scroll_handle: ScrollHandle,
    axis: ScrollbarAxis,
    thickness: Pixels,
}

impl Scrollbar {
    pub fn vertical(scroll_handle: ScrollHandle) -> Self {
        Self {
            scroll_handle,
            axis: ScrollbarAxis::Vertical,
            thickness: px(10.0),
        }
    }

    pub fn horizontal(scroll_handle: ScrollHandle) -> Self {
        Self {
            scroll_handle,
            axis: ScrollbarAxis::Horizontal,
            thickness: px(10.0),
        }
    }

    /// Override the default track thickness (width for vertical, height for
    /// horizontal).
    pub fn thickness(mut self, thickness: Pixels) -> Self {
        self.thickness = thickness;
        self
    }
}

/// Minimum thumb length as a fraction of the track.
const MIN_THUMB_RATIO: f32 = 0.08;

impl RenderOnce for Scrollbar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let bounds = self.scroll_handle.bounds();
        let offset = self.scroll_handle.offset();
        let max_offset = self.scroll_handle.max_offset();

        match self.axis {
            ScrollbarAxis::Vertical => {
                let viewport = bounds.size.height.as_f32();
                let max = max_offset.y.as_f32();
                // Content shorter than viewport - no thumb, zero-sized.
                let content_fits = max <= 0.0 || viewport <= 0.0;
                let (thumb_height, thumb_top) = if content_fits {
                    (px(0.0), px(0.0))
                } else {
                    let content_height = viewport + max;
                    let ratio = (viewport / content_height).clamp(MIN_THUMB_RATIO, 1.0);
                    let th = viewport * ratio;
                    let scroll_ratio = ((-offset.y.as_f32()) / max).clamp(0.0, 1.0);
                    let tt = (viewport - th) * scroll_ratio;
                    (px(th), px(tt))
                };

                let click_handle = self.scroll_handle.clone();
                div()
                    .id("scrollbar-v")
                    .w(self.thickness)
                    .h_full()
                    .relative()
                    .bg(colors.element_background)
                    .rounded(Radius::Full.pixels())
                    .on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
                        let bounds = click_handle.bounds();
                        let max_y = click_handle.max_offset().y.as_f32();
                        let viewport_h = bounds.size.height.as_f32();
                        if max_y <= 0.0 || viewport_h <= 0.0 {
                            return;
                        }
                        let relative_y = (event.position.y - bounds.top()).as_f32();
                        let ratio = (relative_y / viewport_h).clamp(0.0, 1.0);
                        let mut current = click_handle.offset();
                        current.y = px(-max_y * ratio);
                        click_handle.set_offset(current);
                    })
                    .child(
                        div()
                            .absolute()
                            .top(thumb_top)
                            .left_0()
                            .w_full()
                            .h(thumb_height)
                            .bg(colors.border)
                            .rounded(Radius::Full.pixels()),
                    )
            }
            ScrollbarAxis::Horizontal => {
                let viewport = bounds.size.width.as_f32();
                let max = max_offset.x.as_f32();
                let content_fits = max <= 0.0 || viewport <= 0.0;
                let (thumb_width, thumb_left) = if content_fits {
                    (px(0.0), px(0.0))
                } else {
                    let content_width = viewport + max;
                    let ratio = (viewport / content_width).clamp(MIN_THUMB_RATIO, 1.0);
                    let tw = viewport * ratio;
                    let scroll_ratio = ((-offset.x.as_f32()) / max).clamp(0.0, 1.0);
                    let tl = (viewport - tw) * scroll_ratio;
                    (px(tw), px(tl))
                };

                let click_handle = self.scroll_handle.clone();
                div()
                    .id("scrollbar-h")
                    .h(self.thickness)
                    .w_full()
                    .relative()
                    .bg(colors.element_background)
                    .rounded(Radius::Full.pixels())
                    .on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
                        let bounds = click_handle.bounds();
                        let max_x = click_handle.max_offset().x.as_f32();
                        let viewport_w = bounds.size.width.as_f32();
                        if max_x <= 0.0 || viewport_w <= 0.0 {
                            return;
                        }
                        let relative_x = (event.position.x - bounds.left()).as_f32();
                        let ratio = (relative_x / viewport_w).clamp(0.0, 1.0);
                        let mut current = click_handle.offset();
                        current.x = px(-max_x * ratio);
                        click_handle.set_offset(current);
                    })
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(thumb_left)
                            .h_full()
                            .w(thumb_width)
                            .bg(colors.border)
                            .rounded(Radius::Full.pixels()),
                    )
            }
        }
    }
}
