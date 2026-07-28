//! Navigable - a keyboard-navigation wrapper for scrollable views.
//!
//! Wraps a child element and a list of focusable entries. When the menu
//! `SelectNext` / `SelectPrevious` actions fire (arrow keys), focus cycles
//! through the entries in order and optionally scrolls them into view.

use gpui::{
    AnyElement, App, FocusHandle, IntoElement, RenderOnce, ScrollAnchor, ScrollHandle, Window, div,
    prelude::*,
};

use crate::components::menu;

/// An entry that can be navigated to within a [`Navigable`].
#[derive(Clone)]
pub struct NavigableEntry {
    pub focus_handle: FocusHandle,
    pub scroll_anchor: Option<ScrollAnchor>,
}

impl NavigableEntry {
    /// Create a new entry tied to a scroll handle so it auto-scrolls on focus.
    pub fn new(scroll_handle: &ScrollHandle, cx: &App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            scroll_anchor: Some(ScrollAnchor::for_handle(scroll_handle.clone())),
        }
    }

    /// Create an entry that can be focused but has no scroll anchor.
    pub fn focusable(cx: &App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            scroll_anchor: None,
        }
    }
}

/// An element wrapper that handles keyboard navigation between children.
#[derive(IntoElement)]
#[must_use = "Navigable does nothing unless rendered"]
pub struct Navigable {
    child: AnyElement,
    entries: Vec<NavigableEntry>,
}

impl Navigable {
    pub fn new(child: AnyElement) -> Self {
        Self {
            child,
            entries: Vec::new(),
        }
    }

    pub fn entry(mut self, entry: NavigableEntry) -> Self {
        self.entries.push(entry);
        self
    }

    fn find_focused(
        entries: &[NavigableEntry],
        window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        entries
            .iter()
            .position(|entry| entry.focus_handle.contains_focused(window, cx))
    }
}

impl RenderOnce for Navigable {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .on_action({
                let children = self.entries.clone();
                move |_: &menu::SelectNext, window, cx| {
                    let target = Self::find_focused(&children, window, cx)
                        .and_then(|index| index.checked_add(1).filter(|i| *i < children.len()))
                        .unwrap_or(0);
                    if let Some(entry) = children.get(target) {
                        entry.focus_handle.focus(window, cx);
                        if let Some(anchor) = &entry.scroll_anchor {
                            anchor.scroll_to(window, cx);
                        }
                    }
                }
            })
            .on_action({
                let children = self.entries;
                move |_: &menu::SelectPrevious, window, cx| {
                    let target = Self::find_focused(&children, window, cx)
                        .and_then(|index| index.checked_sub(1))
                        .or(children.len().checked_sub(1));
                    if let Some(entry) = target.and_then(|t| children.get(t)) {
                        entry.focus_handle.focus(window, cx);
                        if let Some(anchor) = &entry.scroll_anchor {
                            anchor.scroll_to(window, cx);
                        }
                    }
                }
            })
            .size_full()
            .child(self.child)
    }
}
