//! Shell command view for running shell commands directly.

pub mod config;

use gpui::{div, prelude::*, px, rgba, AnyElement, App};
use ui::{font_size, radius, spacing, ActiveTheme, Color, Label, LabelCommon, LabelSize};

use self::config::ShellConfig;
use crate::launcher::view::{render_footer_hints, LauncherView, ViewContext};

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
        if vx.query.trim().is_empty() {
            0
        } else {
            1
        }
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

    fn render_content(&self, vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        let theme = cx.theme();
        let query = vx.query.trim();
        let has_command = !query.is_empty();

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
                        .child(
                            div()
                                .w_full()
                                .p(px(spacing::SM))
                                .bg(rgba(0x00000066))
                                .rounded(px(radius::SM))
                                .font_family("monospace")
                                .text_size(px(font_size::BASE))
                                .child(if has_command {
                                    Label::new(query.to_string()).color(Color::Default)
                                } else {
                                    Label::new("Type a command to execute...")
                                        .color(Color::Placeholder)
                                }),
                        ),
                )
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
        let actions = if vx.query.trim().is_empty() {
            vec![("Close", "Esc")]
        } else {
            vec![("Run", "Enter"), ("Close", "Esc")]
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
