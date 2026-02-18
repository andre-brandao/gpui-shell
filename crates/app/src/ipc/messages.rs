use gpui::App;

use crate::args::Args;
use crate::launcher;

const LAUNCHER_PREFIX: &str = "ipc:launcher:";

#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub id: u64,
    pub command: IpcCommand,
}

#[derive(Debug, Clone)]
pub enum IpcCommand {
    LauncherToggle { input: Option<String> },
}

pub fn command_for_secondary(args: &Args) -> IpcCommand {
    IpcCommand::LauncherToggle {
        input: args.input.clone(),
    }
}

pub fn encode_command(command: &IpcCommand) -> String {
    match command {
        IpcCommand::LauncherToggle { input } => {
            format!("{}{}", LAUNCHER_PREFIX, input.as_deref().unwrap_or(""))
        }
    }
}

pub fn decode_command(payload: &str) -> IpcCommand {
    if let Some(rest) = payload.strip_prefix(LAUNCHER_PREFIX) {
        return IpcCommand::LauncherToggle {
            input: if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            },
        };
    }

    IpcCommand::LauncherToggle {
        input: if payload.is_empty() {
            None
        } else {
            Some(payload.to_string())
        },
    }
}

pub(super) fn handle_message(message: IpcMessage, cx: &mut App) {
    match message.command {
        IpcCommand::LauncherToggle { input } => {
            tracing::info!(
                "Processing launcher request: id={}, input={:?}",
                message.id,
                input
            );
            launcher::toggle(input, cx);
        }
    }
}
