//! Popover - a floating, anchored overlay used for menus and rich tooltips.

use crate::theme::{ActiveTheme, Radius, Spacing};
use gpui::{
    Anchor, AnyElement, App, Bounds, FocusHandle, IntoElement, ParentElement, Pixels, Point,
    RenderOnce, Window, point, prelude::*, px,
};
use smallvec::SmallVec;

use crate::components::overlay::{
    OVERLAY_PRIORITY_POPOVER, OverlayConfig, OverlayPlacement, overlay_shell,
};
use crate::components::stack::v_flex;
use crate::styles::ElevationIndex;

/// Default vertical padding inside a popover container.
pub const POPOVER_PADDING: Pixels = px(4.0);

/// Default offset between a popover and the trigger element it's anchored to.
pub const POPOVER_OFFSET: Pixels = px(4.0);

/// A styled, surface-elevated container for floating UI.
#[derive(IntoElement)]
#[must_use = "Popover does nothing unless rendered"]
pub struct Popover {
    children: SmallVec<[AnyElement; 2]>,
    min_width: Option<Pixels>,
}

impl Popover {
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
            min_width: None,
        }
    }

    /// Set a minimum width for the popover.
    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = Some(width);
        self
    }
}

impl Default for Popover {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Popover {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        v_flex()
            .when_some(self.min_width, |this, w| this.min_w(w))
            .py(POPOVER_PADDING)
            .px(px(0.0))
            .rounded(Radius::Small.pixels())
            .border_1()
            .border_color(colors.border)
            .bg(colors.elevated_surface_background)
            .shadow(ElevationIndex::ElevatedSurface.shadow(cx))
            .gap(Spacing::None.pixels())
            .children(self.children)
    }
}

/// Wrap a popover (or any element) so it floats anchored to `trigger_bounds`,
/// snapping to the window edges if it would otherwise overflow. Dismisses on
/// click-outside and on `Escape` - the latter only if the caller focuses
/// `focus_handle` when opening.
pub fn anchored_popover(
    focus_handle: FocusHandle,
    corner: Anchor,
    trigger_bounds: Bounds<Pixels>,
    content: impl IntoElement,
    on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Anchor against the *opposite* corner of the trigger so the popover
    // appears adjacent to it rather than overlapping.
    let attach_corner = match corner {
        Anchor::TopLeft => Anchor::BottomLeft,
        Anchor::TopRight => Anchor::BottomRight,
        Anchor::BottomLeft => Anchor::TopLeft,
        Anchor::BottomRight => Anchor::TopRight,
        Anchor::TopCenter => Anchor::BottomCenter,
        Anchor::BottomCenter => Anchor::TopCenter,
        Anchor::LeftCenter => Anchor::RightCenter,
        Anchor::RightCenter => Anchor::LeftCenter,
    };
    let anchor_point = trigger_bounds.corner(attach_corner);
    let offset: Point<Pixels> = match corner {
        Anchor::TopLeft | Anchor::TopRight | Anchor::TopCenter => point(px(0.0), POPOVER_OFFSET),
        Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter => {
            point(px(0.0), -POPOVER_OFFSET)
        }
        Anchor::LeftCenter => point(POPOVER_OFFSET, px(0.0)),
        Anchor::RightCenter => point(-POPOVER_OFFSET, px(0.0)),
    };

    overlay_shell(
        OverlayConfig {
            id: "popover-backdrop",
            focus_handle,
            priority: OVERLAY_PRIORITY_POPOVER,
            backdrop: None,
            placement: OverlayPlacement::Anchored {
                corner,
                origin: anchor_point,
                offset,
                snap_margin: px(8.0),
            },
        },
        on_dismiss,
        content,
    )
}
