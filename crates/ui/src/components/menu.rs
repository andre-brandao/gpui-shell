//! Menu - vertical list of clickable items, separators, and headers,
//! packaged as the content of a [`Popover`](super::popover::Popover) and
//! navigable from the keyboard.
//!
//! ## Why this is a stateful entity
//!
//! Menus need persistent state - `selected_index` for the keyboard cursor,
//! a focus handle so the dispatch tree routes Down / Up / Enter / Esc to
//! the right element, and dismissal via [`gpui::DismissEvent`] so callers
//! can subscribe instead of threading close-callbacks through every entry.
//! All three demand a `Render` (entity) implementation rather than the
//! `RenderOnce` builder this used to be.
//!
//! ## Building one
//!
//! Build the menu inside `cx.new` (so it can grab a focus handle from the
//! current `Context`) and chain entries onto it. Then render the entity
//! directly - `Entity<Menu>` is `IntoElement` because [`Menu`] implements
//! [`Render`].
//!
//! ```ignore
//! let menu = cx.new(|cx| {
//!     Menu::new(cx)
//!         .header("File")
//!         .entry("new", "New File", |_, _, _| {})
//!         .keybinding_entry("save", "Save", ["Ctrl", "S"], |_, _, _| {})
//!         .separator()
//!         .disabled_entry("dis", "Unavailable")
//! });
//!
//! cx.subscribe(&menu, |this, _, _: &gpui::DismissEvent, cx| {
//!     this.menu_open = false;
//!     cx.notify();
//! }).detach();
//! ```
//!
//! ## Keyboard navigation
//!
//! While focused, the menu responds to:
//!
//! | Key      | Action            |
//! |----------|-------------------|
//! | Down     | `SelectNext`      |
//! | Up       | `SelectPrevious`  |
//! | Home     | `SelectFirst`     |
//! | End      | `SelectLast`      |
//! | Enter    | `Confirm`         |
//! | Escape   | `Cancel`          |
//!
//! `Confirm` invokes the selected entry's `on_click` handler (no event
//! payload - the handler is the [`DismissHandler`](crate::traits::DismissHandler)
//! shape, called identically from mouse click and keyboard confirm) and
//! emits a [`DismissEvent`]. `Cancel` only emits the dismiss. The default
//! key bindings are installed by [`crate::init`] under the `Menu` key
//! context.
//!
//! Submenus, search/filter, and the `SelectChild` / `SelectParent`
//! navigation actions from zed's `ContextMenu` are intentionally out of
//! scope - when a real consumer needs them, port them in.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{
    AnyElement, App, ClickEvent, Context, DismissEvent, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, IntoElement, ParentElement, Pixels, Render, SharedString, Window,
    actions, div, prelude::*, px,
};
use smallvec::SmallVec;

use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::keybinding::KeyBinding;
use crate::components::label::{Label, LabelCommon};
use crate::components::popover::Popover;
use crate::components::stack::{h_flex, v_flex};
use crate::theme::TextSize;
use crate::traits::DismissHandler;

// -----------------------------------------------------------------------
// Actions
// -----------------------------------------------------------------------
//
// Action namespace is `engram_menu` to avoid colliding with any host-app
// `menu::*` actions (zed's own `ContextMenu` lives in the `menu` namespace,
// so we deliberately stay out of it).

actions!(
    engram_menu,
    [
        SelectFirst,
        SelectNext,
        SelectPrevious,
        SelectLast,
        Confirm,
        Cancel
    ]
);

/// One row inside a [`Menu`].
pub enum MenuItem {
    /// A clickable entry with a label, optional leading icon, and an
    /// optional trailing keybinding hint.
    Entry {
        id: ElementId,
        label: SharedString,
        icon: Option<IconName>,
        keybinding: Option<Vec<SharedString>>,
        disabled: bool,
        on_click: Option<DismissHandler>,
    },
    /// A non-interactive header row, used to title a group of entries.
    Header(SharedString),
    /// A 1-pixel divider between groups.
    Separator,
}

impl MenuItem {
    fn is_selectable(&self) -> bool {
        matches!(
            self,
            MenuItem::Entry {
                disabled: false,
                on_click: Some(_),
                ..
            }
        )
    }
}

