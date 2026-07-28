use std::cell::Cell;
use std::rc::Rc;

use crate::prelude::*;
use gpui::{Bounds, Pixels, Subscription, canvas};

use crate::layout::{example, example_group};

pub struct MenuStory {
    menu_open: bool,
    menu: Entity<Menu>,
    menu_trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    _menu_sub: Subscription,
}

impl MenuStory {
    fn new(cx: &mut Context<Self>) -> Self {
        let menu = cx.new(|cx| {
            Menu::new(cx)
                .header("File")
                .entry_with_icon("menu-new", IconName::Plus, "New File", |_, _| {})
                .keybinding_entry("menu-save", "Save", ["Ctrl", "S"], |_, _| {})
                .keybinding_entry(
                    "menu-saveas",
                    "Save As...",
                    ["Ctrl", "Shift", "S"],
                    |_, _| {},
                )
                .separator()
                .header("Edit")
                .entry("menu-cut", "Cut", |_, _| {})
                .entry("menu-copy", "Copy", |_, _| {})
                .entry("menu-paste", "Paste", |_, _| {})
                .separator()
                .disabled_entry("menu-disabled", "Unavailable")
        });
        let sub = cx.subscribe(&menu, |this, _, _: &gpui::DismissEvent, cx| {
            this.menu_open = false;
            cx.notify();
        });
        Self {
            menu_open: false,
            menu,
            menu_trigger_bounds: Rc::new(Cell::new(None)),
            _menu_sub: sub,
        }
    }
}

impl Render for MenuStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let bounds_slot = self.menu_trigger_bounds.clone();
        let menu_entity = self.menu.clone();

        let open_handler = {
            let weak = weak.clone();
            move |_event: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                weak.update(cx, |this, cx| {
                    this.menu_open = !this.menu_open;
                    if this.menu_open {
                        let handle = this.menu.read(cx).focus_handle().clone();
                        window.focus(&handle, cx);
                    }
                    cx.notify();
                })
                .ok();
            }
        };

        let trigger = Button::new("btn-menu-trigger", "Open menu")
            .icon(IconName::ChevronDown)
            .style(ButtonStyle::Outlined)
            .on_click(open_handler);

        let trigger_with_capture = gpui::div().relative().child(trigger).child(
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

        let menu_open = self.menu_open;
        let trigger_bounds = self.menu_trigger_bounds.get();
        let anchor_focus = self.menu.read(cx).focus_handle().clone();
        let weak_for_dismiss = weak.clone();

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Menu (click to open - keyboard-navigable)",
            vec![example(
                "Anchored popover menu",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(trigger_with_capture)
                    .when(menu_open, |this| {
                        let Some(bounds) = trigger_bounds else {
                            return this;
                        };
                        this.child(anchored_popover(
                            anchor_focus,
                            gpui::Anchor::TopLeft,
                            bounds,
                            menu_entity,
                            move |_window, cx| {
                                weak_for_dismiss
                                    .update(cx, |this, cx| {
                                        this.menu_open = false;
                                        cx.notify();
                                    })
                                    .ok();
                            },
                        ))
                    })
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(MenuStory::new).into()
}
