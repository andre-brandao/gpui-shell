//! Shell command view for running shell commands directly.

pub mod config;

use std::path::PathBuf;

use gpui::{AnyElement, App, div, prelude::*, px, rgba};
use ui::{ActiveTheme, Color, Label, LabelCommon, LabelSize, radius, spacing};

use self::config::ShellConfig;
use crate::launcher::view::{
    InputResult, LauncherView, ViewContext, ViewInput, render_footer_hints,
};

/// Shell view - executes shell commands in a terminal.
pub struct ShellView {
    prefix: String,
    terminal: String,
}

impl ShellView {
    pub fn new(config: &ShellConfig) -> Self {
        Self {
            prefix: config.prefix.clone(),
            terminal: config.terminal.clone(),
        }
    }
}

/// Check if a command name exists in PATH.
fn command_exists(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // If it contains a slash, check as a file path
    if name.contains('/') {
        let path = PathBuf::from(name);
        return path.exists() && is_executable(&path);
    }
    find_in_path(name).is_some()
}

/// Find the first matching executable in PATH.
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    for dir in path_var.split(':') {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.exists() && is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Check if a path is executable (Unix).
fn is_executable(path: &PathBuf) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Find commands in PATH that start with the given prefix.
fn find_matching_commands(prefix: &str, max: usize) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let path_var = match std::env::var("PATH") {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut matches = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for dir in path_var.split(':') {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(prefix)
                && !seen.contains(name.as_ref())
                && is_executable(&entry.path())
            {
                seen.insert(name.to_string());
                matches.push(name.to_string());
                if matches.len() >= max {
                    matches.sort();
                    return matches;
                }
            }
        }
    }

    matches.sort();
    matches
}

