//! Disclosure - a chevron toggle for expandable sections / tree rows.

use std::rc::Rc;

use crate::theme::Color;
use gpui::{App, ClickEvent, CursorStyle, ElementId, IntoElement, RenderOnce, Window, prelude::*};

use crate::components::button::{ButtonCommon, ButtonSize, ButtonStyle, IconButton};
use crate::components::icon::{IconName, IconSize};
use crate::traits::{ClickHandler, Clickable, Disableable, ToggleState, Toggleable};

#[derive(IntoElement)]
#[must_use = "Disclosure does nothing unless rendered"]
pub struct Disclosure {
    id: ElementId,
    is_open: bool,
    selected: bool,
    disabled: bool,
    opened_icon: IconName,
    closed_icon: IconName,
    cursor_style: CursorStyle,
    on_toggle: Option<ClickHandler>,
}

impl Disclosure {
    pub fn new(id: impl Into<ElementId>, is_open: bool) -> Self {
        Self {
            id: id.into(),
            is_open,
            selected: false,
            disabled: false,
            opened_icon: IconName::ChevronDown,
            closed_icon: IconName::ChevronRight,
            cursor_style: CursorStyle::PointingHand,
            on_toggle: None,
        }
    }

    pub fn opened_icon(mut self, icon: IconName) -> Self {
        self.opened_icon = icon;
        self
    }

    pub fn closed_icon(mut self, icon: IconName) -> Self {
        self.closed_icon = icon;
        self
    }
}

impl Disableable for Disclosure {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for Disclosure {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.selected = state.into().selected();
        self
    }
}

impl Clickable for Disclosure {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.cursor_style = cursor_style;
        self
    }
}

impl RenderOnce for Disclosure {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon = if self.is_open {
            self.opened_icon
        } else {
            self.closed_icon
        };

        IconButton::new(self.id, icon)
            .style(ButtonStyle::Subtle)
            .size(ButtonSize::Compact)
            .icon_color(Color::Muted)
            .icon_size(IconSize::Small)
            .disabled(self.disabled)
            .toggle_state(self.selected)
            .cursor_style(self.cursor_style)
            .when_some(self.on_toggle, |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
    }
}
