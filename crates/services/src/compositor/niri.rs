//! Niri compositor backend.
//!
//! This module provides integration with the Niri compositor via its IPC socket.
//! Uses the niri-ipc crate for protocol types and the EventStreamState tracker
//! for maintaining compositor state from the event stream.

use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader, Write as _},
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use futures_signals::signal::Mutable;
use itertools::Itertools;
use niri_ipc::{
    Action, Event, Reply, Request, WorkspaceReferenceArg,
    state::{EventStreamState, EventStreamStatePart},
};
use tracing::{debug, info};

use crate::lifecycle::RunToken;

use super::types::{ActiveWindow, CompositorCommand, CompositorState, Monitor, Window, Workspace};

/// Check if Niri is available (running).
pub fn is_available() -> bool {
    env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .is_some()
}

/// Execute a compositor command synchronously via niri IPC.
pub fn execute_command(cmd: CompositorCommand) -> Result<()> {
    let action = match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            let id = u64::try_from(id)
                .map_err(|_| anyhow!("Workspace ID {} is out of range for Niri backend", id))?;
            Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Id(id),
            }
        }
        CompositorCommand::FocusSpecialWorkspace(_) => {
            anyhow::bail!("Special workspaces not supported in Niri backend");
        }
        CompositorCommand::ToggleSpecialWorkspace(_) => {
            anyhow::bail!("Special workspaces not supported in Niri backend");
        }
        CompositorCommand::FocusMonitor(_) => {
            anyhow::bail!("FocusMonitor by ID not supported in Niri backend");
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            if dir > 0 {
                Action::FocusWorkspaceUp {}
            } else {
                Action::FocusWorkspaceDown {}
            }
        }
        CompositorCommand::NextKeyboardLayout => Action::SwitchLayout {
            layout: niri_ipc::LayoutSwitchTarget::Next,
        },
        CompositorCommand::FocusWindow(id) => {
            let id: u64 = id
                .parse()
                .map_err(|_| anyhow!("Invalid window id '{}' for Niri backend", id))?;
            Action::FocusWindow { id }
        }
        CompositorCommand::CloseWindow(id) => {
            let id: u64 = id
                .parse()
                .map_err(|_| anyhow!("Invalid window id '{}' for Niri backend", id))?;
            Action::CloseWindow { id: Some(id) }
        }
        CompositorCommand::Custom(action, args) => {
            if action == "spawn" {
                Action::Spawn {
                    command: vec![args],
                }
            } else {
                anyhow::bail!("Unknown custom dispatch: {}", action);
            }
        }
    };

    send_action(action)
}

/// An initial snapshot and the same connection that produced it.
pub(super) struct Bootstrap {
    reader: BufReader<UnixStream>,
    state: EventStreamState,
}

impl Bootstrap {
    pub(super) fn snapshot(&self) -> CompositorState {
        map_state(&self.state)
    }
}

/// Fetch the full compositor state for an explicit refresh.
pub fn fetch_full_state() -> Result<CompositorState> {
    Ok(bootstrap()?.snapshot())
}

pub(super) fn bootstrap() -> Result<Bootstrap> {
    bootstrap_stream(connect()?, Duration::from_secs(5))
}

fn bootstrap_stream(mut stream: UnixStream, timeout: Duration) -> Result<Bootstrap> {
    let deadline = Instant::now() + timeout;
    stream.set_read_timeout(Some(timeout))?;
    let request = serde_json::to_string(&Request::EventStream)? + "\n";
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let reply: Reply = serde_json::from_str(&line).context("Failed to parse handshake")?;
    reply.map_err(|e| anyhow!("Niri refused EventStream: {e}"))?;
    reader.get_ref().shutdown(std::net::Shutdown::Write)?;

    let mut state = EventStreamState::default();
    let (mut workspaces, mut overview) = (false, false);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            anyhow::bail!("Timed out waiting for Niri initial state");
        }
        reader.get_ref().set_read_timeout(Some(remaining))?;
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => anyhow::bail!("Niri event stream closed before initial state"),
            Ok(_) => {
                if let Ok(event) = serde_json::from_str::<Event>(&line) {
                    workspaces |= matches!(&event, Event::WorkspacesChanged { .. });
                    overview |= matches!(&event, Event::OverviewOpenedOrClosed { .. });
                    state.apply(event);
                    // Niri 25.05 through 26.04 emit the window snapshot and
                    // optional keyboard snapshot before the overview event.
                    if workspaces && overview {
                        reader.get_ref().set_read_timeout(None)?;
                        return Ok(Bootstrap { reader, state });
                    }
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                anyhow::bail!("Timed out waiting for Niri initial state");
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Keep the bootstrap connection, including any buffered events, for the listener.
pub(super) fn start_listener(data: Mutable<CompositorState>, token: RunToken, first: Bootstrap) {
    let mut first = Some(first);
    token.supervise_blocking("Niri", move |token| {
        let stream = match first.take() {
            Some(stream) => stream,
            None => bootstrap()?,
        };
        if token.alive() {
            data.set_neq(stream.snapshot());
        }
        run_listener(&data, token, stream)
    });
}

fn run_listener(
    data: &Mutable<CompositorState>,
    token: &RunToken,
    stream: Bootstrap,
) -> Result<()> {
    info!("Starting Niri event listener");
    let Bootstrap {
        mut reader,
        mut state,
    } = stream;
    token.active();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            anyhow::bail!("Niri event stream closed");
        }
        let event: Event = match serde_json::from_str(&line) {
            Ok(event) => event,
            Err(error) => {
                debug!(
                    "Failed to parse Niri event (IPC version mismatch): {:?}",
                    error
                );
                continue;
            }
        };
        state.apply(event);
        if token.alive() {
            data.set_neq(map_state(&state));
        }
    }
}

