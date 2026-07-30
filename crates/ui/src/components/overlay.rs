//! Overlay shell - shared scaffolding for popover/modal/sheet.

use std::rc::Rc;

use gpui::{
    Anchor, App, FocusHandle, Hsla, IntoElement, MouseButton, ParentElement, Pixels, Point, Window,
    anchored, deferred, div, prelude::*,
};

use crate::components::sheet::SheetSide;
use crate::traits::DismissHandler;

/// Deferred-paint priority for anchored popovers.
pub(crate) const OVERLAY_PRIORITY_POPOVER: usize = 1;
/// Deferred-paint priority for modal + sheet overlays.
pub(crate) const OVERLAY_PRIORITY_MODAL: usize = 2;

/// How the overlay's content is positioned inside the full-window backdrop.
pub(crate) enum OverlayPlacement {
    /// Float the content at `origin + offset`, with `corner` of the content
    /// touching the anchor point.
    Anchored {
        corner: Anchor,
        origin: Point<Pixels>,
        offset: Point<Pixels>,
        snap_margin: Pixels,
    },
    /// Center the content in the window (modal).
    Centered,
    /// Pin the content to a window edge (sheet).
    Edge(SheetSide),
}

/// Configuration for [`overlay_shell`] - grouped so call sites stay readable
/// as the shell picks up optional knobs.
pub(crate) struct OverlayConfig {
    /// Stable element id for the backdrop div (useful for debugging).
    pub id: &'static str,
    /// Focus handle to attach so Escape reaches the overlay.
    pub focus_handle: FocusHandle,
    /// [`deferred`] paint priority.
    pub priority: usize,
    /// Backdrop tint. `None` = transparent (popover-style click-catcher).
    pub backdrop: Option<Hsla>,
    /// Layout for the content inside the backdrop.
    pub placement: OverlayPlacement,
}

/// Build a deferred, dismissible overlay.
pub(crate) fn overlay_shell(
    config: OverlayConfig,
    on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    content: impl IntoElement,
) -> impl IntoElement {
    let OverlayConfig {
        id,
        focus_handle,
        priority,
        backdrop,
        placement,
    } = config;

    let on_dismiss: DismissHandler = Rc::new(on_dismiss);
    let click_dismiss = on_dismiss.clone();
    let key_dismiss = on_dismiss;

    let content_card = div()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(content);

    let backdrop_div = div()
        .id(id)
        .track_focus(&focus_handle)
        .absolute()
        .inset_0()
        .size_full()
        .when_some(backdrop, |this, color| this.bg(color))
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            click_dismiss(window, cx);
        })
        .on_key_down(move |event, window, cx| {
            if event.keystroke.key == "escape" {
                key_dismiss(window, cx);
                cx.stop_propagation();
            }
        });

    let laid_out = match placement {
        OverlayPlacement::Anchored {
            corner,
            origin,
            offset,
            snap_margin,
        } => backdrop_div.child(
            anchored()
                .anchor(corner)
                .position(origin + offset)
                .snap_to_window_with_margin(snap_margin)
                .child(content_card),
        ),
        OverlayPlacement::Centered => backdrop_div
            .flex()
            .items_center()
            .justify_center()
            .child(content_card),
        OverlayPlacement::Edge(SheetSide::Right) => backdrop_div
            .flex()
            .flex_row()
            .justify_end()
            .items_stretch()
            .child(content_card),
        OverlayPlacement::Edge(SheetSide::Left) => backdrop_div
            .flex()
            .flex_row()
            .justify_start()
            .items_stretch()
            .child(content_card),
        OverlayPlacement::Edge(SheetSide::Bottom) => backdrop_div
            .flex()
            .flex_col()
            .justify_end()
            .items_stretch()
            .child(content_card),
    };

    deferred(laid_out).with_priority(priority)
}
