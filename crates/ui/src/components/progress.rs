//! Progress indicators - [`ProgressBar`] and [`CircularProgress`].
//!
//! Both are stateless `RenderOnce` views: the parent passes the current value
//! and max, and the component just renders the fill.

use std::f32::consts::PI;

use crate::theme::ActiveTheme;
use gpui::{
    App, Hsla, IntoElement, PathBuilder, PathStyle, Pixels, RenderOnce, StrokeOptions, Styled,
    Window, canvas, div, point, prelude::*, px, relative,
};
use lyon::tessellation::LineCap;

// -------------------- ProgressBar --------------------

/// A horizontal bar that communicates the status of a process.
#[derive(IntoElement)]
#[must_use = "ProgressBar does nothing unless rendered"]
pub struct ProgressBar {
    value: f32,
    max_value: f32,
    bg_color: Option<Hsla>,
    fg_color: Option<Hsla>,
    over_color: Option<Hsla>,
}

impl ProgressBar {
    pub fn new(value: f32, max_value: f32) -> Self {
        Self {
            value,
            max_value,
            bg_color: None,
            fg_color: None,
            over_color: None,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn max_value(mut self, max_value: f32) -> Self {
        self.max_value = max_value;
        self
    }

    pub fn bg_color(mut self, color: Hsla) -> Self {
        self.bg_color = Some(color);
        self
    }

    pub fn fg_color(mut self, color: Hsla) -> Self {
        self.fg_color = Some(color);
        self
    }

    pub fn over_color(mut self, color: Hsla) -> Self {
        self.over_color = Some(color);
        self
    }
}

impl RenderOnce for ProgressBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let bg = self.bg_color.unwrap_or(colors.element_background);
        let fg = self.fg_color.unwrap_or(colors.status.info);
        let over = self.over_color.unwrap_or(colors.status.error);
        let fill_width = (self.value / self.max_value).clamp(0.02, 1.0);

        div()
            .w_full()
            .h(px(8.0))
            .p(px(2.0))
            .rounded_full()
            .bg(bg)
            .shadow(vec![gpui::BoxShadow {
                color: gpui::black().opacity(0.08),
                offset: point(px(0.), px(1.)),
                blur_radius: px(0.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(
                div()
                    .h_full()
                    .rounded_full()
                    .when(self.value > self.max_value, |this| this.bg(over))
                    .when(self.value <= self.max_value, |this| this.bg(fg))
                    .w(relative(fill_width)),
            )
    }
}

// -------------------- CircularProgress --------------------

/// A circular progress indicator that draws an arc from the top, clockwise.
#[derive(IntoElement)]
#[must_use = "CircularProgress does nothing unless rendered"]
pub struct CircularProgress {
    value: f32,
    max_value: f32,
    size: Pixels,
    stroke_width: Pixels,
    bg_color: Option<Hsla>,
    progress_color: Option<Hsla>,
}

impl CircularProgress {
    pub fn new(value: f32, max_value: f32, size: Pixels) -> Self {
        Self {
            value,
            max_value,
            size,
            stroke_width: px(4.0),
            bg_color: None,
            progress_color: None,
        }
    }

    pub fn stroke_width(mut self, stroke_width: Pixels) -> Self {
        self.stroke_width = stroke_width;
        self
    }

    pub fn bg_color(mut self, color: Hsla) -> Self {
        self.bg_color = Some(color);
        self
    }

    pub fn progress_color(mut self, color: Hsla) -> Self {
        self.progress_color = Some(color);
        self
    }
}

impl RenderOnce for CircularProgress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let bg_color = self.bg_color.unwrap_or(colors.border_variant);
        let progress_color = self.progress_color.unwrap_or(colors.status.info);
        let value = self.value;
        let max_value = self.max_value;
        let size = self.size;
        let stroke_width = self.stroke_width;

        canvas(
            |_, _, _| {},
            move |bounds, _, window, _cx| {
                let center_x = bounds.origin.x + bounds.size.width / 2.0;
                let center_y = bounds.origin.y + bounds.size.height / 2.0;
                let radius = (size / 2.0) - stroke_width;

                // Background circle
                let mut bg_builder = PathBuilder::stroke(stroke_width);
                bg_builder.move_to(point(center_x + radius, center_y));
                bg_builder.arc_to(
                    point(radius, radius),
                    px(0.),
                    false,
                    true,
                    point(center_x - radius, center_y),
                );
                bg_builder.arc_to(
                    point(radius, radius),
                    px(0.),
                    false,
                    true,
                    point(center_x + radius, center_y),
                );
                bg_builder.close();
                if let Ok(path) = bg_builder.build() {
                    window.paint_path(path, bg_color);
                }

                // Progress arc - rounded caps so the leading/trailing ends
                // of a partial arc look like SVG `stroke-linecap="round"`.
                let progress = (value / max_value).clamp(0.0, 1.0);
                if progress > 0.0 {
                    let stroke_opts = StrokeOptions::default()
                        .with_line_width(f32::from(stroke_width))
                        .with_start_cap(LineCap::Round)
                        .with_end_cap(LineCap::Round);
                    let mut pb = PathBuilder::default().with_style(PathStyle::Stroke(stroke_opts));

                    if progress >= 0.999 {
                        pb.move_to(point(center_x + radius, center_y));
                        pb.arc_to(
                            point(radius, radius),
                            px(0.),
                            false,
                            true,
                            point(center_x - radius, center_y),
                        );
                        pb.arc_to(
                            point(radius, radius),
                            px(0.),
                            false,
                            true,
                            point(center_x + radius, center_y),
                        );
                        pb.close();
                    } else {
                        let start_x = center_x;
                        let start_y = center_y - radius;
                        pb.move_to(point(start_x, start_y));

                        let angle = -PI / 2.0 + (progress * 2.0 * PI);
                        let end_x = center_x + radius * angle.cos();
                        let end_y = center_y + radius * angle.sin();

                        pb.arc_to(
                            point(radius, radius),
                            px(0.),
                            progress > 0.5,
                            true,
                            point(end_x, end_y),
                        );
                    }

                    if let Ok(path) = pb.build() {
                        window.paint_path(path, progress_color);
                    }
                }
            },
        )
        .size(size)
    }
}
