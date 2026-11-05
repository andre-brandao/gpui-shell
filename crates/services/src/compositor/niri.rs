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
    Action, Event, Reply, Request, Response, WorkspaceReferenceArg,
    state::{EventStreamState, EventStreamStatePart},
};
use tracing::info;

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

    fn publish(&self, data: &Mutable<CompositorState>) {
        publish_state(&self.state, data);
    }
}

fn publish_state(state: &EventStreamState, data: &Mutable<CompositorState>) {
    let mut snapshot = map_state(state);
    let mut current = data.lock_mut();
    if state.keyboard_layouts.keyboard_layouts.is_none() {
        snapshot
            .keyboard_layout
            .clone_from(&current.keyboard_layout);
    }
    if *current != snapshot {
        *current = snapshot;
    }
}

/// Refresh initialized domains without discarding optional runtime state.
pub fn refresh(data: &Mutable<CompositorState>) -> Result<()> {
    bootstrap()?.publish(data);
    Ok(())
}

pub(super) fn bootstrap() -> Result<Bootstrap> {
    bootstrap_stream(connect()?, Duration::from_secs(5))
}

// Only windows and workspaces gate readiness; keyboard layouts may arrive later.
fn bootstrap_stream(mut stream: UnixStream, timeout: Duration) -> Result<Bootstrap> {
    let deadline = Instant::now() + timeout;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(b"\"EventStream\"\n")?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut reader = BufReader::new(stream);
    let reply: Reply = serde_json::from_slice(&read_before(&mut reader, deadline)?)
        .context("Failed to parse Niri handshake")?;
    if !matches!(
        reply.map_err(|e| anyhow!("Niri refused EventStream: {e}"))?,
        Response::Handled
    ) {
        anyhow::bail!("Unexpected Niri EventStream response");
    }

    let mut state = EventStreamState::default();
    let (mut workspaces, mut windows) = (false, false);
    loop {
        let event: Event = serde_json::from_slice(&read_before(&mut reader, deadline)?)
            .context("Invalid Niri initial event (incompatible IPC schema)")?;
        let snapshot_received = match &event {
            Event::WorkspaceUrgencyChanged { .. }
            | Event::WorkspaceActivated { .. }
            | Event::WorkspaceActiveWindowChanged { .. } => workspaces,
            Event::WindowOpenedOrChanged { .. }
            | Event::WindowClosed { .. }
            | Event::WindowFocusChanged { .. }
            | Event::WindowFocusTimestampChanged { .. }
            | Event::WindowUrgencyChanged { .. }
            | Event::WindowLayoutsChanged { .. } => windows,
            _ => true,
        };
        if !snapshot_received {
            anyhow::bail!("Niri sent an update before its initial snapshot");
        }
        workspaces |= matches!(&event, Event::WorkspacesChanged { .. });
        windows |= matches!(&event, Event::WindowsChanged { .. });
        apply_event(&mut state, event)?;
        if workspaces && windows {
            reader.get_ref().set_read_timeout(None)?;
            reader.get_ref().set_write_timeout(None)?;
            return Ok(Bootstrap { reader, state });
        }
    }
}

// read_line's timeout restarts on each read, so a partial-line trickle can evade it.
fn read_before(reader: &mut BufReader<UnixStream>, deadline: Instant) -> Result<Vec<u8>> {
    let mut line = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            anyhow::bail!("Timed out waiting for Niri initial state");
        }
        reader.get_ref().set_read_timeout(Some(remaining))?;
        let bytes = reader.fill_buf().context("Reading Niri initial state")?;
        if bytes.is_empty() {
            anyhow::bail!("Niri event stream closed before initial state");
        }
        if Instant::now() >= deadline {
            anyhow::bail!("Timed out waiting for Niri initial state");
        }
        let end = bytes.iter().position(|byte| *byte == b'\n');
        let consumed = end.map_or(bytes.len(), |end| end + 1);
        line.extend_from_slice(&bytes[..consumed]);
        reader.consume(consumed);
        if end.is_some() {
            return Ok(line);
        }
    }
}

