//! Status bar chrome: the edge-anchored surface and the chip each widget
//! sits in.
//!
//! Both are orientation-aware. A bar on the left is not a bar on top with
//! `flex_col` - the radii, the padding and the label sizes all shift, which
//! is exactly the kind of duplicated `if is_vertical` the app should not be
//! carrying per widget.

use gpui::{
    AnyElement, App, FontWeight, Hsla, IntoElement, Pixels, RenderOnce, Window, div, prelude::*, px,
};

use crate::{ActiveTheme, Radius, Spacing, TextSize};

/// Which screen edge the bar is anchored to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarEdge {
    Top,
    Bottom,
    Left,
    Right,
}

impl BarEdge {
    pub fn is_vertical(self) -> bool {
        matches!(self, BarEdge::Left | BarEdge::Right)
    }
}

/// Where a section sits along the bar's long axis.
#[derive(Clone, Copy)]
enum Align {
    Start,
    Center,
    End,
}

/// The bar itself: a full-bleed surface with three widget sections.
///
/// Only the edge facing the screen gets a border, so the bar reads as an
/// edge rather than a floating box.
#[derive(IntoElement)]
#[must_use = "BarSurface does nothing unless rendered"]
pub struct BarSurface {
    edge: BarEdge,
    border: bool,
    padding: Pixels,
    start: Vec<AnyElement>,
    center: Vec<AnyElement>,
    end: Vec<AnyElement>,
}

impl BarSurface {
    pub fn new(edge: BarEdge) -> Self {
        Self {
            edge,
            border: false,
            padding: px(0.),
            start: Vec::new(),
            center: Vec::new(),
            end: Vec::new(),
        }
    }

    /// Draw the hairline on the screen-facing edge.
    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    /// Padding along the long axis. Ignored when vertical, where the bar is
    /// only as wide as its widgets.
    pub fn padding(mut self, padding: Pixels) -> Self {
        self.padding = padding;
        self
    }

    pub fn start(mut self, widgets: impl IntoIterator<Item = AnyElement>) -> Self {
        self.start.extend(widgets);
        self
    }

    pub fn center(mut self, widgets: impl IntoIterator<Item = AnyElement>) -> Self {
        self.center.extend(widgets);
        self
    }

    pub fn end(mut self, widgets: impl IntoIterator<Item = AnyElement>) -> Self {
        self.end.extend(widgets);
        self
    }
}

impl RenderOnce for BarSurface {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let vertical = self.edge.is_vertical();
        let edge = self.edge;
        let border = self.border;

        let root = div()
            .size_full()
            .flex()
            .text_size(TextSize::Small.rems())
            .font_weight(FontWeight::MEDIUM)
            .text_color(colors.text)
            .bg(colors.background)
            .border_color(colors.border);

        let root = if vertical {
            root.flex_col()
                .items_center()
                .px(px(1.0))
                .py(Spacing::Medium.pixels())
                .when(border && edge == BarEdge::Left, |el| el.border_r_1())
                .when(border && edge == BarEdge::Right, |el| el.border_l_1())
        } else {
            root.items_center()
                .px(self.padding)
                .when(border && edge == BarEdge::Top, |el| el.border_b_1())
                .when(border && edge == BarEdge::Bottom, |el| el.border_t_1())
        };

        root.child(section(vertical, Align::Start, self.start))
            .child(section(vertical, Align::Center, self.center))
            .child(section(vertical, Align::End, self.end))
    }
}

fn section(vertical: bool, align: Align, widgets: Vec<AnyElement>) -> impl IntoElement {
    if vertical {
        // A vertical bar sizes to its widgets, so only the centre section
        // claims the leftover space; start and end stay at their content
        // height and hug their ends.
        div()
            .flex()
            .w_full()
            .flex_col()
            .items_center()
            .map(|el| match align {
                Align::Start => el.justify_start(),
                Align::Center => el.flex_1().justify_center(),
                Align::End => el.justify_end(),
            })
            .children(widgets)
    } else {
        div()
            .flex()
            .h_full()
            .items_center()
            .flex_1()
            .map(|el| match align {
                Align::Start => el.justify_start(),
                Align::Center => el.justify_center().overflow_hidden(),
                Align::End => el.justify_end(),
            })
            .children(widgets)
    }
}

/// The pill a bar widget lives in, plus the breathing room around it.
///
/// Colours are passed in rather than read from the theme: whether a widget
/// paints a background at all is a config decision, and that config lives in
/// the app.
#[derive(IntoElement)]
#[must_use = "BarChip does nothing unless rendered"]
pub struct BarChip {
    content: AnyElement,
    vertical: bool,
    grouped: bool,
    background: Hsla,
    border: Option<Hsla>,
    hover: Option<Hsla>,
}

impl BarChip {
    pub fn new(content: impl IntoElement) -> Self {
        Self {
            content: content.into_any_element(),
            vertical: false,
            grouped: false,
            background: gpui::transparent_black(),
            border: None,
            hover: None,
        }
    }

    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Wider inner padding, for widgets that hold several items (workspaces,
    /// tray) rather than one icon and a value.
    pub fn grouped(mut self, grouped: bool) -> Self {
        self.grouped = grouped;
        self
    }

    pub fn background(mut self, background: Hsla) -> Self {
        self.background = background;
        self
    }

    pub fn border(mut self, border: Option<Hsla>) -> Self {
        self.border = border;
        self
    }

    /// Setting a hover colour also makes the chip look clickable.
    pub fn hover(mut self, hover: Option<Hsla>) -> Self {
        self.hover = hover;
        self
    }
}

/// Vertical padding inside a chip.
const CHIP_PADDING_Y: f32 = 3.0;
/// Breathing room around each chip.
const CHIP_OUTER_MARGIN: f32 = 2.0;

/// Horizontal padding for a single-item bar chip.
fn chip_padding_x(vertical: bool) -> f32 {
    if vertical {
        Spacing::XSmall.value()
    } else {
        Spacing::Medium.value()
    }
}

impl RenderOnce for BarChip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let vertical = self.vertical;
        let hover = self.hover;
        let padding_x = if self.grouped {
            if vertical {
                2.0
            } else {
                Spacing::Medium.value() - 1.0
            }
        } else {
            chip_padding_x(vertical)
        };

        let chip = div()
            .map(|el| {
                if vertical {
                    el.w_full().flex().flex_col().items_center()
                } else {
                    // A horizontal bar is short enough that a fixed widget
                    // height keeps every chip on one baseline.
                    el.flex().items_center().h(px(24.0))
                }
            })
            .justify_center()
            .px(px(padding_x))
            .py(px(if vertical {
                CHIP_PADDING_Y
            } else {
                CHIP_PADDING_Y + 1.0
            }))
            .rounded(px(if vertical {
                Radius::Small.value()
            } else {
                Radius::Large.value()
            }))
            .bg(self.background)
            .when_some(self.border, |el, color| el.border_1().border_color(color))
            .when_some(hover, |el, color| {
                el.cursor_pointer().hover(move |style| style.bg(color))
            })
            .child(self.content);

        if vertical {
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .py(px(CHIP_OUTER_MARGIN))
                .child(chip)
        } else {
            div()
                .mx(px(CHIP_OUTER_MARGIN))
                .my(px(CHIP_OUTER_MARGIN))
                .child(chip)
        }
    }
}
