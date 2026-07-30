//! Accordion - a vertically stacked set of collapsible sections.
//!
//! Each [`AccordionItem`] is a header row (with a [`Disclosure`] chevron)
//! toggling a content body. The parent owns the expanded state.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};

use crate::components::disclosure::Disclosure;
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::v_flex;
use crate::theme::TextSize;
use crate::traits::{ClickHandler, Clickable, Disableable, StyledExt};

// -------------------- AccordionItem --------------------

/// A single collapsible section inside an [`Accordion`].
#[derive(IntoElement)]
#[must_use = "AccordionItem does nothing unless rendered"]
pub struct AccordionItem {
    id: ElementId,
    title: SharedString,
    is_expanded: bool,
    disabled: bool,
    on_toggle: Option<ClickHandler>,
    body: Option<AnyElement>,
}

impl AccordionItem {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        is_expanded: bool,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            is_expanded,
            disabled: false,
            on_toggle: None,
            body: None,
        }
    }

    /// Set the content shown when this item is expanded.
    pub fn body(mut self, body: impl IntoElement) -> Self {
        self.body = Some(body.into_any_element());
        self
    }

    /// Register a toggle handler, fired when the header is clicked.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl Disableable for AccordionItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for AccordionItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let label_color = if self.disabled {
            Color::Disabled
        } else {
            Color::Default
        };

        let mut disclosure =
            Disclosure::new(self.id.clone(), self.is_expanded).disabled(self.disabled);
        if let Some(handler) = self.on_toggle {
            disclosure = disclosure.on_click(move |ev, window, cx| handler(ev, window, cx));
        }

        let header = div()
            .h_flex()
            .gap(Spacing::XSmall.pixels())
            .py(Spacing::Small.pixels())
            .child(disclosure)
            .child(
                Label::new(self.title)
                    .size(TextSize::Small)
                    .color(label_color),
            );

        v_flex()
            .border_b_1()
            .border_color(colors.border_variant)
            .child(header)
            .when(self.is_expanded, |this| {
                this.when_some(self.body, |this, body| {
                    this.child(
                        div()
                            .pl(Spacing::Large.pixels())
                            .pb(Spacing::Small.pixels())
                            .child(body),
                    )
                })
            })
    }
}

// -------------------- Accordion --------------------

/// A vertically stacked set of [`AccordionItem`]s.
#[derive(IntoElement)]
#[must_use = "Accordion does nothing unless rendered"]
pub struct Accordion {
    children: Vec<AnyElement>,
}

impl Accordion {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Append an [`AccordionItem`] to the accordion.
    pub fn child(mut self, item: impl IntoElement) -> Self {
        self.children.push(item.into_any_element());
        self
    }
}

impl Default for Accordion {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        v_flex().children(self.children)
    }
}
