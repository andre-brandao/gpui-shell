//! Squircle - a true superellipse-corner container.
//!
//! Unlike a plain rounded rectangle, a squircle has *continuous curvature*
//! where the straight edge meets the corner - no visible seam, no "flat
//! bumper" look. The shape is the same one used by Apple's app icons and
//! Figma's rounded-rect tool.
//!
//! The corner math in [`figma`] is a direct port of the open-source
//! `figma_squircle` crate by Cameron Campbell (MIT), itself a port of
//! MartinRGB's JavaScript reference implementation. Those in turn implement
//! the construction described in Figma's "Desperately Seeking Squircles"
//! post. Engram vendors the math inline so the crate ships without any
//! additional dependencies.
//!
//! The shape is painted once per frame via an absolute-positioned
//! [`gpui::canvas`] layer that fills the squircle's bounds, so user-supplied
//! children (labels, icons, images) stack normally above the fill.

use std::f32::consts::SQRT_2;

use crate::theme::ActiveTheme;
use gpui::{
    AnyElement, App, Background, Bounds, Hsla, IntoElement, ParentElement, PathBuilder, Pixels,
    Point, RenderOnce, Styled, Window, canvas, div, point, px,
};

// -----------------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------------

/// Background fill for a [`Squircle`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SquircleFill {
    /// No fill - only the child (and optional border) is visible.
    Transparent,
    /// Grounded surface - matches panels and sidebars.
    Surface,
    /// Elevated surface - the default, matches popovers and cards.
    #[default]
    Elevated,
    /// Filled interactive background, good for framing an icon.
    Muted,
    /// Accent color fill.
    Accent,
}

/// A superellipse-corner container.
#[derive(IntoElement)]
#[must_use = "Squircle does nothing unless rendered"]
pub struct Squircle {
    width: Option<Pixels>,
    height: Option<Pixels>,
    fill: SquircleFill,
    bordered: bool,
    corner_radius: Option<Pixels>,
    corner_smoothing: f32,
    children: Vec<AnyElement>,
}

impl Squircle {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
            fill: SquircleFill::default(),
            bordered: false,
            corner_radius: None,
            // 1.0 is the iOS / full superellipse look - corner curvature
            // blends all the way into the straight edge. 0.0 collapses to a
            // plain arc (standard rounded rectangle).
            corner_smoothing: 1.0,
            children: Vec::new(),
        }
    }

    /// Set both width and height to the same value.
    pub fn size(mut self, size: Pixels) -> Self {
        self.width = Some(size);
        self.height = Some(size);
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn fill(mut self, fill: SquircleFill) -> Self {
        self.fill = fill;
        self
    }

    /// Draw a 1px border in the theme's default border color.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Explicit corner radius. Defaults to ~39% of the shorter side, which
    /// reads as an iOS app-icon silhouette.
    pub fn corner_radius(mut self, radius: Pixels) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// Corner smoothing factor, clamped to `0.0..=1.0`. 0.0 behaves like a
    /// plain rounded rectangle; 0.6 matches Figma's look; 1.0 (default) is
    /// the smoothest continuous superellipse.
    pub fn corner_smoothing(mut self, smoothing: f32) -> Self {
        self.corner_smoothing = smoothing.clamp(0.0, 1.0);
        self
    }
}

impl Default for Squircle {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Squircle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Squircle {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();

        let fill: Option<Hsla> = match self.fill {
            SquircleFill::Transparent => None,
            SquircleFill::Surface => Some(colors.surface_background),
            SquircleFill::Elevated => Some(colors.elevated_surface_background),
            SquircleFill::Muted => Some(colors.element_selected),
            SquircleFill::Accent => Some(colors.accent),
        };
        let border_color: Option<Hsla> = self.bordered.then_some(colors.border);
        let corner_radius = self.corner_radius;
        let smoothing = self.corner_smoothing;

