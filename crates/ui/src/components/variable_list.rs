//! VariableList - lazy-rendered scrollable list of variable-height rows.
//!
//! Thin wrapper around [`gpui::list`]. Companion to [`VirtualList`], which
//! wraps [`gpui::uniform_list`]: uniform rows give gpui the content size for
//! free, variable rows have to be laid out to be measured. They stay separate
//! types because the scroll-state shapes ([`ListState`] vs
//! [`UniformListScrollHandle`]) are incompatible.
//!
//! The scrollbar overlay reuses [`ThumbMetrics`] from
//! [`super::scroll_metrics`] and drives [`ListState`]'s scrollbar hooks.
use std::cell::Cell;
use std::rc::Rc;

use crate::theme::{ActiveTheme, Radius};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ListAlignment, ListState, MouseButton,
    ParentElement, Pixels, Point, RenderOnce, StyleRefinement, Styled, Window, div, list, px,
};

use super::scroll_metrics::{SCROLLBAR_THICKNESS, ThumbMetrics};

pub use gpui::ListAlignment as VariableListAlignment;

type RenderItemFn = dyn FnMut(usize, &mut Window, &mut App) -> AnyElement + 'static;

/// Scroll handle for a [`VariableList`].
#[derive(Clone)]
pub struct VariableListScrollHandle {
    state: ListState,
    /// `Some(offset_from_thumb_top)` while a scrollbar drag is in progress.
    drag_offset: Rc<Cell<Option<Pixels>>>,
}

impl Default for VariableListScrollHandle {
    /// Empty list, [`ListAlignment::Top`], no overdraw.
    fn default() -> Self {
        Self::new(0)
    }
}

impl VariableListScrollHandle {
    /// Build a new handle.
    pub fn new(item_count: usize) -> Self {
        Self {
            state: ListState::new(item_count, ListAlignment::Top, px(0.0)),
            drag_offset: Rc::new(Cell::new(None)),
        }
    }

    /// Build a new handle with explicit alignment and overdraw.
    pub fn with_config(item_count: usize, alignment: ListAlignment, overdraw: Pixels) -> Self {
        Self {
            state: ListState::new(item_count, alignment, overdraw),
            drag_offset: Rc::new(Cell::new(None)),
        }
    }

    /// Measure every row after each [`Self::reset`], instead of only the rows
    /// currently on screen.
    #[must_use]
    pub fn measure_all(self) -> Self {
        Self {
            state: self.state.measure_all(),
            drag_offset: self.drag_offset,
        }
    }

    /// Reset to a new item count.
    pub fn reset(&self, item_count: usize) {
        self.state.reset(item_count);
    }

    /// Scroll so the item at `ix` is fully visible.
    pub fn scroll_to_item(&self, ix: usize) {
        self.state.scroll_to_reveal_item(ix);
    }

    /// Access the underlying [`ListState`] for less-common operations
    /// (splicing, scroll handlers, follow-tail mode).
    pub fn as_list_state(&self) -> &ListState {
        &self.state
    }
}

/// Lazy-rendered list of variable-height rows.
#[derive(IntoElement)]
#[must_use = "VariableList does nothing unless rendered"]
pub struct VariableList {
    scroll_handle: VariableListScrollHandle,
    render_item: Option<Box<RenderItemFn>>,
    show_scrollbar: bool,
    style: StyleRefinement,
}

impl VariableList {
    /// Build a new variable-height list.
    pub fn new(
        handle: VariableListScrollHandle,
        render_item: impl FnMut(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            scroll_handle: handle,
            render_item: Some(Box::new(render_item)),
            show_scrollbar: false,
            style: StyleRefinement::default(),
        }
    }

    /// Overlay a themed scrollbar on the right edge of the list.
    pub fn scrollbar(mut self) -> Self {
        self.show_scrollbar = true;
        self
    }
}

impl Styled for VariableList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for VariableList {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let render_item = self.render_item.take().expect("VariableList::render_item");
        let mut inner = list(self.scroll_handle.state.clone(), render_item);
        *inner.style() = self.style;

        if !self.show_scrollbar {
            return div().size_full().child(inner).into_any_element();
        }
        inner = inner.pr(SCROLLBAR_THICKNESS);

        let colors = cx.theme().colors();
        let state = self.scroll_handle.state.clone();
        let viewport = state.viewport_bounds();
        let max = state.max_offset_for_scrollbar();
        let content_h = viewport.size.height + max.y;
        let offset = state.scroll_px_offset_for_scrollbar();
        let metrics = ThumbMetrics::compute(viewport.size.height, content_h);

        let move_handle = self.scroll_handle.clone();
        let up_handle = self.scroll_handle.clone();

        let mut wrapper = div()
            .id("variable-list-wrapper")
            .size_full()
            .relative()
            .on_mouse_move(move |event, window, cx| {
                let Some(grab) = move_handle.drag_offset.get() else {
                    return;
                };
                if event.pressed_button != Some(MouseButton::Left) {
                    move_handle.drag_offset.set(None);
                    move_handle.state.scrollbar_drag_ended();
                    return;
                }
                let viewport = move_handle.state.viewport_bounds();
                let max = move_handle.state.max_offset_for_scrollbar();
                let content_h = viewport.size.height + max.y;
                let Some(m) = ThumbMetrics::compute(viewport.size.height, content_h) else {
                    return;
                };
                cx.stop_propagation();
                let desired_top = event.position.y - viewport.top() - grab;
                let new_scroll = m.scroll_for_thumb_top(desired_top);
                move_handle
                    .state
                    .set_offset_from_scrollbar(Point::new(px(0.0), -new_scroll));
                window.refresh();
            })
            .on_mouse_up(MouseButton::Left, move |_, _window, _cx| {
                if up_handle.drag_offset.take().is_some() {
                    up_handle.state.scrollbar_drag_ended();
                }
            })
            .child(inner);

        if let Some(m) = metrics {
            let thumb_top = px(m.thumb_top_for_scroll(-offset.y.as_f32()));
            let thumb_h = px(m.thumb_h);
            let click_handle = self.scroll_handle.clone();

            let track = div()
                .absolute()
                .top(px(0.0))
                .right_0()
                .w(SCROLLBAR_THICKNESS)
                .h_full()
                .bg(colors.element_background)
                .rounded(Radius::Full.pixels())
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    cx.stop_propagation();
                    let viewport = click_handle.state.viewport_bounds();
                    let click_y = event.position.y - viewport.top();
                    let on_thumb = click_y >= thumb_top && click_y <= thumb_top + thumb_h;
                    click_handle.state.scrollbar_drag_started();
                    if on_thumb {
                        click_handle.drag_offset.set(Some(click_y - thumb_top));
                        return;
                    }
                    let desired_top = click_y - thumb_h / 2.0;
                    let new_scroll = m.scroll_for_thumb_top(desired_top);
                    click_handle
                        .state
                        .set_offset_from_scrollbar(Point::new(px(0.0), -new_scroll));
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
                );
            wrapper = wrapper.child(track);
        }

        wrapper.into_any_element()
    }
}
