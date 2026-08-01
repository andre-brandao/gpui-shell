//! [`IconButton`] - square icon-only button.

use crate::theme::Color;
use gpui::{
    AnyView, App, ClickEvent, CursorStyle, ElementId, FocusHandle, IntoElement, ParentElement,
    Pixels, RenderOnce, Window, px,
};

use crate::components::button::button_like::{
    ButtonCommon, ButtonLike, ButtonSize, ButtonStyle, SelectableButton,
};
use crate::components::icon::{Icon, IconSize, IconSource};
use crate::traits::{Clickable, Disableable, ToggleState, Toggleable};

/// A square icon-only button.
#[derive(IntoElement)]
#[must_use = "IconButton does nothing unless rendered"]
pub struct IconButton {
    base: ButtonLike,
    icon: IconSource,
    icon_color: Option<Color>,
    icon_size: Option<IconSize>,
}

impl IconButton {
    /// Takes an [`IconSource`] rather than only an [`IconName`] so a
    /// user-configured icon (a path from `config.toml`) is still a button
    /// rather than a hand-rolled clickable div.
    pub fn new(id: impl Into<ElementId>, icon: impl Into<IconSource>) -> Self {
        Self {
            base: ButtonLike::new(id),
            icon: icon.into(),
            icon_color: None,
            icon_size: None,
        }
    }

    /// See [`ButtonLike::on_aux_click`].
    pub fn on_aux_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_aux_click(handler);
        self
    }

    /// Override the icon color. When unset, the color is derived from the
    /// button's disabled/selected state (mirrors zed's
    /// `IconButton::icon_color`).
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// Override the icon size. When unset, the size is derived from the
    /// button's [`ButtonSize`] (mirrors zed's `IconButton::icon_size`).
    pub fn icon_size(mut self, size: IconSize) -> Self {
        self.icon_size = Some(size);
        self
    }
}

impl Disableable for IconButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }
}

impl Toggleable for IconButton {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.base = self.base.toggle_state(state);
        self
    }
}

impl SelectableButton for IconButton {
    fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.selected_style(style);
        self
    }
}

impl Clickable for IconButton {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.base = self.base.on_click(handler);
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.base = self.base.cursor_style(cursor_style);
        self
    }
}

impl ButtonCommon for IconButton {
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

impl RenderOnce for IconButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_disabled = self.base.disabled;
        let is_selected = self.base.selected;
        let size = self.base.size;
        let style = self.base.style;
        let pad = padding_for(size);
        let icon_size = self.icon_size.unwrap_or_else(|| icon_size_for(size));

        // Disabled always wins; otherwise a caller-supplied `icon_color`
        // overrides the state-derived default.
        let icon_color = if is_disabled {
            Color::Disabled
        } else if let Some(color) = self.icon_color {
            color
        } else if is_selected {
            Color::Selected
        } else {
            style.label_color_override(cx).unwrap_or(Color::Default)
        };

        self.base
            .padding(pad, pad)
            .child(Icon::new(self.icon).size(icon_size).color(icon_color))
    }
}

/// Square padding step.
fn padding_for(size: ButtonSize) -> Pixels {
    match size {
        ButtonSize::Compact => px(2.0),
        ButtonSize::Default => px(4.0),
        ButtonSize::Large => px(6.0),
    }
}

fn icon_size_for(size: ButtonSize) -> IconSize {
    match size {
        ButtonSize::Compact => IconSize::XSmall,
        ButtonSize::Default => IconSize::Small,
        ButtonSize::Large => IconSize::Medium,
    }
}