impl LauncherView for ShellView {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn name(&self) -> &'static str {
        "Shell"
    }

    fn icon(&self) -> &'static str {
        "󰆍"
    }

    fn description(&self) -> &'static str {
        "Run shell commands in terminal"
    }

    fn match_count(&self, vx: &ViewContext, _cx: &App) -> usize {
        if vx.query.trim().is_empty() { 0 } else { 1 }
    }

    fn render_item(
        &self,
        _index: usize,
        _selected: bool,
        _vx: &ViewContext,
        _cx: &App,
    ) -> AnyElement {
        div().into_any_element()
    }

    fn handle_input(&self, input: &ViewInput, vx: &ViewContext, _cx: &mut App) -> InputResult {
        if let ViewInput::Tab = input {
            let query = vx.query.trim();
            if query.is_empty() {
                return InputResult::Unhandled;
            }

            // Get the first word (command name) for completion
            let parts: Vec<&str> = query.splitn(2, ' ').collect();
            let cmd = parts[0];
            let rest = parts.get(1).map(|s| format!(" {}", s)).unwrap_or_default();

            let matches = find_matching_commands(cmd, 1);
            if let Some(completed) = matches.first()
                && completed != cmd
            {
                let new_query = format!("{}{}", completed, rest);
                return InputResult::Handled {
                    query: new_query,
                    close: false,
                };
            }
            InputResult::Unhandled
        } else {
            InputResult::Unhandled
        }
    }

    fn render_content(&self, vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        let theme = cx.theme();
        let query = vx.query.trim();
        let has_command = !query.is_empty();

        // Determine if the command exists in PATH
        let cmd_name = query.split_whitespace().next().unwrap_or("");
        let cmd_valid = command_exists(cmd_name);

        // Find suggestions for autocomplete
        let suggestions = if !cmd_name.is_empty() && !cmd_valid {
            find_matching_commands(cmd_name, 5)
        } else {
            Vec::new()
        };

        let bg_secondary = theme.bg.secondary;
        let interactive_default = theme.interactive.default;
        let accent_selection = theme.accent.selection;
        let interactive_hover = theme.interactive.hover;
        let icon = self.icon();
        Some(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(spacing::MD))
                .p(px(spacing::MD))
                .child(
                    div()
                        .w_full()
                        .p(px(spacing::MD))
                        .bg(bg_secondary)
                        .rounded(px(radius::MD))
                        .flex()
                        .flex_col()
                        .gap(px(spacing::SM))
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
                                            Label::new(icon)
                                                .size(LabelSize::Large)
                                                .color(Color::Default),
                                        )
                                        .child(Label::new("Terminal").size(LabelSize::Default))
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(interactive_default)
                                                .child(
                                                    Label::new("$")
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Muted),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(spacing::SM))
                                        .px(px(spacing::SM))
                                        .py(px(4.))
                                        .rounded(px(radius::SM))
                                        .when(has_command && vx.selected_index == 0, move |el| {
                                            el.bg(accent_selection)
                                        })
                                        .when(has_command && vx.selected_index != 0, move |el| {
                                            el.bg(interactive_hover)
                                        })
                                        .when(!has_command, |el| el.bg(rgba(0x00000033)))
                                        .child(if has_command {
                                            Label::new("Run").size(LabelSize::Small)
                                        } else {
                                            Label::new("Run")
                                                .size(LabelSize::Small)
                                                .color(Color::Disabled)
                                        })
                                        .child(
                                            div()
                                                .px(px(4.))
                                                .py(px(2.))
                                                .rounded(px(3.))
                                                .bg(rgba(0x00000044))
                                                .child(if has_command {
                                                    Label::new("Enter")
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Muted)
                                                } else {
                                                    Label::new("Enter")
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Disabled)
                                                }),
                                        ),
                                ),
                        )
                        // Command display with color coding
                        .child(
                            div()
                                .w_full()
                                .p(px(spacing::SM))
                                .bg(rgba(0x00000066))
                                .rounded(px(radius::SM))
                                .font_family("monospace")
                                .text_size(theme.font_sizes.base)
                                .child(if has_command {
                                    if cmd_valid {
                                        // Command exists — show in green
                                        div()
                                            .flex()
                                            .child(
                                                Label::new(cmd_name.to_string())
                                                    .color(Color::Success),
                                            )
                                            .child(Label::new(
                                                query[cmd_name.len()..].to_string(),
                                            ))
                                            .into_any_element()
                                    } else {
                                        // Command not found — default color
                                        Label::new(query.to_string())
                                            .color(Color::Default)
                                            .into_any_element()
                                    }
                                } else {
                                    Label::new("Type a command to execute...")
                                        .color(Color::Placeholder)
                                        .into_any_element()
                                }),
                        )
                        // Suggestions
                        .when(!suggestions.is_empty(), |el| {
                            el.child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        Label::new("Suggestions (Tab to complete)")
                                            .size(LabelSize::XSmall)
                                            .color(Color::Disabled),
                                    )
                                    .children(suggestions.iter().map(|s| {
                                        div()
                                            .px(px(spacing::SM))
                                            .py(px(2.))
                                            .font_family("monospace")
                                            .text_size(theme.font_sizes.sm)
                                            .child(Label::new(s.clone()).color(Color::Muted))
                                    })),
                            )
                        }),
                )
                // Tips section
                .child(
                    div()
                        .w_full()
                        .pt(px(spacing::MD))
                        .flex()
                        .flex_col()
                        .gap(px(spacing::XS))
                        .child(
                            Label::new("TIPS")
                                .size(LabelSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(
                            Label::new("• Commands run in your default terminal emulator")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new("• Interactive commands and output are fully supported")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new("• Example: $htop, $vim ~/.config, $cargo build")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                )
                .into_any_element(),
        )
    }

    fn on_select(&self, _index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let command = vx.query.trim();
        if command.is_empty() {
            return false;
        }

        run_in_terminal(command, &self.terminal);
        true
    }

    fn render_footer_bar(&self, vx: &ViewContext, cx: &App) -> AnyElement {
        let has_command = !vx.query.trim().is_empty();
        let actions = if has_command {
            vec![("Run", "Enter"), ("Complete", "Tab"), ("Close", "Esc")]
        } else {
            vec![("Close", "Esc")]
        };
        render_footer_hints(actions, cx)
    }
}

fn run_in_terminal(command: &str, preferred: &str) {
    let command = command.to_string();
    let preferred = preferred.to_string();
    std::thread::spawn(move || {
        if !preferred.is_empty() {
            let full_command =
                format!("{}; echo ''; echo 'Press Enter to close...'; read", command);
            if std::process::Command::new(&preferred)
                .args(["-e", "sh", "-c", &full_command])
                .spawn()
                .is_ok()
            {
                return;
            }
            tracing::warn!(
                "Configured terminal '{}' not found, trying defaults",
                preferred
            );
        }

        let terminals: &[(&str, &[&str])] = &[
            ("ghostty", &["-e", "sh", "-c"]),
            ("kitty", &["--", "sh", "-c"]),
            ("alacritty", &["-e", "sh", "-c"]),
            ("wezterm", &["start", "--", "sh", "-c"]),
            ("foot", &["sh", "-c"]),
            ("gnome-terminal", &["--", "sh", "-c"]),
            ("konsole", &["-e", "sh", "-c"]),
            ("xfce4-terminal", &["-e", "sh", "-c"]),
            ("xterm", &["-e", "sh", "-c"]),
        ];

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

        let _ = std::process::Command::new("x-terminal-emulator")
            .args(["-e", "sh", "-c", &full_command])
            .spawn();
    });
}
