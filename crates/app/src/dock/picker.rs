//! Picker panel for pinning apps that aren't currently running.

use gpui::{Context, FocusHandle, Focusable, MouseButton, Render, Window, div, prelude::*, px};
use ui::{ActiveTheme, InputBuffer, Radius, Spacing, render_input_line};

use super::item::desktop_file_id;
use super::toggle_pin;
use crate::config::State;
use crate::keybinds::{
    Backspace, Cancel, CursorLeft, CursorRight, DeleteWordBack, SelectAll, SelectLeft, SelectRight,
    SelectWordLeft, SelectWordRight, WordLeft, WordRight,
};
use crate::state::AppState;

pub(super) const DOCK_APP_PICKER_HEIGHT: f32 = 280.0;
const INPUT_LINE_HEIGHT: f32 = 14.0;
const RESULT_ROW_HEIGHT: f32 = 32.0;
const MAX_RESULTS: usize =
    ((DOCK_APP_PICKER_HEIGHT - 2.0 * Spacing::Medium.value() - INPUT_LINE_HEIGHT)
        / (RESULT_ROW_HEIGHT + Spacing::XSmall.value())) as usize;

pub(super) struct DockAppPicker {
    input: InputBuffer,
    focus_handle: FocusHandle,
    needs_initial_focus: bool,
}

impl DockAppPicker {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            input: InputBuffer::default(),
            focus_handle: cx.focus_handle(),
            needs_initial_focus: true,
        }
    }

    fn matching_unpinned_apps(&self, cx: &gpui::App) -> Vec<services::Application> {
        let pinned = State::pinned(cx);
        AppState::applications(cx)
            .search(self.input.text())
            .into_iter()
            .filter(|app| !pinned.contains(&desktop_file_id(app)))
            .take(MAX_RESULTS)
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
        if self.needs_initial_focus {
            self.focus_handle.focus(window, cx);
            self.needs_initial_focus = false;
        }

        let theme = cx.theme().clone();
        let apps = self.matching_unpinned_apps(cx);

        let entries = apps.into_iter().map(|app| {
            let id = desktop_file_id(&app);
            let label = app.name.clone();
            div()
                .id(gpui::SharedString::from(id.clone()))
                .px(Spacing::Large.pixels())
                .py(Spacing::Medium.pixels())
                .min_h(px(RESULT_ROW_HEIGHT))
                .rounded(Radius::Small.pixels())
                .cursor_pointer()
                .text_color(theme.colors.text)
                .hover(move |style| style.bg(theme.colors.element_hover))
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
            .on_action(cx.listener(|_this, _: &Cancel, window, _cx| {
                window.remove_window();
                crate::panel::forget_panel("dock-app-picker");
            }))
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
            .gap(Spacing::XSmall.pixels())
            .p(Spacing::Medium.pixels())
            .rounded(Radius::Medium.pixels())
            .bg(theme.colors.background)
            .text_color(theme.colors.text)
            .border_1()
            .border_color(theme.colors.border_variant)
            .child(render_input_line(&self.input, "Search apps to pin...", cx))
            .children(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::{DOCK_APP_PICKER_HEIGHT, INPUT_LINE_HEIGHT, MAX_RESULTS, RESULT_ROW_HEIGHT};
    use ui::Spacing;

    fn picker_content_height(result_count: usize) -> f32 {
        2.0 * Spacing::Medium.value()
            + INPUT_LINE_HEIGHT
            + result_count as f32 * (RESULT_ROW_HEIGHT + Spacing::XSmall.value())
    }

    #[test]
    fn max_results_fit_the_fixed_picker_panel() {
        assert_eq!(MAX_RESULTS, 6);
        assert!(picker_content_height(MAX_RESULTS) <= DOCK_APP_PICKER_HEIGHT);
        assert!(picker_content_height(MAX_RESULTS + 1) > DOCK_APP_PICKER_HEIGHT);
    }
}
