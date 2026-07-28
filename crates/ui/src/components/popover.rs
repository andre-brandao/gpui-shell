//! Popover - a floating, anchored overlay used for menus and rich tooltips.
//!
//! Zed's `PopoverMenu` is a 500-line custom `Element` implementation that
//! manages an internal `ManagedView`, dismissal events, focus handling, and
//! deferred drawing. Engram takes a much leaner approach: the parent view
//! owns the open/closed state, and uses [`anchored_popover`] to position the
//! [`Popover`] container against an anchor (a fixed window point).
//!
//! Typical use (including the focus handle needed for Esc / click-outside):
//!
//! ```ignore
//! // On construction:
//! self.menu_focus = cx.focus_handle();
//!
//! // On trigger click: set is_open AND focus the handle.
//! Button::new("menu", "Menu").on_click(cx.listener(|this, _, window, cx| {
//!     this.menu_open = true;
//!     window.focus(&this.menu_focus, cx);
//!     cx.notify();
//! }))
//!
//! // In render: place the overlay inside a `when` guard.
//! .when(self.menu_open, |this| {
//!     this.child(anchored_popover(
//!         self.menu_focus.clone(),
//!         gpui::Anchor::TopLeft,
//!         self.menu_trigger_bounds.get().unwrap_or_default(),
//!         Popover::new().child(/* menu content */),
//!         cx.listener(|this, _, _, cx| { this.menu_open = false; cx.notify(); }),
//!     ))
//! })
//! ```
//!
//! For real menus (`ContextMenu` / `DropdownMenu`), use the wrappers in
//! [`super::menu`] which package this pattern.

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
///
/// Stateless: just a card with a border, background, padding, and shadow-like
/// elevation via the `surface_elevated` theme color. Children fill the body.
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

    /// Set a minimum width for the popover. Useful for menus where you want
    /// a stable width regardless of the longest item.
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
/// snapping to the window edges if it would otherwise overflow. Handles
/// dismissal on **click-outside** and **`Escape`**.
///
/// `corner` chooses which point of the popover sits against the trigger:
/// - `Anchor::TopLeft` -> popover hangs below-and-right of the trigger
/// - `Anchor::TopRight` -> hangs below-and-left
/// - `Anchor::BottomLeft` -> grows upward-and-right
/// - `Anchor::BottomRight` -> grows upward-and-left
/// - `Anchor::TopCenter` / `Anchor::BottomCenter` -> hangs below / above,
///   horizontally centered on the trigger
/// - `Anchor::LeftCenter` / `Anchor::RightCenter` -> sits to the right / left,
///   vertically centered on the trigger
///
/// # Dismissal
///
/// The overlay draws an invisible full-window backdrop *behind* the popover
/// to catch clicks outside the popover content. Clicks inside the popover
/// are `.occlude()`d so they don't bubble to the backdrop. For `Escape` to
/// work, the caller must focus `focus_handle` when opening the popover.
///
/// The popover is rendered inside a [`deferred`] node so it paints above
/// later siblings, matching the Z-order callers expect from a real overlay.
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
            id: "engram-popover-backdrop",
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