/// Connect to the niri IPC socket.
fn connect() -> Result<UnixStream> {
    let socket_path = env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .ok_or_else(|| anyhow!("NIRI_SOCKET or NIRI_SOCKET_PATH environment variable not set"))?;

    UnixStream::connect(socket_path).context("Failed to connect to Niri socket")
}

/// Send a single action to niri and read the reply.
fn send_action(action: Action) -> Result<()> {
    let mut stream = connect()?;

    let mut json = serde_json::to_string(&Request::Action(action))?;
    json.push('\n');
    stream.write_all(json.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    let reply: Reply = serde_json::from_str(&response_line)?;
    reply.map_err(|e| anyhow!("Niri error: {}", e)).map(|_| ())
}

/// Map niri's internal EventStreamState to our generic CompositorState.
fn map_state(niri: &EventStreamState) -> CompositorState {
    // Build a map of output name -> active workspace id
    let output_to_active_ws: HashMap<_, _> = niri
        .workspaces
        .workspaces
        .values()
        .filter_map(|ws| {
            if let Some(out) = &ws.output
                && ws.is_active
            {
                Some((out.clone(), ws.id as i32))
            } else {
                None
            }
        })
        .collect();

    // Sort outputs for stable monitor IDs (matches niri's internal ordering)
    let outputs: Vec<_> = output_to_active_ws.keys().sorted_unstable().collect();

    let mut workspaces: Vec<Workspace> = niri
        .workspaces
        .workspaces
        .values()
        .sorted_by_key(|w| w.idx)
        .map(|w| Workspace {
            id: w.id as i32,
            index: w.idx as i32,
            name: w.name.clone().unwrap_or_else(|| w.idx.to_string()),
            monitor: w.output.clone().unwrap_or_default(),
            monitor_id: w.output.as_ref().map(|wo| {
                outputs
                    .iter()
                    .position(|o| *o == wo)
                    .map_or(-1, |i| i as i32) as i128
            }),
            windows: 0,
            is_special: false,
            has_urgent: w.is_urgent,
        })
        .collect();

    // Window counts per workspace; older niri reports urgency on windows only.
    for win in niri.windows.windows.values() {
        if let Some(ws_id) = win.workspace_id
            && let Some(ws) = niri.workspaces.workspaces.get(&ws_id)
            && let Some(generic_ws) = workspaces.iter_mut().find(|w| w.id == ws.id as i32)
        {
            generic_ws.windows += 1;
            generic_ws.has_urgent |= win.is_urgent;
        }
    }

    // Build monitors
    let monitors: Vec<Monitor> = output_to_active_ws
        .iter()
        .map(|(name, active_ws_id)| Monitor {
            id: outputs
                .iter()
                .position(|o| *o == name)
                .map_or(-1, |i| i as i128),
            name: name.clone(),
            active_workspace_id: *active_ws_id,
            special_workspace_id: -1,
            ..Default::default()
        })
        .collect();

    let active_workspace_id = niri
        .workspaces
        .workspaces
        .values()
        .find(|w| w.is_focused)
        .map(|w| w.id as i32);

    let active_window = niri
        .windows
        .windows
        .values()
        .find(|w| w.is_focused)
        .map(|w| ActiveWindow {
            title: w.title.clone().unwrap_or_default(),
            class: w.app_id.clone().unwrap_or_default(),
            address: w.id.to_string(),
        });

    let keyboard_layout = niri.keyboard_layouts.keyboard_layouts.as_ref().map_or_else(
        || "Unknown".to_string(),
        |k| {
            k.names
                .get(k.current_idx as usize)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string())
        },
    );

    let windows: Vec<Window> = niri
        .windows
        .windows
        .values()
        .map(|w| Window {
            id: w.id.to_string(),
            app_id: w.app_id.clone().unwrap_or_default(),
            title: w.title.clone().unwrap_or_default(),
            monitor: w
                .workspace_id
                .and_then(|ws_id| niri.workspaces.workspaces.get(&ws_id))
                .and_then(|ws| ws.output.clone())
                .unwrap_or_default(),
            workspace_id: w.workspace_id.unwrap_or(0) as i32,
            is_focused: w.is_focused,
            is_minimized: false,
            is_urgent: w.is_urgent,
            pid: w.pid.and_then(|pid| u32::try_from(pid).ok()),
            geometry: None,
        })
        .collect();

    CompositorState {
        workspaces,
        monitors,
        active_workspace_id,
        active_window,
        windows,
        keyboard_layout,
        submap: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{Lifecycle, ServiceMode};
    use futures_signals::signal::SignalExt;
    use futures_util::{FutureExt, StreamExt};
    use std::{os::unix::net::UnixListener, sync::mpsc, thread};

    // Wire fixtures are intentionally independent of niri-ipc serialization.
    const WORKSPACES: &str = "{\"WorkspacesChanged\":{\"workspaces\":[{\"id\":1,\"idx\":1,\"name\":null,\"output\":\"DP-1\",\"is_urgent\":false,\"is_active\":true,\"is_focused\":true,\"active_window_id\":1}]}}\n";
    const WINDOWS: &str = "{\"WindowsChanged\":{\"windows\":[{\"id\":1,\"title\":\"before\",\"app_id\":\"test\",\"pid\":null,\"workspace_id\":1,\"is_focused\":true,\"is_floating\":false,\"is_urgent\":false,\"layout\":{\"pos_in_scrolling_layout\":null,\"tile_size\":[100.0,100.0],\"window_size\":[100,100],\"tile_pos_in_workspace_view\":null,\"window_offset_in_tile\":[0.0,0.0]}}]}}\n";
    const KEYBOARD: &str = "{\"KeyboardLayoutsChanged\":{\"keyboard_layouts\":{\"names\":[\"English\"],\"current_idx\":0}}}\n";
    const OVERVIEW: &str = "{\"OverviewOpenedOrClosed\":{\"is_open\":false}}\n";
    const FOCUS_NONE: &str = "{\"WindowFocusChanged\":{\"id\":null}}\n";
    const FOCUS_ONE: &str = "{\"WindowFocusChanged\":{\"id\":1}}\n";
    const CLOSE_ONE: &str = "{\"WindowClosed\":{\"id\":1}}\n";

    fn send_reply(stream: &mut UnixStream, events: &str) {
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        assert_eq!(request.trim(), "\"EventStream\"");
        stream
            .write_all(format!("{{\"Ok\":\"Handled\"}}\n{events}").as_bytes())
            .unwrap();
    }

    #[test]
    fn bootstrap_preserves_buffered_events_and_order_on_one_unix_connection() {
        let dir = std::env::temp_dir().join(format!("gpui-niri-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("niri.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let reopen = WINDOWS
                .replace(
                    "{\"WindowsChanged\":{\"windows\":[",
                    "{\"WindowOpenedOrChanged\":{\"window\":",
                )
                .replace("]}}\n", "}}\n")
                .replace("\"id\":1,", "\"id\":2,")
                .replace("before", "after");
            send_reply(
                &mut socket,
                &format!(
                    "{WORKSPACES}{WINDOWS}{KEYBOARD}{OVERVIEW}{CLOSE_ONE}{reopen}{FOCUS_NONE}"
                ),
            );
            done_rx.recv().unwrap();
        });
        let socket = UnixStream::connect(&path).unwrap();
        let boot = bootstrap_stream(socket, Duration::from_secs(2)).unwrap();
        let initial = boot.snapshot();
        assert!(initial.active_window.is_some());
        assert_eq!(initial.keyboard_layout, "English");
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let token = lifecycle.begin();
        let data = Mutable::new(initial);
        done_tx.send(()).unwrap();
        assert!(run_listener(&data, &token, boot).is_err());
        assert!(data.lock_ref().active_window.is_none());
        assert_eq!(data.lock_ref().windows.len(), 1);
        assert_eq!(data.lock_ref().windows[0].id, "2");
        assert_eq!(data.lock_ref().windows[0].title, "after");
        server.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bootstrap_finishes_during_continuous_events_without_config_loaded() {
        let (client, mut server) = UnixStream::pair().unwrap();
        let thread = thread::spawn(move || {
            send_reply(&mut server, &format!("{WORKSPACES}{WINDOWS}{OVERVIEW}"));
            for _ in 0..200 {
                if server.write_all(FOCUS_ONE.as_bytes()).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        let boot = bootstrap_stream(client, Duration::from_secs(1)).unwrap();
        assert!(boot.snapshot().active_window.is_some());
        drop(boot);
        thread.join().unwrap();
    }

    #[test]
    fn bootstrap_waits_for_each_required_snapshot_even_if_order_changes() {
        let (client, mut server) = UnixStream::pair().unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            send_reply(
                &mut server,
                &format!("{KEYBOARD}{OVERVIEW}{WINDOWS}{WORKSPACES}"),
            );
            done_rx.recv().unwrap();
        });
        let boot = bootstrap_stream(client, Duration::from_secs(1)).unwrap();
        let state = boot.snapshot();
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.windows.len(), 1);
        assert_eq!(state.keyboard_layout, "English");
        done_tx.send(()).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn bootstrap_times_out_if_snapshot_never_completes_during_events() {
        let (client, mut server) = UnixStream::pair().unwrap();
        let thread = thread::spawn(move || {
            send_reply(&mut server, &format!("{WORKSPACES}{WINDOWS}"));
            for _ in 0..100 {
                if server.write_all(FOCUS_ONE.as_bytes()).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        assert!(bootstrap_stream(client, Duration::from_millis(200)).is_err());
        thread.join().unwrap();
    }

    #[test]
    fn populated_window_schema_compatibility() {
        // Niri 25.08 has layout; 25.11 and 26.04 also have focus_timestamp.
        for windows in [
            WINDOWS.to_string(),
            WINDOWS.replace("\"layout\":", "\"focus_timestamp\":null,\"layout\":"),
        ] {
            let (client, mut server) = UnixStream::pair().unwrap();
            let (done_tx, done_rx) = mpsc::channel();
            let thread = thread::spawn(move || {
                send_reply(&mut server, &format!("{WORKSPACES}{windows}{OVERVIEW}"));
                done_rx.recv().unwrap();
            });
            assert_eq!(
                bootstrap_stream(client, Duration::from_secs(1))
                    .unwrap()
                    .snapshot()
                    .windows
                    .len(),
                1
            );
            done_tx.send(()).unwrap();
            thread.join().unwrap();
        }
        // The 25.05 window schema has no layout and was already incompatible
        // with the niri-ipc 26.4 Window type used by this project.
        let old = "{\"WindowsChanged\":{\"windows\":[{\"id\":1,\"is_focused\":true,\"is_floating\":false,\"is_urgent\":false}]}}";
        assert!(serde_json::from_str::<Event>(old).is_err());

        // Preserve startup on the older schema, although the existing IPC
        // dependency cannot deserialize its populated window snapshot.
        let (client, mut server) = UnixStream::pair().unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            send_reply(&mut server, &format!("{WORKSPACES}{old}\n{OVERVIEW}"));
            done_rx.recv().unwrap();
        });
        let state = bootstrap_stream(client, Duration::from_secs(2))
            .unwrap()
            .snapshot();
        assert_eq!(state.workspaces.len(), 1);
        assert!(state.windows.is_empty());
        done_tx.send(()).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn unchanged_events_do_not_notify_subscribers() {
        let (client, mut server) = UnixStream::pair().unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            send_reply(
                &mut server,
                &format!("{WORKSPACES}{WINDOWS}{OVERVIEW}{OVERVIEW}"),
            );
            done_rx.recv().unwrap();
        });
        let boot = bootstrap_stream(client, Duration::from_secs(2)).unwrap();
        let data = Mutable::new(boot.snapshot());
        let mut signal = Box::pin(data.signal_cloned().to_stream());
        assert!(signal.next().now_or_never().is_some());
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let token = lifecycle.begin();
        done_tx.send(()).unwrap();
        assert!(run_listener(&data, &token, boot).is_err());
        assert!(signal.next().now_or_never().is_none());
        thread.join().unwrap();
    }
}
