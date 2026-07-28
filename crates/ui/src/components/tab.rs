//! TabBar / Tab - horizontal strip of selectable tabs.
//!
//! Engram's tab bar is a much-trimmed take on Zed's `TabBar` + `Tab` pair.
//! Zed's version supports drag-reorder, overflow scrolling, sticky tabs,
//! `start_children` / `end_children` action slots, and hooks into the
//! workspace pane system. Engram exposes:
//!
//! - [`Tab`]: a selectable, optionally-closable cell with a label and
//!   leading icon.
//! - [`TabBar`]: a horizontal container that arranges `Tab`s with a bottom
//!   border and a small leading/trailing gutter.
//!
//! As with most engram components, statefulness lives with the parent: the
//! caller decides which tab is selected and wires `on_click` / `on_close`
//! to the appropriate state mutations.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    Window, div, prelude::*, px,
};
use smallvec::SmallVec;

use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::{h_flex, v_flex};
use crate::traits::{ClickHandler, Disableable, ToggleState, Toggleable};

/// One selectable tab inside a [`TabBar`].
#[derive(IntoElement)]
#[must_use = "Tab does nothing unless rendered"]
pub struct Tab {
    id: ElementId,
    label: SharedString,
    icon: Option<IconName>,
    selected: bool,
    disabled: bool,
    on_click: Option<ClickHandler>,
    on_close: Option<ClickHandler>,
}

impl Tab {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            selected: false,
            disabled: false,
            on_click: None,
            on_close: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Show a small x next to the label and route its click to `handler`.
    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl Disableable for Tab {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for Tab {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.selected = state.into().selected();
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let label_color = if self.disabled {
            Color::Disabled
        } else if self.selected {
            Color::Default
        } else {
            Color::Muted
        };
        let bg = if self.selected {
            colors.elevated_surface_background
        } else {
            colors.background
        };
        // Underline the active tab with a 2px accent stripe via a child div.
        let underline_color = if self.selected {
            colors.accent
        } else {
            colors.border
        };

        let row = h_flex()
            .id(self.id.clone())
            .h(px(24.0))
            .px(Spacing::Medium.pixels())
            .gap(Spacing::XSmall.pixels())
            .items_center()
            .bg(bg)
            .when(!self.disabled && !self.selected, |this| {
                this.cursor_pointer()
                    .hover(|s| s.bg(colors.ghost_element_hover))
            })
            .when_some(self.icon, |this, icon| {
                this.child(
                    Icon::new(icon)
                        .size(IconSize::Small)
                        .color(if self.disabled {
                            Color::Disabled
                        } else {
                            Color::Muted
                        }),
                )
            })
            .child(Label::new(self.label).color(label_color))
            .when_some(self.on_close.clone(), |this, close| {
                let id = self.id.clone();
                this.child(
                    div()
                        .id((id, "close"))
                        .ml(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(colors.ghost_element_active))
                        .rounded(px(2.0))
                        .child(
                            Icon::new(IconName::Close)
                                .size(IconSize::XSmall)
                                .color(Color::Muted),
                        )
                        .on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            close(event, window, cx);
                        }),
                )
            })
            .when_some(
                (!self.disabled).then_some(self.on_click).flatten(),
                |this, handler| this.on_click(move |event, window, cx| handler(event, window, cx)),
            );

        // Wrap so the underline draws inside the tab bounds.
        v_flex()
            .child(row)
            .child(div().h(px(2.0)).bg(underline_color))
    }
}

/// A horizontal strip of [`Tab`]s.
#[derive(IntoElement)]
#[must_use = "TabBar does nothing unless rendered"]
pub struct TabBar {
    children: SmallVec<[AnyElement; 6]>,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
        }
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for TabBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for TabBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        h_flex()
            .w_full()
            .items_end()
            .bg(colors.background)
            .border_b_1()
            .border_color(colors.border)
            .children(self.children)
    }
}
