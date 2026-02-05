//! Shell command view for running shell commands directly.

use crate::launcher::view::{LauncherView, ViewContext};
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px, rgba};
use ui::{ActiveTheme, font_size, radius, spacing};

/// Shell view - executes shell commands in a terminal.
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
        "Run shell commands in terminal"
    }

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let theme = cx.theme();
        let query = vx.query.trim();
        let has_command = !query.is_empty();

        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_disabled = theme.text.disabled;
        let text_placeholder = theme.text.placeholder;
        let bg_secondary = theme.bg.secondary;

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
                    .bg(bg_secondary)
                    .rounded(px(radius::MD))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::SM))
                    // Header with run action
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::SM))
                                    .child(
                                        div()
                                            .text_size(px(font_size::XL))
                                            .text_color(text_primary)
                                            .child(""),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(font_size::BASE))
                                            .text_color(text_primary)
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Terminal"),
                                    )
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(rgba(0x555555ff))
                                            .text_size(px(font_size::XS))
                                            .child("$"),
                                    ),
                            )
                            // Run hint (always visible, changes appearance when command exists)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::SM))
                                    .px(px(spacing::SM))
                                    .py(px(4.))
                                    .rounded(px(radius::SM))
                                    .when(has_command && vx.selected_index == 0, |el| {
                                        el.bg(rgba(0x3b82f6ff))
                                    })
                                    .when(has_command && vx.selected_index != 0, |el| {
                                        el.bg(rgba(0x333333ff))
                                    })
                                    .when(!has_command, |el| el.bg(rgba(0x00000033)))
                                    .child(
                                        div()
                                            .text_size(px(font_size::SM))
                                            .text_color(if has_command {
                                                text_primary
                                            } else {
                                                text_disabled
                                            })
                                            .child("Run"),
                                    )
                                    .child(
                                        div()
                                            .px(px(4.))
                                            .py(px(2.))
                                            .rounded(px(3.))
                                            .bg(rgba(0x00000044))
                                            .text_size(px(font_size::XS))
                                            .text_color(if has_command {
                                                text_muted
                                            } else {
                                                text_disabled
                                            })
                                            .child("Enter"),
                                    ),
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
                                text_primary
                            } else {
                                text_placeholder
                            })
                            .child(if has_command {
                                query.to_string()
                            } else {
                                "Type a command to execute...".to_string()
                            }),
                    ),
            )
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
                            .text_color(text_disabled)
                            .font_weight(FontWeight::MEDIUM)
                            .child("TIPS"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text_muted)
                            .child("• Commands run in your default terminal emulator"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text_muted)
                            .child("• Interactive commands and output are fully supported"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text_muted)
                            .child("• Example: $htop, $vim ~/.config, $cargo build"),
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

        run_in_terminal(command);
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

/// Run a command in the default terminal emulator.
fn run_in_terminal(command: &str) {
    let command = command.to_string();
    std::thread::spawn(move || {
        // Try common terminal emulators in order of preference
        let terminals: &[(&str, &[&str])] = &[
            // Modern terminals
            ("ghostty", &["-e", "sh", "-c"]),
            ("kitty", &["--", "sh", "-c"]),
            ("alacritty", &["-e", "sh", "-c"]),
            ("wezterm", &["start", "--", "sh", "-c"]),
            ("foot", &["sh", "-c"]),
            // Classic terminals
            ("gnome-terminal", &["--", "sh", "-c"]),
            ("konsole", &["-e", "sh", "-c"]),
            ("xfce4-terminal", &["-e", "sh", "-c"]),
            ("xterm", &["-e", "sh", "-c"]),
        ];

        // Build command that keeps terminal open after command finishes
        // This allows seeing output and interacting if needed
        let full_command = format!("{}; echo ''; echo 'Press Enter to close...'; read", command);

        for (terminal, args) in terminals {
            let mut cmd_args: Vec<&str> = args.to_vec();
            cmd_args.push(&full_command);

            if std::process::Command::new(terminal)
                .args(&cmd_args)
                .spawn()
                .is_ok()
            {
                return;
            }
        }

        // Fallback: try x-terminal-emulator (Debian/Ubuntu alternative system)
        let _ = std::process::Command::new("x-terminal-emulator")
            .args(["-e", "sh", "-c", &full_command])
            .spawn();
    });
}
