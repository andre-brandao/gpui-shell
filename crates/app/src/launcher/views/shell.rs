//! Shell view — execute commands in a terminal emulator.

use gpui::{AnyElement, App, Context, EventEmitter};
use ui::{ActiveTheme, prelude::*};

use crate::launcher::view::{FooterAction, LauncherView, ViewEvent};

pub struct ShellView {
    query: String,
}

impl EventEmitter<ViewEvent> for ShellView {}

impl ShellView {
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }
}

impl LauncherView for ShellView {
    fn id(&self) -> &'static str {
        "shell"
    }

    fn prefix(&self) -> &'static str {
        "$"
    }

    fn name(&self) -> &'static str {
        "Shell"
    }

    fn icon(&self) -> IconName {
        IconName::Terminal
    }

    fn description(&self) -> &'static str {
        "Run shell commands in terminal"
    }

    fn match_count(&self) -> usize {
        if self.query.trim().is_empty() { 0 } else { 1 }
    }

    fn set_query(&mut self, query: &str, _cx: &mut Context<Self>) {
        self.query = query.to_string();
    }

    fn render_item(&self, _index: usize, _selected: bool, _cx: &App) -> AnyElement {
        gpui::Empty.into_any_element()
    }

    fn render_content(&self, cx: &App) -> Option<AnyElement> {
        let colors = cx.theme().colors();
        let has_command = !self.query.trim().is_empty();

        Some(
            div()
                .w_full()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(12.))
                // Command preview card
                .child(
                    div()
                        .w_full()
                        .p(px(12.))
                        .bg(colors.surface_background)
                        .rounded(px(8.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        // Header row
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            Label::new("\u{f120}")
                                                .size(LabelSize::Large)
                                                .color(Color::Default),
                                        )
                                        .child(
                                            Label::new("Terminal")
                                                .size(LabelSize::Default)
                                                .color(Color::Default),
                                        )
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(colors.element_background)
                                                .child(
                                                    Label::new("$")
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Muted),
                                                ),
                                        ),
                                )
                                .when(has_command, |el| {
                                    el.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .px(px(8.))
                                            .py(px(4.))
                                            .rounded(px(6.))
                                            .bg(colors.ghost_element_selected)
                                            .child(Label::new("Run").size(LabelSize::Small))
                                            .child(
                                                div()
                                                    .px(px(4.))
                                                    .py(px(2.))
                                                    .rounded(px(3.))
                                                    .bg(colors.element_background)
                                                    .child(
                                                        Label::new("Enter")
                                                            .size(LabelSize::XSmall)
                                                            .color(Color::Muted),
                                                    ),
                                            ),
                                    )
                                }),
                        )
                        // Command display
                        .child(
                            div()
                                .w_full()
                                .p(px(8.))
                                .bg(colors.editor_background)
                                .rounded(px(6.))
                                .font_family("monospace")
                                .text_size(px(14.))
                                .child(if has_command {
                                    Label::new(self.query.trim().to_string()).color(Color::Default)
                                } else {
                                    Label::new("Type a command to execute...")
                                        .color(Color::Placeholder)
                                }),
                        ),
                )
                // Tips
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            Label::new("TIPS")
                                .size(LabelSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(
                            Label::new("\u{2022} Commands run in your default terminal emulator")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new(
                                "\u{2022} Interactive commands and output are fully supported",
                            )
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                        )
                        .child(
                            Label::new("\u{2022} Example: $htop, $vim ~/.config, $cargo build")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                )
                .into_any_element(),
        )
    }

    fn confirm(&mut self, _index: usize, cx: &mut Context<Self>) {
        let command = self.query.trim();
        if command.is_empty() {
            return;
        }
        run_in_terminal(command);
        cx.emit(ViewEvent::Close);
    }

    fn footer_actions(&self) -> Vec<FooterAction> {
        if self.query.trim().is_empty() {
            vec![FooterAction {
                label: "Close",
                key: "Esc",
            }]
        } else {
            vec![
                FooterAction {
                    label: "Run",
                    key: "Enter",
                },
                FooterAction {
                    label: "Close",
                    key: "Esc",
                },
            ]
        }
    }
}

fn run_in_terminal(command: &str) {
    let command = command.to_string();
    std::thread::spawn(move || {
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
