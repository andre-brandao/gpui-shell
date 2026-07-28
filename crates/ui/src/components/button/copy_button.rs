//! [`CopyButton`] - an icon button that copies a string to the clipboard.
//!
//! Shows a copy icon that transitions to a check icon for 2 seconds after
//! the user clicks it. Uses `window.use_keyed_state` to persist the
//! "just copied" state across re-renders without needing a parent entity.

use std::time::Duration;

use crate::theme::Color;
use std::time::Instant;

use gpui::{
    App, ClipboardItem, Context, ElementId, Entity, IntoElement, RenderOnce, SharedString, Window,
};

use crate::components::button::button_like::{ButtonCommon, ButtonStyle};
use crate::components::button::icon_button::IconButton;
use crate::components::icon::{IconName, IconSize};
use crate::components::tooltip::Tooltip;
use crate::traits::{Clickable, Disableable};

const COPIED_STATE_DURATION: Duration = Duration::from_secs(2);

struct CopyButtonState {
    copied_at: Option<Instant>,
}

impl CopyButtonState {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { copied_at: None }
    }

    fn is_copied(&self) -> bool {
        self.copied_at
            .map(|t| t.elapsed() < COPIED_STATE_DURATION)
            .unwrap_or(false)
    }

    fn mark_copied(&mut self) {
        self.copied_at = Some(Instant::now());
    }
}

/// An icon button that copies a string to the clipboard on click.
#[derive(IntoElement)]
#[must_use = "CopyButton does nothing unless rendered"]
pub struct CopyButton {
    id: ElementId,
    message: SharedString,
    icon_size: IconSize,
    disabled: bool,
    tooltip_label: SharedString,
}

impl CopyButton {
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            icon_size: IconSize::Small,
            disabled: false,
            tooltip_label: "Copy".into(),
        }
    }

    pub fn icon_size(mut self, icon_size: IconSize) -> Self {
        self.icon_size = icon_size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip_label(mut self, tooltip_label: impl Into<SharedString>) -> Self {
        self.tooltip_label = tooltip_label.into();
        self
    }
}

impl RenderOnce for CopyButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let message = self.message;

        let state: Entity<CopyButtonState> =
            window.use_keyed_state(id.clone(), cx, CopyButtonState::new);
        let is_copied = state.read(cx).is_copied();

        let (icon, color, tooltip) = if is_copied {
            (IconName::Check, Color::Success, "Copied!".into())
        } else {
            (IconName::Copy, Color::Muted, self.tooltip_label)
        };

        IconButton::new(id, icon)
            .icon_color(color)
            .icon_size(self.icon_size)
            .style(ButtonStyle::Subtle)
            .disabled(self.disabled)
            .tooltip(Tooltip::text(tooltip))
            .on_click(move |_, _window: &mut Window, cx: &mut App| {
                state.update(cx, |state, _cx| {
                    state.mark_copied();
                });

                cx.stop_propagation();
                cx.write_to_clipboard(ClipboardItem::new_string(message.to_string()));

                let state_id = state.entity_id();
                cx.spawn(async move |cx: &mut gpui::AsyncApp| {
                    cx.background_executor().timer(COPIED_STATE_DURATION).await;
                    cx.update(|cx: &mut App| {
                        cx.notify(state_id);
                    })
                })
                .detach();
            })
    }
}
