//! Shell command view for running shell commands directly.

use crate::launcher::view::{LauncherView, ViewContext};
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px, rgba};
use ui::{bg, font_size, interactive, radius, spacing, text};

/// Shell view - executes shell commands directly.
pub struct ShellView;

impl LauncherView for ShellView {
    fn prefix(&self) -> &'static str {
        "$"
    }

    fn name(&self) -> &'static str {
        "Shell"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Run shell commands"
    }

    fn render(&self, vx: &ViewContext, _cx: &App) -> (AnyElement, usize) {
        let query = vx.query.trim();
        let has_command = !query.is_empty();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(spacing::MD))
            .p(px(spacing::MD))
            // Command preview
            .child(
                div()
                    .w_full()
                    .p(px(spacing::MD))
                    .bg(bg::secondary())
                    .rounded(px(radius::MD))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::SM))
                    // Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(font_size::LG))
                                    .text_color(text::muted())
                                    .child(""),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(text::muted())
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("COMMAND"),
                            ),
                    )
                    // Command display
                    .child(
                        div()
                            .w_full()
                            .p(px(spacing::SM))
                            .bg(rgba(0x00000066))
                            .rounded(px(radius::SM))
                            .font_family("monospace")
                            .text_size(px(font_size::BASE))
                            .text_color(if has_command {
                                text::primary()
                            } else {
                                text::placeholder()
                            })
                            .child(if has_command {
                                query.to_string()
                            } else {
                                "Type a command to execute...".to_string()
                            }),
                    ),
            )
            // Run button (visual indicator)
            .when(has_command, |el| {
                el.child(
                    div()
                        .id("run-command")
                        .w_full()
                        .h(px(48.))
                        .px(px(spacing::MD))
                        .rounded(px(radius::MD))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(spacing::SM))
                        .bg(interactive::default())
                        .hover(|s| s.bg(interactive::hover()))
                        .when(vx.selected_index == 0, |el| el.bg(rgba(0x3b82f6ff)))
                        .child(div().text_size(px(font_size::LG)).child(""))
                        .child(
                            div()
                                .text_size(px(font_size::BASE))
                                .font_weight(FontWeight::MEDIUM)
                                .child("Run Command"),
                        )
                        .child(
                            div()
                                .px(px(spacing::SM))
                                .py(px(2.))
                                .rounded(px(radius::SM))
                                .bg(rgba(0x00000033))
                                .text_size(px(font_size::XS))
                                .child("Enter"),
                        ),
                )
            })
            // Help text
            .child(
                div()
                    .w_full()
                    .pt(px(spacing::MD))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(text::disabled())
                            .font_weight(FontWeight::MEDIUM)
                            .child("TIPS"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Commands run in a detached shell process"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child(
                                "• Output is not captured (use terminal for interactive commands)",
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Example: $firefox, $code ., $systemctl restart ..."),
                    ),
            )
            .into_any_element();

        // 1 selectable item when there's a command, 0 otherwise
        let count = if has_command { 1 } else { 0 };
        (element, count)
    }

    fn on_select(&self, _index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let command = vx.query.trim();
        if command.is_empty() {
            return false;
        }

        run_shell_command(command);
        true // Close launcher
    }

    fn footer_actions(&self, vx: &ViewContext) -> Vec<(&'static str, &'static str)> {
        if vx.query.trim().is_empty() {
            vec![("Close", "Esc")]
        } else {
            vec![("Run", "Enter"), ("Close", "Esc")]
        }
    }
}

/// Run a shell command in a detached process.
fn run_shell_command(command: &str) {
    let command = command.to_string();
    std::thread::spawn(move || {
        let _ = std::process::Command::new("sh")
            .args(["-c", &command])
            .spawn();
    });
}
