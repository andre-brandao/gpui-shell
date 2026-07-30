//! [`DropdownMenu`] - a stateful menu anchored to a trigger button.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Anchor, App, Bounds, Context, Entity, IntoElement, ParentElement, Pixels, Render, SharedString,
    Styled, Subscription, Window, canvas, div, prelude::*,
};

use crate::components::button::{Button, ButtonCommon, ButtonSize, ButtonStyle};
use crate::components::icon::IconName;
use crate::components::menu::Menu;
use crate::components::popover::anchored_popover;
use crate::traits::{Clickable, Disableable, Toggleable};

pub struct DropdownMenu {
    id: SharedString,
    label: SharedString,
    menu: Entity<Menu>,
    is_open: bool,
    trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    style: ButtonStyle,
    size: ButtonSize,
    icon: Option<IconName>,
    anchor: Anchor,
    full_width: bool,
    disabled: bool,
    _dismiss_sub: Subscription,
}

impl DropdownMenu {
    /// Create a dropdown with a trigger label and a menu built from the
    /// provided closure.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
        build_menu: impl FnOnce(Menu) -> Menu,
    ) -> Self {
        let menu = cx.new(|cx| build_menu(Menu::new(cx)));
        let sub = cx.subscribe(&menu, |this: &mut Self, _, _: &gpui::DismissEvent, cx| {
            this.is_open = false;
            cx.notify();
        });
        Self {
            id: id.into(),
            label: label.into(),
            menu,
            is_open: false,
            trigger_bounds: Rc::new(Cell::new(None)),
            style: ButtonStyle::Outlined,
            size: ButtonSize::Default,
            icon: Some(IconName::ChevronDown),
            anchor: Anchor::TopLeft,
            full_width: false,
            disabled: false,
            _dismiss_sub: sub,
        }
    }

    /// Create a dropdown from a pre-built [`Menu`] entity.
    pub fn from_menu(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        menu: Entity<Menu>,
        cx: &mut Context<Self>,
    ) -> Self {
        let sub = cx.subscribe(&menu, |this: &mut Self, _, _: &gpui::DismissEvent, cx| {
            this.is_open = false;
            cx.notify();
        });
        Self {
            id: id.into(),
            label: label.into(),
            menu,
            is_open: false,
            trigger_bounds: Rc::new(Cell::new(None)),
            style: ButtonStyle::Outlined,
            size: ButtonSize::Default,
            icon: Some(IconName::ChevronDown),
            anchor: Anchor::TopLeft,
            full_width: false,
            disabled: false,
            _dismiss_sub: sub,
        }
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn no_icon(mut self) -> Self {
        self.icon = None;
        self
    }

    pub fn anchor(mut self, corner: Anchor) -> Self {
        self.anchor = corner;
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Render for DropdownMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let bounds_slot = self.trigger_bounds.clone();
        let menu_entity = self.menu.clone();
        let is_open = self.is_open;
        let disabled = self.disabled;

        let open_handler = {
            let weak = weak.clone();
            move |_event: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                weak.update(cx, |this, cx| {
                    this.is_open = !this.is_open;
                    if this.is_open {
                        let handle = this.menu.read(cx).focus_handle().clone();
                        window.focus(&handle, cx);
                    }
                    cx.notify();
                })
                .ok();
            }
        };

        let mut trigger = Button::new(self.id.clone(), self.label.clone())
            .style(self.style)
            .size(self.size);

        if let Some(icon) = self.icon {
            trigger = trigger.icon(icon);
        }
        if disabled {
            trigger = trigger.disabled(true);
        } else {
            trigger = trigger.on_click(open_handler);
        }
        if is_open {
            trigger = trigger.toggle_state(true);
        }

        let trigger_with_bounds = div()
            .relative()
            .when(self.full_width, |this| this.w_full())
            .child(trigger)
            .child(
                canvas(
                    move |bounds, _window, _cx| {
                        bounds_slot.set(Some(bounds));
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0()
                .size_full(),
            );

        let trigger_bounds = self.trigger_bounds.get();
        let anchor_focus = self.menu.read(cx).focus_handle().clone();
        let anchor_corner = self.anchor;
        let weak_for_dismiss = weak;

        div()
            .when(self.full_width, |this| this.w_full())
            .child(trigger_with_bounds)
            .when(is_open && !disabled, |this| {
                let Some(bounds) = trigger_bounds else {
                    return this;
                };
                this.child(anchored_popover(
                    anchor_focus,
                    anchor_corner,
                    bounds,
                    menu_entity,
                    move |_window, cx| {
                        weak_for_dismiss
                            .update(cx, |this, cx| {
                                this.is_open = false;
                                cx.notify();
                            })
                            .ok();
                    },
                ))
            })
    }
}
