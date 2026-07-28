//! VirtualList - lazy-rendered scrollable list of uniform-height items.
//!
//! Thin wrapper around [`gpui::uniform_list`]. Zed's `uniform_list` is the
//! canonical primitive for "render only the visible subset of N items of
//! uniform height". Engram's job here is only to:
//!
//! 1. give it a matching component-style builder surface (engram tends to
//!    expose types, not free functions, for components);
//! 2. provide one-call scrollbar attachment via a [`UniformListDecoration`]
//!    implementation - same integration pattern Zed's own `Scrollbar` uses;
//! 3. expose a [`VirtualListScrollHandle`] wrapper that bundles the gpui
//!    scroll handle with a small shared cell for scrollbar drag state,
//!    plus the shared padded bounds needed to convert mouse coords
//!    back to scroll offsets.
//!
//! Variable-height rows are out of scope for `VirtualList` - use
//! [`gpui::list`] directly for that case (same shape, slower layout,
//! different scroll-handle type).
//!
//! ## Usage
//!
//! ```ignore
//! let handle = VirtualListScrollHandle::new();
//! VirtualList::new("rows", items.len(), move |range, _window, _cx| {
//!     items[range].iter().map(|item| row(item)).collect()
//! })
//! .track_scroll(handle.clone())
//! .scrollbar()
//! .h_full()
//! ```

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use crate::theme::{ActiveTheme, Radius};
use gpui::{
    AnyElement, App, Bounds, Div, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, Point, Stateful, Styled, UniformListDecoration, UniformListScrollHandle,
    Window, div, px, uniform_list,
};

use super::scroll_metrics::{SCROLLBAR_THICKNESS, ThumbMetrics};

pub use gpui::ScrollStrategy;

/// Scroll handle for a [`VirtualList`]. Clones share state.
#[derive(Clone, Default)]
pub struct VirtualListScrollHandle {
    inner: UniformListScrollHandle,
    /// `Some(offset_from_thumb_top)` while a scrollbar drag is in progress.
    drag_offset: Rc<Cell<Option<Pixels>>>,
    /// Padded viewport bounds (where items are drawn), captured each frame
    /// by the scrollbar decoration. Used by the drag-move handler to map
    /// mouse Y back to a scroll offset.
    viewport: Rc<Cell<Bounds<Pixels>>>,
    /// Content height of the last drawn frame. Zero means "unknown".
    content_height: Rc<Cell<Pixels>>,
}

impl VirtualListScrollHandle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scroll the list so that the given item index is visible.
    pub fn scroll_to_item(&self, ix: usize, strategy: ScrollStrategy) {
        self.inner.scroll_to_item(ix, strategy);
    }

    /// Access the underlying gpui handle (e.g. for less-common methods
    /// like `scroll_to_item_strict`).
    pub fn as_uniform(&self) -> &UniformListScrollHandle {
        &self.inner
    }
}

/// Lazy-rendered list of uniform-height items.
#[must_use = "VirtualList does nothing unless rendered"]
pub struct VirtualList {
    inner: gpui::UniformList,
    scroll_handle: Option<VirtualListScrollHandle>,
    show_scrollbar: bool,
}

impl VirtualList {
    /// Build a new virtual list. `item_count` is the total number of rows
    /// (not just the visible window); `render_items` receives the visible
    /// index range and returns an element per index.
    pub fn new<R>(
        id: impl Into<ElementId>,
        item_count: usize,
        render_items: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
    ) -> Self
    where
        R: IntoElement,
    {
        Self {
            inner: uniform_list(id, item_count, render_items),
            scroll_handle: None,
            show_scrollbar: false,
        }
    }

    /// Attach a scroll handle.
    pub fn track_scroll(mut self, handle: VirtualListScrollHandle) -> Self {
        self.inner = self.inner.track_scroll(&handle.inner);
        self.scroll_handle = Some(handle);
        self
    }

    /// Overlay an engram-styled scrollbar on the right edge of the list.
    /// The thumb is draggable and the track is click-to-jump.
    pub fn scrollbar(mut self) -> Self {
        if self.scroll_handle.is_none() {
            let handle = VirtualListScrollHandle::default();
            self.inner = self.inner.track_scroll(&handle.inner);
            self.scroll_handle = Some(handle);
        }
        let handle = self.scroll_handle.clone().unwrap();
        self.inner = self
            .inner
            .pr(SCROLLBAR_THICKNESS)
            .with_decoration(VirtualListScrollbar { handle });
        self.show_scrollbar = true;
        self
    }

