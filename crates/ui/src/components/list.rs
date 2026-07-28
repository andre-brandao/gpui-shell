//! List and ListItem - the workhorse of sidebars, command palettes, and
//! settings panes.
//!
//! [`List`] is a thin vertical container: stack of children plus an optional
//! header and empty-state message. [`ListItem`] is a stateful row with an
//! optional start slot (icon / avatar), label content, an optional end slot
//! (accessory / chevron), click handler, and hover / selected styling.
//!
//! Beyond the basics, `ListItem` also supports tree-style indentation
//! ([`ListItem::indent_level`]), three density steps via [`ListItemSpacing`],
//! an "inset" mode that draws the indent gutter outside the item chrome,
//! optional `outlined` borders, hover-only end slots
//! ([`EndSlotVisibility::OnHover`]), and right-click handling. The shape
//! mirrors zed's `ListItem` so we can drop in tree pickers and command
//! palettes without re-litigating layout each time.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Radius, Spacing};
use gpui::{
    AnyElement, AnyView, App, ClickEvent, CursorStyle, ElementId, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Pixels, RenderOnce, SharedString, Window, div, prelude::*, px,
};
use smallvec::SmallVec;

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::{h_flex, v_flex};
use crate::theme::TextSize;
use crate::traits::{
    ClickHandler, Clickable, Disableable, HoverHandler, MouseDownHandler, ToggleState, Toggleable,
    TooltipBuilder,
};

/// Vertical density of a [`ListItem`].
///
/// `Sparse` matches engram's pre-Phase-8 default - comfortable padding,
/// suitable for sidebars and settings panes. `Dense` and `ExtraDense` shrink
/// the row to fit tree views, command palettes, and other places that need
/// to pack many items into a fixed height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ListItemSpacing {
    /// 4px vertical padding (engram default; matches the original look).
    #[default]
    Sparse,
    /// 2px vertical padding.
    Dense,
    /// 0px vertical padding.
    ExtraDense,
}

/// Controls when a [`ListItem`]'s end slot is shown.
///
/// `Always` is the default and matches the original behavior. `OnHover`
/// hides the slot until the mouse enters the row - a common pattern for
/// inline action affordances (delete buttons, secondary menus) that should
/// not draw the eye on every row of a long list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EndSlotVisibility {
    /// End slot is always painted.
    #[default]
    Always,
    /// End slot is only painted while the row is hovered.
    OnHover,
}

/// Group name attached to the inner row so [`EndSlotVisibility::OnHover`]
/// can target it via `group_hover`. Multiple list items in the same parent
/// share the name on purpose - `group_hover` only walks up to the nearest
/// ancestor with the matching group, so each row is self-contained.
const HOVER_GROUP: &str = "engram_list_item";

/// A single row inside a [`List`]. Supports start / end slots, selection
/// state, disabled state, click handlers, tree-style indentation, and
/// right-click / hover callbacks.
#[derive(IntoElement)]
#[must_use = "ListItem does nothing unless rendered"]
pub struct ListItem {
    id: ElementId,
    disabled: bool,
    selected: bool,
    spacing: ListItemSpacing,
    indent_level: usize,
    indent_step_size: Pixels,
    inset: bool,
    outlined: bool,
    rounded: bool,
    end_slot_visibility: EndSlotVisibility,
    start_slot: Option<AnyElement>,
    end_slot: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
    cursor_style: CursorStyle,
    on_click: Option<ClickHandler>,
    on_hover: Option<HoverHandler>,
    on_secondary_mouse_down: Option<MouseDownHandler>,
    tooltip: Option<TooltipBuilder>,
}

impl ListItem {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
            selected: false,
            spacing: ListItemSpacing::default(),
            indent_level: 0,
            indent_step_size: px(12.0),
            inset: false,
            outlined: false,
            // Engram's pre-Phase-8 baseline always rounded the row, so the
            // default stays `true`. Pass `.rounded(false)` to opt into a
            // square hover surface.
            rounded: true,
            end_slot_visibility: EndSlotVisibility::default(),
            start_slot: None,
            end_slot: None,
            children: SmallVec::new(),
            cursor_style: CursorStyle::PointingHand,
            on_click: None,
            on_hover: None,
            on_secondary_mouse_down: None,
            tooltip: None,
        }
    }

    pub fn start_slot(mut self, slot: impl IntoElement) -> Self {
        self.start_slot = Some(slot.into_any_element());
        self
    }

    pub fn end_slot(mut self, slot: impl IntoElement) -> Self {
        self.end_slot = Some(slot.into_any_element());
        self
    }

    /// Set the row's vertical density. See [`ListItemSpacing`] for the
    /// available steps.
    pub fn spacing(mut self, spacing: ListItemSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the indentation depth in steps of [`ListItem::indent_step_size`].
    /// Used by tree views to express hierarchy.
    pub fn indent_level(mut self, indent_level: usize) -> Self {
        self.indent_level = indent_level;
        self
    }

    /// Customize the per-step indent width. Defaults to 12px.
    pub fn indent_step_size(mut self, indent_step_size: Pixels) -> Self {
        self.indent_step_size = indent_step_size;
        self
    }

    /// When `true`, the indent gutter is drawn *outside* the row's
    /// background - the hover/selected chrome shifts right with the indent
    /// instead of spanning the full container width. This is the look used
    /// by zed's tree views.
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    /// Draw a 1px border around the row. Useful for cards-as-list-items.
    pub fn outlined(mut self) -> Self {
        self.outlined = true;
        self
    }

    /// Toggle rounded corners on the row's hover/selected chrome.
    /// Defaults to `true` to preserve engram's pre-Phase-8 visual.
    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    /// Hide the end slot until the row is hovered. Common pattern for
    /// inline action buttons that should not draw attention on idle rows.
    pub fn show_end_slot_on_hover(mut self) -> Self {
        self.end_slot_visibility = EndSlotVisibility::OnHover;
        self
    }

    /// Bind a callback to mouse hover-enter / hover-leave. The boolean is
    /// `true` on enter and `false` on leave.
    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(handler));
        self
    }

    /// Bind a callback to right-click events. Useful for surfacing a
    /// context menu.
    pub fn on_secondary_mouse_down(
        mut self,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_secondary_mouse_down = Some(Rc::new(handler));
        self
    }

    /// Attach a tooltip builder. Typically used with
    /// [`Tooltip::text`](crate::components::Tooltip::text).
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }
}

