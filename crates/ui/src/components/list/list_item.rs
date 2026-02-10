use gpui::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, RenderOnce, Window, prelude::*, px,
};

use crate::{ActiveTheme, h_flex, spacing};

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Vertical spacing for list items.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum ListItemSpacing {
    #[default]
    Dense,
    ExtraDense,
    Sparse,
}

/// A list item component with optional start/end slots, selection state,
/// and click handling.
///
/// Adapted from Zed's `ListItem`, simplified for the local theme system.
#[derive(IntoElement)]
pub struct ListItem {
    id: ElementId,
    disabled: bool,
    selected: bool,
    spacing: ListItemSpacing,
    start_slot: Option<AnyElement>,
    end_slot: Option<AnyElement>,
    on_click: Option<ClickHandler>,
    children: Vec<AnyElement>,
    selectable: bool,
}

impl ListItem {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
            selected: false,
            spacing: ListItemSpacing::Dense,
            start_slot: None,
            end_slot: None,
            on_click: None,
            children: Vec::new(),
            selectable: true,
        }
    }

    pub fn spacing(mut self, spacing: ListItemSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    pub fn start_slot<E: IntoElement>(mut self, start_slot: impl Into<Option<E>>) -> Self {
        self.start_slot = start_slot.into().map(IntoElement::into_any_element);
        self
    }

    pub fn end_slot<E: IntoElement>(mut self, end_slot: impl Into<Option<E>>) -> Self {
        self.end_slot = end_slot.into().map(IntoElement::into_any_element);
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
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
        let theme = cx.theme();

        let selected_bg = theme.accent.selection;
        let hover_bg = theme.interactive.hover;
        let active_bg = theme.interactive.active;

        h_flex()
            .id(self.id)
            .w_full()
            .relative()
            .when(!self.disabled && self.selectable, move |this| {
                this.hover(move |s| s.bg(hover_bg))
                    .active(move |s| s.bg(active_bg))
                    .when(self.selected, move |this| this.bg(selected_bg))
            })
            .when_some(
                self.on_click.filter(|_| !self.disabled),
                |this, on_click| this.cursor_pointer().on_click(on_click),
            )
            .child(
                h_flex()
                    .w_full()
                    .relative()
                    .gap(px(spacing::SM))
                    .px(px(spacing::SM))
                    .map(|this| match self.spacing {
                        ListItemSpacing::Dense => this,
                        ListItemSpacing::ExtraDense => this.py(px(0.)),
                        ListItemSpacing::Sparse => this.py(px(spacing::XS)),
                    })
                    .child(
                        h_flex()
                            .flex_grow()
                            .flex_shrink_0()
                            .gap(px(spacing::SM))
                            .overflow_hidden()
                            .children(self.start_slot)
                            .children(self.children),
                    )
                    .when_some(self.end_slot, |this, end_slot| {
                        this.justify_between()
                            .child(h_flex().flex_shrink().overflow_hidden().child(end_slot))
                    }),
            )
    }
}