/// A vertical menu, rendered as the body of a popover. See the module
/// docs for usage.
pub struct Menu {
    focus_handle: FocusHandle,
    items: SmallVec<[MenuItem; 6]>,
    min_width: Option<Pixels>,
    selected_index: Option<usize>,
}

impl Menu {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            items: SmallVec::new(),
            min_width: Some(px(180.0)),
            selected_index: None,
        }
    }

    /// Convenience constructor that mirrors the old `RenderOnce` API: build
    /// the menu inside a `cx.new` block in one call.
    pub fn build(cx: &mut App, f: impl FnOnce(&mut Context<Self>) -> Self) -> Entity<Self> {
        cx.new(f)
    }

    /// Override the popover's minimum width. The default (180px) gives
    /// menu rows a stable, non-jittery width.
    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Append a clickable entry.
    pub fn entry(
        mut self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(MenuItem::Entry {
            id: id.into(),
            label: label.into(),
            icon: None,
            keybinding: None,
            disabled: false,
            on_click: Some(Rc::new(on_click)),
        });
        self
    }

    /// Append an entry with a leading icon.
    pub fn entry_with_icon(
        mut self,
        id: impl Into<ElementId>,
        icon: IconName,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(MenuItem::Entry {
            id: id.into(),
            label: label.into(),
            icon: Some(icon),
            keybinding: None,
            disabled: false,
            on_click: Some(Rc::new(on_click)),
        });
        self
    }

    /// Append an entry with a trailing key chip strip (e.g. `["Cmd", "S"]`).
    pub fn keybinding_entry<I, S>(
        mut self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        keys: I,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.items.push(MenuItem::Entry {
            id: id.into(),
            label: label.into(),
            icon: None,
            keybinding: Some(keys.into_iter().map(Into::into).collect()),
            disabled: false,
            on_click: Some(Rc::new(on_click)),
        });
        self
    }

    /// Append a disabled (non-clickable, muted) entry.
    pub fn disabled_entry(
        mut self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.items.push(MenuItem::Entry {
            id: id.into(),
            label: label.into(),
            icon: None,
            keybinding: None,
            disabled: true,
            on_click: None,
        });
        self
    }

    /// Append a non-interactive header that titles the next group of entries.
    pub fn header(mut self, label: impl Into<SharedString>) -> Self {
        self.items.push(MenuItem::Header(label.into()));
        self
    }

    /// Append a 1px divider between groups.
    pub fn separator(mut self) -> Self {
        self.items.push(MenuItem::Separator);
        self
    }

    /// Read the keyboard cursor position. `None` means nothing is selected.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Borrow the focus handle so the parent overlay can route focus to it
    /// when the menu opens.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    // ------------------------------------------------------------------
    // Selection helpers
    // ------------------------------------------------------------------

    fn first_selectable(&self) -> Option<usize> {
        self.items.iter().position(MenuItem::is_selectable)
    }

    fn last_selectable(&self) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .rev()
            .find(|(_, item)| item.is_selectable())
            .map(|(ix, _)| ix)
    }

    // ------------------------------------------------------------------
    // Action handlers
    // ------------------------------------------------------------------

    pub fn select_first(&mut self, _: &SelectFirst, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.first_selectable() {
            self.selected_index = Some(ix);
            cx.notify();
        }
    }

    pub fn select_last(&mut self, _: &SelectLast, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.last_selectable() {
            self.selected_index = Some(ix);
            cx.notify();
        }
    }

    pub fn select_next(&mut self, _: &SelectNext, _window: &mut Window, cx: &mut Context<Self>) {
        let start = self.selected_index.map(|i| i + 1).unwrap_or(0);
        let next = (start..self.items.len()).find(|&i| {
            self.items
                .get(i)
                .map(MenuItem::is_selectable)
                .unwrap_or(false)
        });
        if let Some(ix) = next.or_else(|| self.first_selectable()) {
            self.selected_index = Some(ix);
            cx.notify();
        }
    }

    pub fn select_previous(
        &mut self,
        _: &SelectPrevious,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prev = self.selected_index.and_then(|cur| {
            (0..cur).rev().find(|&i| {
                self.items
                    .get(i)
                    .map(MenuItem::is_selectable)
                    .unwrap_or(false)
            })
        });
        if let Some(ix) = prev.or_else(|| self.last_selectable()) {
            self.selected_index = Some(ix);
            cx.notify();
        }
    }

    pub fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.selected_index else {
            return;
        };
        let handler = match self.items.get(ix) {
            Some(MenuItem::Entry {
                on_click: Some(handler),
                disabled: false,
                ..
            }) => handler.clone(),
            _ => return,
        };
        handler(window, cx);
        cx.emit(DismissEvent);
    }

    pub fn cancel(&mut self, _: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for Menu {}

impl Focusable for Menu {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let mut popover = Popover::new();
        if let Some(w) = self.min_width {
            popover = popover.min_width(w);
        }

        let selected = self.selected_index;
        let rows: Vec<AnyElement> = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| match item {
                MenuItem::Separator => div()
                    .my(px(2.0))
                    .h(px(1.0))
                    .bg(colors.border_variant)
                    .into_any_element(),
                MenuItem::Header(text) => div()
                    .px(Spacing::Medium.pixels())
                    .pt(px(6.0))
                    .pb(px(2.0))
                    .child(
                        Label::new(text.clone())
                            .size(TextSize::XSmall)
                            .color(Color::Muted),
                    )
                    .into_any_element(),
                MenuItem::Entry {
                    id,
                    label,
                    icon,
                    keybinding,
                    disabled,
                    on_click,
                } => {
                    let disabled = *disabled;
                    let label_color = if disabled {
                        Color::Disabled
                    } else {
                        Color::Default
                    };
                    let is_selected = selected == Some(ix);
                    let row_id = id.clone();
                    let label = label.clone();
                    let icon = *icon;
                    let keybinding = keybinding.clone();
                    let row_handler = on_click.clone();

                    h_flex()
                        .id(row_id)
                        .w_full()
                        .gap(Spacing::Small.pixels())
                        .px(Spacing::Medium.pixels())
                        .py(px(4.0))
                        .items_center()
                        .when(is_selected, |this| this.bg(colors.ghost_element_selected))
                        .when(!disabled, |this| {
                            this.cursor_pointer()
                                .hover(|s| s.bg(colors.ghost_element_hover))
                        })
                        .when_some(icon, |this, icon| {
                            this.child(Icon::new(icon).size(IconSize::Small).color(if disabled {
                                Color::Disabled
                            } else {
                                Color::Muted
                            }))
                        })
                        .child(
                            div()
                                .flex_grow(1.)
                                .child(Label::new(label).color(label_color)),
                        )
                        .when_some(keybinding, |this, keys| this.child(KeyBinding::new(keys)))
                        .when(!disabled && row_handler.is_some(), |this| {
                            let handler = row_handler.clone().expect("guarded above");
                            this.on_click(cx.listener(
                                move |menu, _event: &ClickEvent, window, cx| {
                                    menu.selected_index = Some(ix);
                                    handler(window, cx);
                                    cx.emit(DismissEvent);
                                },
                            ))
                        })
                        .into_any_element()
                }
            })
            .collect();

        // Outer interactive root: tracks focus, owns the key context, and
        // hosts the action listeners. The popover then provides the visual
        // chrome (background, border, shadow).
        div()
            .id("engram-menu")
            .key_context("Menu")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_first))
            .on_action(cx.listener(Self::select_last))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::cancel))
            .child(popover.child(v_flex().gap(Spacing::None.pixels()).children(rows)))
    }
}

// -----------------------------------------------------------------------
// Keybinding registration
// -----------------------------------------------------------------------

/// Register the default keyboard navigation bindings for [`Menu`]. Called
/// from [`crate::init`]; exposed standalone in case an app wants to
/// initialize engram without binding our keys.
pub fn bind_menu_keys(cx: &mut App) {
    use gpui::KeyBinding;
    cx.bind_keys([
        KeyBinding::new("down", SelectNext, Some("Menu")),
        KeyBinding::new("up", SelectPrevious, Some("Menu")),
        KeyBinding::new("home", SelectFirst, Some("Menu")),
        KeyBinding::new("end", SelectLast, Some("Menu")),
        KeyBinding::new("enter", Confirm, Some("Menu")),
        KeyBinding::new("escape", Cancel, Some("Menu")),
    ]);
}
