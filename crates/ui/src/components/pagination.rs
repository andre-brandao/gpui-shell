//! Pagination - page navigation with truncation and boundary controls.
//!
//! Renders previous / next buttons and a row of page numbers with
//! ellipsis placeholders when the total page count exceeds the visible
//! window. The component is stateless - the parent owns the current page
//! and receives clicks via a handler.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Radius, Spacing};
use gpui::{
    App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div, prelude::*, px,
};

use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::Disableable;
use crate::traits::handlers::UsizeHandler;

/// Internal representation of a slot in the page row.
#[derive(Debug, Clone)]
enum PageSlot {
    /// A clickable page number (1-based).
    Page(usize),
    /// An ellipsis representing a gap between visible pages.
    Ellipsis,
}

/// A page navigation control with prev/next buttons and numbered pages.
#[derive(IntoElement)]
#[must_use = "Pagination does nothing unless rendered"]
pub struct Pagination {
    id: ElementId,
    current_page: usize,
    total_pages: usize,
    visible_pages: usize,
    disabled: bool,
    on_click: Option<UsizeHandler>,
}

impl Pagination {
    /// Create a new pagination control. Pages are 1-based.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            current_page: 1,
            total_pages: 1,
            visible_pages: 5,
            disabled: false,
            on_click: None,
        }
    }

    /// Set the current active page (1-based, clamped to valid range).
    pub fn current_page(mut self, page: usize) -> Self {
        self.current_page = page.max(1);
        self
    }

    /// Set the total number of pages.
    pub fn total_pages(mut self, pages: usize) -> Self {
        self.total_pages = pages.max(1);
        self
    }

    /// Set how many page buttons are visible before truncation (minimum 5).
    pub fn visible_pages(mut self, max: usize) -> Self {
        self.visible_pages = max.max(5);
        self
    }

    /// Register a click handler, invoked with the 1-based page number.
    pub fn on_click(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Disableable for Pagination {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Compute which page slots to display, inserting ellipsis where needed.
fn page_slots(current: usize, total: usize, max_visible: usize) -> Vec<PageSlot> {
    if total <= 1 {
        return vec![PageSlot::Page(1)];
    }

    // If everything fits, show all pages.
    if total <= max_visible {
        return (1..=total).map(PageSlot::Page).collect();
    }

    // Number of pages shown on each side of the current page (excluding
    // first, last, and current itself).
    let side = (max_visible.saturating_sub(3)) / 2;
    let mut start = current.saturating_sub(side).max(2);
    let mut end = current.saturating_add(side).min(total - 1);

    // Adjust window if it's pinned against a boundary.
    let window_size = end - start + 1;
    let needed = max_visible.saturating_sub(2); // slots between first and last
    if window_size < needed {
        if start == 2 {
            end = (start + needed - 1).min(total - 1);
        } else {
            start = end.saturating_sub(needed - 1).max(2);
        }
    }

    let mut slots = Vec::with_capacity(max_visible + 2);
    slots.push(PageSlot::Page(1));

    if start > 2 {
        slots.push(PageSlot::Ellipsis);
    }

    for p in start..=end {
        slots.push(PageSlot::Page(p));
    }

    if end < total - 1 {
        slots.push(PageSlot::Ellipsis);
    }

    slots.push(PageSlot::Page(total));
    slots
}

impl RenderOnce for Pagination {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let current = self.current_page.clamp(1, self.total_pages);
        let total = self.total_pages;

        let btn_size = px(32.0);
        let radius = Radius::XSmall.pixels();

        let nav_button = |id_suffix: &str,
                          icon: IconName,
                          target_page: usize,
                          enabled: bool,
                          handler: &Option<UsizeHandler>| {
            let btn = div()
                .id(SharedString::from(format!("{}-{}", self.id, id_suffix)))
                .size(btn_size)
                .flex()
                .items_center()
                .justify_center()
                .rounded(radius)
                .border_1()
                .border_color(colors.border_variant)
                .child(Icon::new(icon).size(IconSize::Small).color(if enabled {
                    Color::Default
                } else {
                    Color::Disabled
                }));

            if enabled && !self.disabled {
                let btn = btn
                    .cursor_pointer()
                    .hover(|s| s.bg(colors.ghost_element_hover));
                if let Some(h) = handler.clone() {
                    btn.on_click(move |_, window, cx| h(target_page, window, cx))
                } else {
                    btn
                }
            } else {
                btn
            }
        };

        let prev = nav_button(
            "prev",
            IconName::ChevronLeft,
            current.saturating_sub(1).max(1),
            current > 1,
            &self.on_click,
        );

        let next = nav_button(
            "next",
            IconName::ChevronRight,
            (current + 1).min(total),
            current < total,
            &self.on_click,
        );

        let slots = page_slots(current, total, self.visible_pages);

        h_flex()
            .gap(Spacing::XXSmall.pixels())
            .items_center()
            .child(prev)
            .children(slots.into_iter().map(|slot| {
                match slot {
                    PageSlot::Page(page) => {
                        let is_current = page == current;
                        let btn = div()
                            .id(SharedString::from(format!("{}-p{}", self.id, page)))
                            .min_w(btn_size)
                            .h(btn_size)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(radius)
                            .border_1()
                            .px(Spacing::XSmall.pixels());

                        let btn = if is_current {
                            btn.bg(colors.accent).border_color(colors.accent).child(
                                Label::new(format!("{page}"))
                                    .size(TextSize::Small)
                                    .color(Color::Custom(colors.background)),
                            )
                        } else {
                            let btn = btn.border_color(colors.border_variant).child(
                                Label::new(format!("{page}"))
                                    .size(TextSize::Small)
                                    .color(Color::Default),
                            );
                            if !self.disabled {
                                let btn = btn
                                    .cursor_pointer()
                                    .hover(|s| s.bg(colors.ghost_element_hover));
                                if let Some(h) = self.on_click.clone() {
                                    btn.on_click(move |_, window, cx| h(page, window, cx))
                                } else {
                                    btn
                                }
                            } else {
                                btn
                            }
                        };
                        btn.into_any_element()
                    }
                    PageSlot::Ellipsis => div()
                        .min_w(btn_size)
                        .h(btn_size)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Label::new("...").size(TextSize::Small).color(Color::Muted))
                        .into_any_element(),
                }
            }))
            .child(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(slots: &[PageSlot]) -> Vec<Option<usize>> {
        slots
            .iter()
            .map(|s| match s {
                PageSlot::Page(p) => Some(*p),
                PageSlot::Ellipsis => None,
            })
            .collect()
    }

    #[test]
    fn slots_all_visible() {
        let slots = page_slots(1, 5, 5);
        assert_eq!(slots.len(), 5);
        assert!(slots.iter().all(|s| matches!(s, PageSlot::Page(_))));
    }

    #[test]
    fn slots_truncate_right() {
        let slots = page_slots(1, 20, 7);
        assert_eq!(
            pages(&slots),
            vec![
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                None,
                Some(20)
            ],
        );
    }

    #[test]
    fn slots_truncate_left() {
        let slots = page_slots(20, 20, 7);
        assert_eq!(
            pages(&slots),
            vec![
                Some(1),
                None,
                Some(15),
                Some(16),
                Some(17),
                Some(18),
                Some(19),
                Some(20),
            ],
        );
    }

    #[test]
    fn slots_truncate_both() {
        let slots = page_slots(10, 20, 7);
        assert_eq!(
            pages(&slots),
            vec![
                Some(1),
                None,
                Some(8),
                Some(9),
                Some(10),
                Some(11),
                Some(12),
                None,
                Some(20),
            ],
        );
    }

    #[test]
    fn slots_single_page() {
        assert_eq!(page_slots(1, 1, 5).len(), 1);
    }

    #[test]
    fn slots_zero_total_degrades_to_single() {
        let slots = page_slots(1, 0, 5);
        assert_eq!(pages(&slots), vec![Some(1)]);
    }

    #[test]
    fn slots_boundary_page_two_does_not_produce_left_ellipsis() {
        let slots = page_slots(2, 20, 7);
        let first_ellipsis = slots
            .iter()
            .position(|s| matches!(s, PageSlot::Ellipsis))
            .unwrap();
        assert!(first_ellipsis > 2);
    }

    #[test]
    fn slots_window_widens_when_pinned_left() {
        let slots = page_slots(1, 20, 9);
        let visible_pages: Vec<usize> = slots
            .iter()
            .filter_map(|s| match s {
                PageSlot::Page(p) => Some(*p),
                _ => None,
            })
            .collect();
        assert!(visible_pages.contains(&1));
        assert!(visible_pages.contains(&20));
        assert_eq!(visible_pages.len(), slots.len() - 1);
    }

    #[test]
    fn slots_exact_fit_no_ellipsis() {
        let slots = page_slots(4, 7, 7);
        assert_eq!(slots.len(), 7);
        assert!(!slots.iter().any(|s| matches!(s, PageSlot::Ellipsis)));
    }

    #[test]
    fn slots_no_adjacent_ellipsis_to_boundary() {
        let slots = page_slots(3, 20, 7);
        let page_numbers: Vec<usize> = slots
            .iter()
            .filter_map(|s| match s {
                PageSlot::Page(p) => Some(*p),
                _ => None,
            })
            .collect();
        assert_eq!(page_numbers[0], 1);
        assert_eq!(*page_numbers.last().unwrap(), 20);
    }
}
