//! Shell view: run the typed command in a terminal or in the background.

pub mod config;

use std::process::Command;

use gpui::{Context, EventEmitter, SharedString};
use ui::IconName;

use self::config::ShellConfig;
use crate::launcher::item::{Item, ItemIcon};
use crate::launcher::view::{
    Items, LauncherView, Matching, Outcome, ViewDescriptor, ViewEvent, ViewMeta,
};

const ICON: IconName = IconName::SquareTerminal;

/// Tried in order when no terminal is configured, or the configured one is
/// missing.
const TERMINALS: &[(&str, &[&str])] = &[
    ("ghostty", &["-e", "sh", "-c"]),
    ("kitty", &["--", "sh", "-c"]),
    ("alacritty", &["-e", "sh", "-c"]),
    ("wezterm", &["start", "--", "sh", "-c"]),
    ("foot", &["sh", "-c"]),
    ("gnome-terminal", &["--", "sh", "-c"]),
    ("konsole", &["-e", "sh", "-c"]),
    ("xfce4-terminal", &["-e", "sh", "-c"]),
    ("xterm", &["-e", "sh", "-c"]),
    ("x-terminal-emulator", &["-e", "sh", "-c"]),
];

pub fn descriptor(config: &ShellConfig) -> ViewDescriptor {
    let terminal = config.terminal.clone();
    ViewDescriptor::new(
        ViewMeta {
            id: "shell",
            prefix: config.prefix.clone().into(),
            name: "Shell",
            icon: ICON,
            description: "Run shell commands in terminal",
            default: false,
            in_help: true,
        },
        move |_| ShellView {
            terminal: terminal.clone(),
        },
    )
}

pub struct ShellView {
    terminal: String,
}

impl EventEmitter<ViewEvent> for ShellView {}

impl LauncherView for ShellView {
    fn matching(&self) -> Matching {
        Matching::View
    }

    fn items(&mut self, query: &str, _cx: &mut Context<Self>) -> Items {
        let command = query.trim();
        if command.is_empty() {
            return Items::Ready(Vec::new());
        }
        let terminal = if self.terminal.is_empty() {
            "your terminal"
        } else {
            &self.terminal
        };
        Items::Ready(vec![
            Item::new("run", command.to_string())
                .subtitle(format!("Run in {terminal}"))
                .icon(ItemIcon::Icon(ICON.into()))
                .action("terminal", "Run")
                .action("background", "Run in background"),
        ])
    }

    fn activate(&mut self, item: &Item, action: &str, _cx: &mut Context<Self>) -> Outcome {
        let command = item.title.to_string();
        if action == "background" {
            let mut sh = Command::new("sh");
            sh.args(["-c", &command]);
            if let Err(err) = services::spawn_reaped(sh, command.clone()) {
                tracing::warn!("Failed to run `{command}`: {err}");
            }
        } else {
            run_in_terminal(command, self.terminal.clone());
        }
        Outcome::Close
    }

    fn empty_text(&self) -> Option<SharedString> {
        Some("Type a command to run, e.g. htop or cargo build".into())
    }
}

/// Keeps the terminal open after the command exits so its output stays.
fn run_in_terminal(command: String, preferred: String) {
    let script = format!("{command}; echo ''; echo 'Press Enter to close...'; read");
    let preferred =
        (!preferred.is_empty()).then(|| (preferred.as_str(), ["-e", "sh", "-c"].as_slice()));

    for (terminal, args) in preferred.into_iter().chain(TERMINALS.iter().copied()) {
        let mut cmd = Command::new(terminal);
        cmd.args(args).arg(&script);
        if services::spawn_reaped(cmd, terminal).is_ok() {
            return;
        }
        tracing::debug!("Terminal '{terminal}' not available, trying the next");
    }
    tracing::warn!("No terminal found to run `{command}`");
}