        let shape = canvas(
            |_bounds, _, _| (),
            move |bounds, _, window, _cx| {
                paint_squircle(bounds, corner_radius, smoothing, fill, border_color, window);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let mut container = div()
            .relative()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center();
        if let Some(w) = self.width {
            container = container.w(w);
        }
        if let Some(h) = self.height {
            container = container.h(h);
        }
        container.child(shape).children(self.children)
    }
}

// -----------------------------------------------------------------------------
// Painting
// -----------------------------------------------------------------------------

const BORDER_WIDTH: f32 = 1.0;

fn paint_squircle(
    bounds: Bounds<Pixels>,
    corner_radius: Option<Pixels>,
    smoothing: f32,
    fill: Option<Hsla>,
    border_color: Option<Hsla>,
    window: &mut Window,
) {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    if fill.is_none() && border_color.is_none() {
        return;
    }

    let budget = width.min(height) / 2.0;
    // Default radius tuned for the iOS app-icon silhouette. Users override
    // via `.corner_radius(px)`.
    let radius = corner_radius
        .map(f32::from)
        .unwrap_or_else(|| budget * 0.78)
        .min(budget);

    let params = figma::corner_path_params(radius, smoothing, budget);

    if let Some(bg) = fill {
        let mut builder = PathBuilder::fill();
        trace_squircle(&mut builder, bounds.origin, width, height, &params);
        if let Ok(path) = builder.build() {
            window.paint_path(path, Background::from(bg));
        }
    }

    if let Some(border) = border_color {
        let mut builder = PathBuilder::stroke(px(BORDER_WIDTH));
        trace_squircle(&mut builder, bounds.origin, width, height, &params);
        if let Ok(path) = builder.build() {
            window.paint_path(path, Background::from(border));
        }
    }
}

/// Trace the full squircle outline onto `builder`, clockwise from the top
/// edge: moveTo, top-right corner, line, bottom-right corner, line,
/// bottom-left corner, line, top-left corner, close.
fn trace_squircle(
    builder: &mut PathBuilder,
    origin: Point<Pixels>,
    width: f32,
    height: f32,
    params: &figma::CornerPathParams,
) {
    let mut pen = Pen::new(builder, origin);
    let &figma::CornerPathParams {
        a,
        b,
        c,
        d,
        p: pp,
        corner_radius: r,
        arc_section_length: arc,
    } = params;
    let abc = a + b + c;

    pen.move_to(width - pp, 0.0);
    pen.cubic_rel(a, 0.0, a + b, 0.0, abc, d);
    pen.arc_rel(r, arc, arc);
    pen.cubic_rel(d, c, d, b + c, d, abc);

    pen.line_to(width, height - pp);
    pen.cubic_rel(0.0, a, 0.0, a + b, -d, abc);
    pen.arc_rel(r, -arc, arc);
    pen.cubic_rel(-c, d, -(b + c), d, -abc, d);

    pen.line_to(pp, height);
    pen.cubic_rel(-a, 0.0, -(a + b), 0.0, -abc, -d);
    pen.arc_rel(r, -arc, -arc);
    pen.cubic_rel(-d, -c, -d, -(b + c), -d, -abc);

    pen.line_to(0.0, pp);
    pen.cubic_rel(0.0, -a, 0.0, -(a + b), d, -abc);
    pen.arc_rel(r, arc, -arc);
    pen.cubic_rel(c, -d, b + c, -d, abc, -d);

    builder.close();
}

/// Stateful cursor that converts local `(x, y)` coordinates to absolute
/// screen points and forwards drawing commands to a [`PathBuilder`]. The
/// relative helpers (`cubic_rel`, `arc_rel`) mirror the SVG `c`/`a` commands
/// emitted by the figma_squircle math.
struct Pen<'a> {
    builder: &'a mut PathBuilder,
    origin: Point<Pixels>,
    x: f32,
    y: f32,
}

impl<'a> Pen<'a> {
    fn new(builder: &'a mut PathBuilder, origin: Point<Pixels>) -> Self {
        Self {
            builder,
            origin,
            x: 0.0,
            y: 0.0,
        }
    }

