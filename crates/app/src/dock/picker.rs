//! Picker panel for pinning apps that aren't currently running.

use gpui::{
    Context, FocusHandle, Focusable, MouseButton, Render, Subscription, Window, div, prelude::*, px,
};
use ui::{ActiveTheme, InputBuffer, radius, render_input_line, spacing};

use super::item::desktop_file_id;
use super::toggle_pin;
use crate::config::ActiveConfig;
use crate::keybinds::{
    Backspace, CursorLeft, CursorRight, DeleteWordBack, SelectAll, SelectLeft, SelectRight,
    SelectWordLeft, SelectWordRight, WordLeft, WordRight,
};
use crate::state::AppState;

pub(super) struct DockAppPicker {
    input: InputBuffer,
    focus_handle: FocusHandle,
    focus_out_subscription: Option<Subscription>,
}

impl DockAppPicker {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            input: InputBuffer::default(),
            focus_handle: cx.focus_handle(),
            focus_out_subscription: None,
        }
    }

    fn matching_unpinned_apps(&self, cx: &gpui::App) -> Vec<services::Application> {
        let pinned = &cx.config().dock.pinned;
        AppState::applications(cx)
            .search(self.input.text())
            .into_iter()
            .filter(|app| !pinned.contains(&desktop_file_id(app)))
            .take(8)
            .cloned()
            .collect()
    }
}

impl Focusable for DockAppPicker {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DockAppPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_out_subscription.is_none() {
            self.focus_out_subscription =
                Some(
                    cx.on_focus_out(&self.focus_handle, window, move |_, _, window, _| {
                        window.remove_window();
                        crate::panel::forget_panel("dock-app-picker");
                    }),
                );
        }
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        let theme = cx.theme().clone();
        let apps = self.matching_unpinned_apps(cx);

        let entries = apps.into_iter().map(|app| {
            let id = desktop_file_id(&app);
            let label = app.name.clone();
            div()
                .id(gpui::SharedString::from(id.clone()))
                .px(px(spacing::MD))
                .py(px(spacing::SM))
                .rounded(px(radius::SM))
                .cursor_pointer()
                .text_color(theme.text.primary)
                .hover(move |style| style.bg(theme.interactive.hover))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_this, _event, window, cx| {
                        toggle_pin(&id, cx);
                        window.remove_window();
                        crate::panel::forget_panel("dock-app-picker");
                    }),
                )
                .child(label)
        });

        div()
            .id("dock-app-picker")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|this, _: &Backspace, _window, cx| {
                this.input.backspace();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DeleteWordBack, _window, cx| {
                this.input.delete_word_back();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorLeft, _window, cx| {
                this.input.move_left(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorRight, _window, cx| {
                this.input.move_right(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &WordLeft, _window, cx| {
                this.input.move_word_left(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &WordRight, _window, cx| {
                this.input.move_word_right(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectWordLeft, _window, cx| {
                this.input.move_word_left(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectWordRight, _window, cx| {
                this.input.move_word_right(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectLeft, _window, cx| {
                this.input.move_left(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _window, cx| {
                this.input.move_right(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _window, cx| {
                this.input.select_all();
                cx.notify();
            }))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, _window, cx| {
                    if event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
                        return;
                    }

                    let input = event.keystroke.key_char.as_ref().cloned().or_else(|| {
                        let key = event.keystroke.key.as_str();
                        (key.chars().count() == 1).then(|| key.to_string())
                    });
                    let Some(input) = input else {
                        return;
                    };
                    if input.chars().any(char::is_control) {
                        return;
                    }

                    this.input.insert_str(&input);
                    cx.notify();
                }),
            )
            .flex()
            .flex_col()
            .gap(px(spacing::XS))
            .p(px(spacing::SM))
            .rounded(px(radius::MD))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.subtle)
            .child(render_input_line(&self.input, "Search apps to pin...", cx))
            .children(entries)
    }
}
