//! TreeViewItem - a hierarchical list row with expand/collapse support.
//!
//! Root items display a [`Disclosure`] toggle and a label. Non-root items
//! show an indentation line followed by their label. Selection and expansion
//! state are owned by the parent - the item is a stateless `RenderOnce`.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{
    App, ClickEvent, ElementId, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString,
    Window, div, prelude::*, px,
};

use crate::components::disclosure::Disclosure;
use crate::components::icon::IconName;
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::traits::{
    ClickHandler, Clickable, Disableable, MouseDownHandler, ToggleState, Toggleable, TooltipBuilder,
};

/// One row inside a tree view.
#[derive(IntoElement)]
#[must_use = "TreeViewItem does nothing unless rendered"]
pub struct TreeViewItem {
    id: ElementId,
    label: SharedString,
    expanded: bool,
    selected: bool,
    disabled: bool,
    root_item: bool,
    tooltip: Option<TooltipBuilder>,
    on_click: Option<ClickHandler>,
    on_toggle: Option<ClickHandler>,
    on_secondary_mouse_down: Option<MouseDownHandler>,
}

impl TreeViewItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            expanded: false,
            selected: false,
            disabled: false,
            root_item: false,
            tooltip: None,
            on_click: None,
            on_toggle: None,
            on_secondary_mouse_down: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn on_secondary_mouse_down(
        mut self,
        handler: impl Fn(&gpui::MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_secondary_mouse_down = Some(Rc::new(handler));
        self
    }

    pub fn tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static,
    ) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn on_toggle(
        mut self,
        on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(on_toggle));
        self
    }

    pub fn root_item(mut self, root_item: bool) -> Self {
        self.root_item = root_item;
        self
    }
}

impl Disableable for TreeViewItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for TreeViewItem {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.selected = state.into().selected();
        self
    }
}

impl RenderOnce for TreeViewItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let selected_bg = colors.element_active.opacity(0.5);
        let selected_border = colors.border.opacity(0.4);

        let indentation_line = h_flex()
            .h(px(24.0))
            .w(px(22.0))
            .flex_none()
            .justify_center()
            .child(div().w_px().h_full().bg(colors.border.opacity(0.5)));

        h_flex().id(self.id).w_full().child(
            h_flex()
                .id("inner_tree_view_item")
                .cursor_pointer()
                .size_full()
                .h(px(24.0))
                .pl(px(2.0))
                .pr(Spacing::Small.pixels())
                .gap(Spacing::Medium.pixels())
                .rounded(px(2.0))
                .border_1()
                .border_color(gpui::transparent_black())
                .when(self.selected, |this| {
                    this.border_color(selected_border).bg(selected_bg)
                })
                .hover(|s| s.bg(colors.element_hover))
                .map(|this| {
                    let label = self.label;

                    if self.root_item {
                        this.child(
                            Disclosure::new("toggle", self.expanded)
                                .when_some(self.on_toggle.clone(), |disclosure, on_toggle| {
                                    disclosure.on_click(move |event, window, cx| {
                                        on_toggle(event, window, cx)
                                    })
                                })
                                .opened_icon(IconName::ChevronDown)
                                .closed_icon(IconName::ChevronRight),
                        )
                        .child(
                            Label::new(label).when(!self.selected, |this| this.color(Color::Muted)),
                        )
                    } else {
                        this.child(indentation_line)
                            .child(
                                h_flex().w_full().flex_grow(1.).child(
                                    Label::new(label)
                                        .when(!self.selected, |this| this.color(Color::Muted)),
                                ),
                            )
                    }
                })
                .when_some(
                    self.on_click.filter(|_| !self.disabled),
                    |this, on_click| {
                        this.on_click(move |event, window, cx| on_click(event, window, cx))
                    },
                )
                .when_some(self.on_secondary_mouse_down, |this, on_mouse_down| {
                    this.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        on_mouse_down(event, window, cx)
                    })
                })
                .when_some(self.tooltip, |this, tooltip| {
                    this.tooltip(move |window, cx| tooltip(window, cx))
                }),
        )
    }
}