impl Clickable for ListItem {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.cursor_style = cursor_style;
        self
    }
}

impl Disableable for ListItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for ListItem {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.selected = state.into().selected();
        self
    }
}

impl ParentElement for ListItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let interactive = !self.disabled;
        let indent = self.indent_level as f32 * self.indent_step_size;
        let group_name: SharedString = HOVER_GROUP.into();

        let py = match self.spacing {
            ListItemSpacing::ExtraDense => Spacing::None.pixels(),
            ListItemSpacing::Dense => Spacing::XXSmall.pixels(),
            ListItemSpacing::Sparse => Spacing::XSmall.pixels(),
        };

        let background = if self.selected {
            Some(colors.element_selected)
        } else {
            None
        };

        let end_slot_visibility = self.end_slot_visibility;
        let end_slot_group = group_name.clone();

        // Inner row carries all of the interactive chrome (background,
        // hover/active, click, tooltip, etc.). When `inset` is set, the
        // outer wrapper supplies the indent margin so the chrome shifts
        // right; otherwise the inner row supplies its own left margin and
        // the chrome spans the full container width.
        let inner = h_flex()
            .id(self.id)
            .group(group_name)
            .w_full()
            .gap(Spacing::Small.pixels())
            .px(Spacing::Medium.pixels())
            .py(py)
            .when(!self.inset && self.indent_level > 0, |this| this.ml(indent))
            .when(self.rounded, |this| this.rounded(Radius::XSmall.pixels()))
            .when(self.outlined, |this| {
                this.border_1()
                    .border_color(colors.border)
                    .overflow_hidden()
            })
            .when_some(background, |this, bg| this.bg(bg))
            .when(interactive, |this| {
                this.cursor(self.cursor_style)
                    .hover(|s| s.bg(colors.ghost_element_hover))
                    .active(|s| s.bg(colors.ghost_element_active))
            })
            .when_some(self.start_slot, |this, slot| this.child(slot))
            .child(
                h_flex()
                    .flex_grow(1.)
                    .gap(Spacing::Small.pixels())
                    .children(self.children),
            )
            .when_some(self.end_slot, |this, slot| {
                let wrapped = h_flex().flex_shrink_0().child(slot);
                this.child(match end_slot_visibility {
                    EndSlotVisibility::Always => wrapped,
                    // Mirror zed's `VisibleOnHover` trait inline: start
                    // invisible, become visible while the parent group is
                    // hovered.
                    EndSlotVisibility::OnHover => wrapped
                        .invisible()
                        .group_hover(end_slot_group, |s| s.visible()),
                })
            })
            .when_some(
                (!self.disabled).then_some(self.on_click).flatten(),
                |this, handler| this.on_click(move |event, window, cx| handler(event, window, cx)),
            )
            .when_some(self.tooltip, |this, builder| {
                this.tooltip(move |window, cx| builder(window, cx))
            })
            .when_some(self.on_hover, |this, handler| {
                this.on_hover(move |state, window, cx| handler(state, window, cx))
            })
            .when_some(self.on_secondary_mouse_down, |this, handler| {
                this.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    handler(event, window, cx)
                })
            });

        div()
            .when(self.inset && self.indent_level > 0, |this| this.ml(indent))
            .child(inner)
    }
}

/// A vertical container for [`ListItem`]s with an optional header label and a
/// message displayed when the list has no children.
#[derive(IntoElement)]
#[must_use = "List does nothing unless rendered"]
pub struct List {
    header: Option<SharedString>,
    empty_message: SharedString,
    children: SmallVec<[AnyElement; 2]>,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    pub fn new() -> Self {
        Self {
            header: None,
            empty_message: "No items".into(),
            children: SmallVec::new(),
        }
    }

    pub fn header(mut self, header: impl Into<SharedString>) -> Self {
        self.header = Some(header.into());
        self
    }

    pub fn empty_message(mut self, message: impl Into<SharedString>) -> Self {
        self.empty_message = message.into();
        self
    }
}

impl ParentElement for List {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for List {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let is_empty = self.children.is_empty();

        v_flex()
            .w_full()
            .gap(Spacing::XXSmall.pixels())
            .when_some(self.header, |this, header| {
                this.child(
                    div()
                        .px(Spacing::Medium.pixels())
                        .py(Spacing::XSmall.pixels())
                        .child(Label::new(header).size(TextSize::Small).color(Color::Muted)),
                )
            })
            .map(|this| {
                if is_empty {
                    this.child(
                        div()
                            .px(Spacing::Medium.pixels())
                            .py(Spacing::Small.pixels())
                            .child(
                                Label::new(self.empty_message)
                                    .size(TextSize::Small)
                                    .color(Color::Muted),
                            ),
                    )
                } else {
                    this.children(self.children)
                }
            })
    }
}
