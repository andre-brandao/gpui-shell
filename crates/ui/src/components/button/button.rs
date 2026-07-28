//! [`Button`] - text button with an optional leading icon.
//!
//! Composes [`ButtonLike`] for chrome and forwards every builder method
//! through [`ButtonCommon`] / [`Clickable`] / [`Disableable`] /
//! [`Toggleable`] / [`SelectableButton`]. The render fn just decides which
//! label/icon to draw and applies the size-derived padding before handing
//! the assembled child off to ButtonLike.

use crate::theme::{Color, Spacing};
use gpui::{
    AnyView, App, ClickEvent, CursorStyle, ElementId, FocusHandle, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, prelude::FluentBuilder,
};

use crate::components::button::button_like::{
    ButtonCommon, ButtonLike, ButtonSize, ButtonStyle, SelectableButton,
};
use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;
use crate::traits::{Clickable, Disableable, ToggleState, Toggleable};

/// A text button with an optional leading icon.
#[derive(IntoElement)]
#[must_use = "Button does nothing unless rendered"]
pub struct Button {
    base: ButtonLike,
    label: SharedString,
    icon: Option<IconName>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            base: ButtonLike::new(id),
            label: label.into(),
            icon: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }
}

impl Toggleable for Button {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.base = self.base.toggle_state(state);
        self
    }
}

impl SelectableButton for Button {
    fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.selected_style(style);
        self
    }
}

impl Clickable for Button {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.base = self.base.on_click(handler);
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.base = self.base.cursor_style(cursor_style);
        self
    }
}

impl ButtonCommon for Button {
    fn id(&self) -> &ElementId {
        self.base.id()
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.style(style);
        self
    }

    fn size(mut self, size: ButtonSize) -> Self {
        self.base = self.base.size(size);
        self
    }

    fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.base = self.base.tooltip(tooltip);
        self
    }

    fn tab_index(mut self, tab_index: isize) -> Self {
        self.base = self.base.tab_index(tab_index);
        self
    }

    fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.base = self.base.track_focus(focus_handle);
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_disabled = self.base.disabled;
        let is_selected = self.base.selected;
        let size = self.base.size;
        let style = self.base.style;
        let (pad_x, pad_y) = padding_for(size);
        let text_size = text_size_for(size);
        let icon_size = icon_size_for(size);

        let label_color = if is_disabled {
            Color::Disabled
        } else if is_selected {
            Color::Selected
        } else {
            style.label_color_override(cx).unwrap_or(Color::Default)
        };

        self.base.padding(pad_x.pixels(), pad_y.pixels()).child(
            h_flex()
                .gap(Spacing::XSmall.pixels())
                .when_some(self.icon, |this, icon| {
                    this.child(Icon::new(icon).size(icon_size).color(label_color))
                })
                .child(Label::new(self.label).size(text_size).color(label_color)),
        )
    }
}

fn padding_for(size: ButtonSize) -> (Spacing, Spacing) {
    match size {
        ButtonSize::Compact => (Spacing::XSmall, Spacing::XXSmall),
        ButtonSize::Default => (Spacing::Small, Spacing::XXSmall),
        ButtonSize::Large => (Spacing::Large, Spacing::Small),
    }
}

fn text_size_for(size: ButtonSize) -> TextSize {
    match size {
        ButtonSize::Compact => TextSize::XSmall,
        ButtonSize::Default => TextSize::Small,
        ButtonSize::Large => TextSize::Default,
    }
}

fn icon_size_for(size: ButtonSize) -> IconSize {
    match size {
        ButtonSize::Compact => IconSize::XSmall,
        ButtonSize::Default => IconSize::Small,
        ButtonSize::Large => IconSize::Medium,
    }
}