    /// Pick which item is measured to determine uniform row height.
    pub fn width_from_item(mut self, item_index: Option<usize>) -> Self {
        self.inner = self.inner.with_width_from_item(item_index);
        self
    }
}

impl Styled for VirtualList {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.inner.style()
    }
}

impl IntoElement for VirtualList {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let Self {
            inner,
            scroll_handle,
            show_scrollbar,
        } = self;

        let wrapper = div().id("virtual-list-wrapper").size_full();

        if show_scrollbar {
            let handle = scroll_handle.expect("scrollbar attaches a handle");
            let move_handle = handle.clone();
            let up_handle = handle;
            wrapper
                .on_mouse_move(move |event, window, cx| {
                    let Some(grab) = move_handle.drag_offset.get() else {
                        return;
                    };
                    if event.pressed_button != Some(MouseButton::Left) {
                        move_handle.drag_offset.set(None);
                        return;
                    }
                    let viewport = move_handle.viewport.get();
                    let content = move_handle.content_height.get();
                    let Some(m) = ThumbMetrics::compute(viewport.size.height, content) else {
                        return;
                    };
                    cx.stop_propagation();
                    let desired_top = event.position.y - viewport.top() - grab;
                    let new_scroll = m.scroll_for_thumb_top(desired_top);
                    let base = move_handle.inner.0.borrow().base_handle.clone();
                    let mut off = base.offset();
                    off.y = -new_scroll;
                    base.set_offset(off);
                    window.refresh();
                })
                .on_mouse_up(MouseButton::Left, move |_, _window, _cx| {
                    up_handle.drag_offset.set(None);
                })
                .child(inner)
        } else {
            wrapper.child(inner)
        }
    }
}

struct VirtualListScrollbar {
    handle: VirtualListScrollHandle,
}

impl UniformListDecoration for VirtualListScrollbar {
    fn compute(
        &self,
        _visible_range: Range<usize>,
        bounds: Bounds<Pixels>,
        scroll_offset: Point<Pixels>,
        item_height: Pixels,
        item_count: usize,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let colors = cx.theme().colors();
        let content_px = item_height * item_count as f32;
        let viewport_top = bounds.top() - scroll_offset.y;
        self.handle.viewport.set(Bounds {
            origin: Point::new(bounds.origin.x, viewport_top),
            size: bounds.size,
        });
        self.handle.content_height.set(content_px);

        let Some(metrics) = ThumbMetrics::compute(bounds.size.height, content_px) else {
            return div().size_0().into_any_element();
        };
        let thumb_top = px(metrics.thumb_top_for_scroll(-scroll_offset.y.as_f32()));
        let thumb_h = px(metrics.thumb_h);
        let click_handle = self.handle.clone();

        div()
            .size_full()
            .relative()
            .child(
                div()
                    .absolute()
                    .top(-scroll_offset.y)
                    .right_0()
                    .w(SCROLLBAR_THICKNESS)
                    .h(bounds.size.height)
                    .bg(colors.element_background)
                    .rounded(Radius::Full.pixels())
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        cx.stop_propagation();
                        let click_y = event.position.y - viewport_top;
                        let on_thumb = click_y >= thumb_top && click_y <= thumb_top + thumb_h;
                        if on_thumb {
                            click_handle.drag_offset.set(Some(click_y - thumb_top));
                            return;
                        }
                        let desired_top = click_y - thumb_h / 2.0;
                        let new_scroll = metrics.scroll_for_thumb_top(desired_top);
                        let base = click_handle.inner.0.borrow().base_handle.clone();
                        let mut off = base.offset();
                        off.y = -new_scroll;
                        base.set_offset(off);
                        click_handle.drag_offset.set(Some(thumb_h / 2.0));
                        window.refresh();
                    })
                    .child(
                        div()
                            .absolute()
                            .top(thumb_top)
                            .left_0()
                            .w_full()
                            .h(thumb_h)
                            .bg(colors.border)
                            .rounded(Radius::Full.pixels()),
                    ),
            )
            .into_any_element()
    }
}