    fn abs(&self, x: f32, y: f32) -> Point<Pixels> {
        point(self.origin.x + px(x), self.origin.y + px(y))
    }

    fn move_to(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        let p = self.abs(x, y);
        self.builder.move_to(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        let p = self.abs(x, y);
        self.builder.line_to(p);
    }

    fn cubic_rel(&mut self, dx1: f32, dy1: f32, dx2: f32, dy2: f32, dx: f32, dy: f32) {
        let ctrl1 = self.abs(self.x + dx1, self.y + dy1);
        let ctrl2 = self.abs(self.x + dx2, self.y + dy2);
        let end = self.abs(self.x + dx, self.y + dy);
        self.builder.cubic_bezier_to(end, ctrl1, ctrl2);
        self.x += dx;
        self.y += dy;
    }

    fn arc_rel(&mut self, radius: f32, dx: f32, dy: f32) {
        let end = self.abs(self.x + dx, self.y + dy);
        self.builder.arc_to(
            point(px(radius), px(radius)),
            px(0.0),
            /* large_arc */ false,
            /* sweep */ true,
            end,
        );
        self.x += dx;
        self.y += dy;
    }
}

// -----------------------------------------------------------------------------
// Vendored corner math
// -----------------------------------------------------------------------------
//
// Port of `figma_squircle::draw::get_path_params_for_corner` (MIT, Cameron
// Campbell) - which is itself a port of MartinRGB's JavaScript reference
// implementation, following the construction in Figma's blog post
// "Desperately Seeking Squircles". Kept inline to avoid a crate dependency.

mod figma {
    use super::SQRT_2;

    #[derive(Debug, Clone, Copy)]
    pub(super) struct CornerPathParams {
        pub a: f32,
        pub b: f32,
        pub c: f32,
        pub d: f32,
        pub p: f32,
        pub corner_radius: f32,
        pub arc_section_length: f32,
    }

    /// Engram always asks for the `preserve_smoothing=true` flavour of the
    /// upstream function - when the requested smoothing overflows the
    /// available budget, the algorithm keeps the smoothing ratio and lets
    /// the radius shrink rather than dropping smoothing back to zero.
    pub(super) fn corner_path_params(
        corner_radius: f32,
        corner_smoothing: f32,
        rounding_and_smoothing_budget: f32,
    ) -> CornerPathParams {
        if corner_radius <= 0.0 {
            return CornerPathParams {
                a: 0.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                p: 0.0,
                corner_radius: 0.0,
                arc_section_length: 0.0,
            };
        }

        let mut p = (1.0 + corner_smoothing) * corner_radius;

        let arc_measure = 90.0 * (1.0 - corner_smoothing);
        let arc_section_length = (arc_measure / 2.0).to_radians().sin() * corner_radius * SQRT_2;

        let angle_alpha = (90.0 - arc_measure) / 2.0;
        let p3_to_p4_distance = corner_radius * (angle_alpha / 2.0).to_radians().tan();

        let angle_beta = 45.0 * corner_smoothing;
        let c = p3_to_p4_distance * angle_beta.to_radians().cos();
        let d = c * angle_beta.to_radians().tan();

        let mut b = (p - arc_section_length - c - d) / 3.0;
        let mut a = 2.0 * b;

        if p > rounding_and_smoothing_budget {
            let p1_to_p3_max_distance = rounding_and_smoothing_budget - d - arc_section_length - c;
            let min_a = p1_to_p3_max_distance / 6.0;
            let max_b = p1_to_p3_max_distance - min_a;
            b = b.min(max_b);
            a = p1_to_p3_max_distance - b;
            p = rounding_and_smoothing_budget;
        }

        CornerPathParams {
            a,
            b,
            c,
            d,
            p,
            corner_radius,
            arc_section_length,
        }
    }
}
