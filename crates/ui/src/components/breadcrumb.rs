//! Breadcrumb - a navigation path showing the user's location in a hierarchy.
//!
//! A [`Breadcrumb`] is a horizontal strip of [`BreadcrumbItem`]s separated
//! by chevrons. Each item can be clickable (an ancestor you can navigate
//! back to) or plain (the current location). The component is stateless -
//! the parent decides which item is "current" and wires click handlers.

use std::rc::Rc;

use crate::theme::{Color, Spacing};
use gpui::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window,
    prelude::*,
};

use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::{ClickHandler, Clickable, Disableable};

// -------------------- BreadcrumbItem --------------------

/// A single segment in a [`Breadcrumb`] path.
#[derive(IntoElement)]
#[must_use = "BreadcrumbItem does nothing unless rendered"]
pub struct BreadcrumbItem {
    id: ElementId,
    label: SharedString,
    icon: Option<IconName>,
    current: bool,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

impl BreadcrumbItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            current: false,
            disabled: false,
            on_click: None,
        }
    }

    /// Mark this item as the current (final) breadcrumb. Renders with
    /// default text color instead of the muted clickable style.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }

    /// Prepend an icon before the label text.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl Clickable for BreadcrumbItem {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn cursor_style(self, _cursor_style: gpui::CursorStyle) -> Self {
        self
    }
}

impl Disableable for BreadcrumbItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for BreadcrumbItem {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = if self.disabled {
            Color::Disabled
        } else if self.current {
            Color::Default
        } else {
            Color::Muted
        };
        let icon_color = color;

        let clickable = !self.disabled && !self.current && self.on_click.is_some();

        h_flex()
            .id(self.id)
            .gap(Spacing::Small.pixels())
            .when(clickable, |this| this.cursor_pointer())
            .when_some(self.icon, |this, icon| {
                this.child(Icon::new(icon).size(IconSize::Small).color(icon_color))
            })
            .child(
                Label::new(self.label)
                    .size(TextSize::Small)
                    .color(color)
                    .when(clickable, |this| this.underline()),
            )
            .when_some(
                (!self.disabled && !self.current)
                    .then_some(self.on_click)
                    .flatten(),
                |this, handler| this.on_click(move |event, window, cx| handler(event, window, cx)),
            )
    }
}

// -------------------- Breadcrumb --------------------

/// A horizontal navigation path.
#[derive(IntoElement)]
#[must_use = "Breadcrumb does nothing unless rendered"]
pub struct Breadcrumb {
    items: Vec<AnyElement>,
    separator: IconName,
}

impl Breadcrumb {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            separator: IconName::ChevronRight,
        }
    }

    /// Override the separator icon (default: `ChevronRight`).
    pub fn separator(mut self, icon: IconName) -> Self {
        self.separator = icon;
        self
    }

    /// Append a breadcrumb item.
    pub fn child(mut self, item: impl IntoElement) -> Self {
        self.items.push(item.into_any_element());
        self
    }
}

impl Default for Breadcrumb {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let sep = self.separator;

        let mut row = h_flex().gap(Spacing::XXSmall.pixels()).items_center();

        for (i, item) in self.items.into_iter().enumerate() {
            if i > 0 {
                row = row.child(Icon::new(sep).size(IconSize::XSmall).color(Color::Muted));
            }
            row = row.child(item);
        }

        row
    }
}