// Reject invalid references before niri-ipc's state tracker can panic.
fn apply_event(state: &mut EventStreamState, event: Event) -> Result<()> {
    let valid = match &event {
        Event::WorkspaceActivated { id, .. } => state.workspaces.workspaces.contains_key(id),
        Event::WorkspaceActiveWindowChanged { workspace_id, .. } => {
            state.workspaces.workspaces.contains_key(workspace_id)
        }
        Event::WindowClosed { id } => state.windows.windows.contains_key(id),
        Event::WindowLayoutsChanged { changes } => changes
            .iter()
            .all(|(id, _)| state.windows.windows.contains_key(id)),
        Event::KeyboardLayoutSwitched { .. } => state.keyboard_layouts.keyboard_layouts.is_some(),
        Event::CastStopped { stream_id } => state.casts.casts.contains_key(stream_id),
        _ => true,
    };
    if !valid {
        anyhow::bail!("Niri event references missing state");
    }
    state.apply(event);
    Ok(())
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
            stream.publish(&data);
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
        if !line.ends_with('\n') {
            anyhow::bail!("Niri event stream closed with an incomplete record");
        }
        let event: Event =
            serde_json::from_str(&line).context("Invalid Niri event (incompatible IPC schema)")?;
        apply_event(&mut state, event)?;
        if token.alive() {
            publish_state(&state, data);
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

    fn fixture(events: String) -> (UnixStream, mpsc::Sender<()>, thread::JoinHandle<()>) {
        let (client, mut server) = UnixStream::pair().unwrap();
        let (done, wait) = mpsc::channel();
        let thread = thread::spawn(move || {
            send_reply(&mut server, &events);
            let _ = wait.recv();
        });
        (client, done, thread)
    }

    fn bootstrap_fixture(events: String) -> Result<Bootstrap> {
        let (client, done, thread) = fixture(events);
        let result = bootstrap_stream(client, Duration::from_secs(2));
        done.send(()).unwrap();
        thread.join().unwrap();
        result
    }

    #[test]
    fn snapshots_complete_in_either_order_without_overview_or_config_loaded() {
        for events in [
            format!("{WORKSPACES}{WINDOWS}"),
            format!("{WINDOWS}{WORKSPACES}"),
        ] {
            let boot = bootstrap_fixture(events).unwrap();
            assert_eq!(boot.snapshot().windows.len(), 1);
            assert_eq!(boot.snapshot().workspaces.len(), 1);
            assert_eq!(boot.snapshot().keyboard_layout, "Unknown");
        }
    }

    #[test]
    fn bootstrap_finishes_during_continuous_window_events() {
        let (client, mut server) = UnixStream::pair().unwrap();
        let thread = thread::spawn(move || {
            send_reply(&mut server, &format!("{WORKSPACES}{WINDOWS}"));
            while server.write_all(FOCUS_ONE.as_bytes()).is_ok() {}
        });
        let boot = bootstrap_stream(client, Duration::from_secs(2)).unwrap();
        assert!(boot.snapshot().active_window.is_some());
        drop(boot);
        thread.join().unwrap();
    }

    #[test]
    fn each_required_snapshot_is_required_despite_continuous_activity() {
        for partial in [WORKSPACES, WINDOWS] {
            let (client, mut server) = UnixStream::pair().unwrap();
            let thread = thread::spawn(move || {
                send_reply(&mut server, partial);
                while server.write_all(OVERVIEW.as_bytes()).is_ok() {}
            });
            assert!(bootstrap_stream(client, Duration::from_millis(100)).is_err());
            thread.join().unwrap();
        }
    }

    #[test]
    fn bootstrap_preserves_buffered_events_exactly_once_on_one_unix_connection() {
        let dir = std::env::temp_dir().join(format!("gpui-niri-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("niri.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (done, wait) = mpsc::channel();
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
                &format!("{WORKSPACES}{WINDOWS}{KEYBOARD}{CLOSE_ONE}{reopen}{FOCUS_NONE}"),
            );
            let _ = wait.recv();
        });
        let boot =
            bootstrap_stream(UnixStream::connect(&path).unwrap(), Duration::from_secs(2)).unwrap();
        assert_eq!(boot.snapshot().keyboard_layout, "Unknown");
        let data = Mutable::new(boot.snapshot());
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        done.send(()).unwrap();
        let error = run_listener(&data, &lifecycle.begin(), boot).unwrap_err();
        assert!(error.to_string().contains("closed"));
        let state = data.lock_ref();
        assert_eq!(state.windows.len(), 1);
        assert_eq!(state.windows[0].id, "2");
        assert_eq!(state.windows[0].title, "after");
        assert!(state.active_window.is_none());
        assert_eq!(state.keyboard_layout, "English");
        server.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn incremental_events_before_their_domain_snapshot_fail_safely() {
        assert!(bootstrap_fixture(format!("{WORKSPACES}{FOCUS_ONE}{WINDOWS}")).is_err());
        assert!(
            bootstrap_fixture(format!(
                "{WINDOWS}{{\"WorkspaceActivated\":{{\"id\":1,\"focused\":true}}}}\n{WORKSPACES}"
            ))
            .is_err()
        );
    }

    #[test]
    fn malformed_and_incompatible_snapshots_fail_instead_of_publishing_partial_state() {
        let old = "{\"WindowsChanged\":{\"windows\":[{\"id\":1,\"is_focused\":true,\"is_floating\":false,\"is_urgent\":false}]}}\n";
        for malformed in ["not JSON\n", "{\"WindowsChanged\":{}}\n", old] {
            assert!(bootstrap_fixture(format!("{WORKSPACES}{malformed}{OVERVIEW}")).is_err());
        }
    }

    #[test]
    fn populated_window_schema_compatibility() {
        for (version, windows) in [
            ("25.08", WINDOWS.to_string()),
            (
                "25.11",
                WINDOWS.replace("\"layout\":", "\"focus_timestamp\":null,\"layout\":"),
            ),
            (
                "26.04",
                WINDOWS.replace("\"layout\":", "\"focus_timestamp\":null,\"layout\":"),
            ),
        ] {
            assert_eq!(
                bootstrap_fixture(format!("{WORKSPACES}{windows}"))
                    .unwrap()
                    .snapshot()
                    .windows
                    .len(),
                1,
                "{version}"
            );
        }
    }

    #[test]
    fn disconnect_and_truncated_json_during_bootstrap_fail() {
        for partial in ["", WORKSPACES, "{\"WindowsChanged\":"] {
            let (client, mut server) = UnixStream::pair().unwrap();
            let thread = thread::spawn(move || send_reply(&mut server, partial));
            assert!(bootstrap_stream(client, Duration::from_secs(2)).is_err());
            thread.join().unwrap();
        }
    }

    #[test]
    fn read_before_handles_json_split_across_buffer_fills() {
        let (client, mut server) = UnixStream::pair().unwrap();
        server.write_all(WORKSPACES.as_bytes()).unwrap();
        let mut reader = BufReader::with_capacity(1, client);
        let line = read_before(&mut reader, Instant::now() + Duration::from_secs(2)).unwrap();
        assert_eq!(line, WORKSPACES.as_bytes());
    }

    #[test]
    fn deadline_applies_even_to_buffered_bytes() {
        let (client, mut server) = UnixStream::pair().unwrap();
        server.write_all(WORKSPACES.as_bytes()).unwrap();
        let mut reader = BufReader::new(client);
        assert!(!reader.fill_buf().unwrap().is_empty());
        assert!(read_before(&mut reader, Instant::now()).is_err());
        assert_eq!(reader.buffer(), WORKSPACES.as_bytes());
    }

    #[test]
    fn silence_and_continuous_partial_json_cannot_extend_deadline() {
        let (client, _server) = UnixStream::pair().unwrap();
        assert!(
            read_before(
                &mut BufReader::new(client),
                Instant::now() + Duration::from_millis(30)
            )
            .is_err()
        );

        let (client, mut server) = UnixStream::pair().unwrap();
        let thread = thread::spawn(move || while server.write_all(b" ").is_ok() {});
        let mut reader = BufReader::new(client);
        assert!(read_before(&mut reader, Instant::now() + Duration::from_millis(100)).is_err());
        drop(reader);
        thread.join().unwrap();
    }

    #[test]
    fn invalid_handshake_is_not_accepted() {
        for reply in [
            "{\"Ok\":{\"Version\":\"26.04\"}}\n",
            "{\"Err\":\"refused\"}\n",
            "broken\n",
        ] {
            let (client, mut server) = UnixStream::pair().unwrap();
            let thread = thread::spawn(move || {
                let mut request = String::new();
                BufReader::new(server.try_clone().unwrap())
                    .read_line(&mut request)
                    .unwrap();
                server.write_all(reply.as_bytes()).unwrap();
            });
            assert!(bootstrap_stream(client, Duration::from_secs(2)).is_err());
            thread.join().unwrap();
        }
    }

    #[test]
    fn reconnect_replaces_old_state_and_processes_new_events() {
        let first = bootstrap_fixture(format!("{WORKSPACES}{WINDOWS}")).unwrap();
        let data = Mutable::new(first.snapshot());
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let token = lifecycle.begin();
        assert!(run_listener(&data, &token, first).is_err());
        let next_windows = WINDOWS.replace("before", "reconnected");
        let (socket, done, thread) = fixture(format!("{WORKSPACES}{next_windows}{FOCUS_NONE}"));
        let next = bootstrap_stream(socket, Duration::from_secs(2)).unwrap();
        next.publish(&data);
        done.send(()).unwrap();
        assert!(run_listener(&data, &token, next).is_err());
        assert_eq!(data.lock_ref().windows[0].title, "reconnected");
        assert!(data.lock_ref().active_window.is_none());
        thread.join().unwrap();
    }

    #[test]
    fn invalid_delta_references_return_errors_without_panicking() {
        for event in [
            CLOSE_ONE,
            "{\"WorkspaceActivated\":{\"id\":1,\"focused\":true}}\n",
            "{\"KeyboardLayoutSwitched\":{\"idx\":0}}\n",
        ] {
            let event = serde_json::from_str(event).unwrap();
            assert!(apply_event(&mut EventStreamState::default(), event).is_err());
        }
    }

    #[test]
    fn refresh_and_reconnect_preserve_layout_until_its_snapshot_arrives() {
        let first = bootstrap_fixture(format!("{KEYBOARD}{WORKSPACES}{WINDOWS}")).unwrap();
        let data = Mutable::new(first.snapshot());
        let refreshed = bootstrap_fixture(format!("{WORKSPACES}{WINDOWS}")).unwrap();
        refreshed.publish(&data);
        assert_eq!(data.lock_ref().keyboard_layout, "English");

        let keyboard = KEYBOARD.replace("English", "German");
        let (socket, done, thread) =
            fixture(format!("{WORKSPACES}{WINDOWS}{FOCUS_NONE}{keyboard}"));
        let next = bootstrap_stream(socket, Duration::from_secs(2)).unwrap();
        next.publish(&data);
        let mut pending = next.state;
        apply_event(&mut pending, serde_json::from_str(FOCUS_NONE).unwrap()).unwrap();
        publish_state(&pending, &data);
        assert_eq!(data.lock_ref().keyboard_layout, "English");
        let next = Bootstrap {
            reader: next.reader,
            state: pending,
        };
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        done.send(()).unwrap();
        assert!(run_listener(&data, &lifecycle.begin(), next).is_err());
        assert_eq!(data.lock_ref().keyboard_layout, "German");
        thread.join().unwrap();
    }

    #[test]
    fn unterminated_runtime_record_is_not_published() {
        let (socket, done, thread) =
            fixture(format!("{WORKSPACES}{WINDOWS}{}", FOCUS_NONE.trim_end()));
        let boot = bootstrap_stream(socket, Duration::from_secs(2)).unwrap();
        let data = Mutable::new(boot.snapshot());
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        done.send(()).unwrap();
        let error = run_listener(&data, &lifecycle.begin(), boot).unwrap_err();
        assert!(error.to_string().contains("incomplete record"));
        assert!(data.lock_ref().active_window.is_some());
        thread.join().unwrap();
    }

    #[test]
    fn unchanged_events_do_not_notify_subscribers() {
        let (client, done, thread) = fixture(format!("{WORKSPACES}{WINDOWS}{OVERVIEW}"));
        let boot = bootstrap_stream(client, Duration::from_secs(2)).unwrap();
        let data = Mutable::new(boot.snapshot());
        let mut signal = Box::pin(data.signal_cloned().to_stream());
        assert!(signal.next().now_or_never().is_some());
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        done.send(()).unwrap();
        assert!(run_listener(&data, &lifecycle.begin(), boot).is_err());
        assert!(signal.next().now_or_never().is_none());
        thread.join().unwrap();
    }
}
