//! Privacy service for monitoring camera, microphone, and screenshare access.
//!
//! This module provides a reactive subscriber for tracking media stream access
//! via PipeWire and webcam device usage via inotify.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
use inotify::{EventMask, Inotify, WatchMask};
use tracing::{debug, warn};

use crate::lifecycle::{Lifecycle, ManagedService, RunToken};

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";

/// Media type being accessed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    /// Video stream (screenshare).
    Video,
    /// Audio stream (microphone).
    Audio,
}

/// An application node accessing media via PipeWire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationNode {
    /// PipeWire node ID.
    pub id: u32,
    /// Type of media being accessed.
    pub media: Media,
}

/// Privacy-related data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivacyData {
    /// Active PipeWire media stream nodes.
    pub nodes: Vec<ApplicationNode>,
    /// Number of processes with webcam device open.
    pub webcam_access: i32,
}

impl PrivacyData {
    /// Returns true if nothing is accessing camera/mic.
    pub fn no_access(&self) -> bool {
        self.nodes.is_empty() && self.webcam_access == 0
    }

    /// Returns true if microphone is being accessed.
    pub fn microphone_access(&self) -> bool {
        self.nodes.iter().any(|n| n.media == Media::Audio)
    }

    /// Returns true if webcam is being accessed.
    pub fn webcam_access(&self) -> bool {
        self.webcam_access > 0
    }

    /// Returns true if screen is being shared.
    pub fn screenshare_access(&self) -> bool {
        self.nodes.iter().any(|n| n.media == Media::Video)
    }
}

/// Event-driven privacy subscriber.
///
/// This subscriber monitors media access via PipeWire and webcam device
/// usage via inotify, providing reactive state updates through `futures_signals`.
#[derive(Debug, Clone, Default)]
pub struct PrivacySubscriber {
    data: Mutable<PrivacyData>,
    lifecycle: Lifecycle,
}

impl PrivacySubscriber {
    /// Create a new privacy subscriber. Monitoring starts with
    /// [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a signal that emits when privacy state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<PrivacyData> {
        self.data.signal_cloned()
    }

    /// Get the current privacy data snapshot.
    pub fn get(&self) -> PrivacyData {
        self.data.get_cloned()
    }
}

impl ManagedService for PrivacySubscriber {
    fn name(&self) -> &'static str {
        "Privacy"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    fn memory_bytes(&self) -> usize {
        size_of::<PrivacyData>() + self.data.lock_ref().nodes.len() * size_of::<ApplicationNode>()
    }

    /// Both watchers share one run token: an error in either surfaces as the
    /// service's status, and stopping silences both.
    fn start(&self) {
        let token = self.lifecycle.begin();
        let data = self.data.clone();
        token
            .clone()
            .supervise_blocking("PipeWire", move |token| run_pipewire_listener(&data, token));
        let data = self.data.clone();
        token.supervise("Webcam watcher", move |token| {
            run_webcam_watcher(data.clone(), token)
        });
    }
}

/// A sink monitor sets `stream.capture.sink`: it records the speakers, not the microphone.
fn captured_media(media_class: &str, capture_sink: Option<&str>) -> Option<Media> {
    match media_class {
        "Stream/Input/Video" => Some(Media::Video),
        "Stream/Input/Audio" if !matches!(capture_sink, Some("true" | "1")) => Some(Media::Audio),
        _ => None,
    }
}

/// Add or drop a node only on a change: each write wakes every subscriber.
fn track_node(data: &Mutable<PrivacyData>, id: u32, media: Option<Media>) {
    let tracked = data.lock_ref().nodes.iter().any(|n| n.id == id);
    match media {
        Some(media) if !tracked => {
            debug!("New media node: id={id}, media={media:?}");
            data.lock_mut().nodes.push(ApplicationNode { id, media });
        }
        None if tracked => {
            debug!("Removed tracked media node: {id}");
            data.lock_mut().nodes.retain(|n| n.id != id);
        }
        _ => {}
    }
}

/// One PipeWire session (blocking). The loop is not async: a message on a
/// loop-attached channel quits it when the run ends, and a core error (the
/// daemon went away) quits it to fail the attempt.
fn run_pipewire_listener(data: &Mutable<PrivacyData>, token: &RunToken) -> anyhow::Result<()> {
    use pipewire::{
        context::ContextRc,
        core::PW_ID_CORE,
        main_loop::MainLoopRc,
        node::{Node, NodeChangeMask, NodeListener},
    };

    let mainloop = MainLoopRc::new(None)?;
    let (quit_tx, quit_rx) = pipewire::channel::channel::<()>();
    let _quit = quit_rx.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |()| mainloop.quit()
    });
    // Dropped when this attempt returns, which retires the canceller with it.
    let (_attempt, attempt_done) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn({
        let token = token.clone();
        async move {
            tokio::select! {
                _ = token.cancelled() => {
                    let _ = quit_tx.send(());
                }
                _ = attempt_done => {}
            }
        }
    });

    let context = ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;
    let failure = Rc::new(RefCell::new(None::<String>));
    let _core_listener = core
        .add_listener_local()
        .error({
            let failure = failure.clone();
            let mainloop = mainloop.clone();
            move |id, _seq, res, message| {
                if id == PW_ID_CORE {
                    *failure.borrow_mut() = Some(format!("{message} ({res})"));
                    mainloop.quit();
                }
            }
        })
        .register();
    let registry = core.get_registry_rc()?;

    // The registry replays every live node on connect.
    token.write(data, |data| data.nodes.clear());

    // The registry omits `stream.capture.sink`, so each capture node stays bound to read it.
    // The listener comes first in the tuple: it must drop before its proxy.
    let bound: Rc<RefCell<HashMap<u32, (NodeListener, Node)>>> = Rc::default();
    let bound_remove = bound.clone();
    let registry_bind = registry.clone();
    let data_add = data.clone();
    let data_remove = data.clone();
    let token_add = token.clone();
    let token_remove = token.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if !token_add.alive() {
                return;
            }
            let Some(class) = global.props.and_then(|props| props.get("media.class")) else {
                return;
            };
            if !matches!(class, "Stream/Input/Video" | "Stream/Input/Audio") {
                return;
            }
            let node: Node = match registry_bind.bind(global) {
                Ok(node) => node,
                Err(e) => {
                    warn!("Could not bind media node {}: {}", global.id, e);
                    track_node(&data_add, global.id, captured_media(class, None));
                    return;
                }
            };
            let id = global.id;
            let data = data_add.clone();
            let token = token_add.clone();
            let listener = node
                .add_listener_local()
                .info(move |info| {
                    // Props are only valid when the mask says so.
                    if !token.alive() || !info.change_mask().contains(NodeChangeMask::PROPS) {
                        return;
                    }
                    let media = info.props().and_then(|props| {
                        captured_media(props.get("media.class")?, props.get("stream.capture.sink"))
                    });
                    track_node(&data, id, media);
                })
                .register();
            bound.borrow_mut().insert(id, (listener, node));
        })
        .global_remove(move |id| {
            if !token_remove.alive() {
                return;
            }
            bound_remove.borrow_mut().remove(&id);
            track_node(&data_remove, id, None);
        })
        .register();

    token.active();
    mainloop.run();

    let failure = failure.borrow_mut().take();
    anyhow::bail!(
        "PipeWire connection lost: {}",
        failure.as_deref().unwrap_or("loop ended")
    )
}

/// Run the webcam watcher. The inotify fd sits on the tokio reactor, so a
/// stop drops the watch at once instead of at the next device event.
async fn run_webcam_watcher(data: Mutable<PrivacyData>, token: RunToken) -> anyhow::Result<()> {
    // Check if webcam device exists
    if !Path::new(WEBCAM_DEVICE_PATH).exists() {
        warn!("Webcam device not found: {}", WEBCAM_DEVICE_PATH);
        return Ok(());
    }

    let inotify = Inotify::init()?;

    inotify.watches().add(
        WEBCAM_DEVICE_PATH,
        WatchMask::CLOSE_WRITE
            | WatchMask::CLOSE_NOWRITE
            | WatchMask::DELETE_SELF
            | WatchMask::OPEN
            | WatchMask::ATTRIB,
    )?;

    let mut events = inotify.into_event_stream([0; 1024])?;
    // Watching first, so an open between the count and the watch is not lost.
    let in_use = tokio::task::spawn_blocking(|| is_device_in_use(WEBCAM_DEVICE_PATH)).await?;
    data.lock_mut().webcam_access = in_use;
    token.active();

    while let Some(event) = events.next().await {
        let event = event?;
        debug!("Webcam event: {:?}", event.mask);

        if event.mask.contains(EventMask::OPEN) {
            data.lock_mut().webcam_access += 1;
            debug!("Webcam opened: {}", data.lock_ref().webcam_access);
        } else if event.mask.contains(EventMask::CLOSE_WRITE)
            || event.mask.contains(EventMask::CLOSE_NOWRITE)
        {
            let mut guard = data.lock_mut();
            guard.webcam_access = i32::max(guard.webcam_access - 1, 0);
            debug!("Webcam closed: {}", guard.webcam_access);
        } else if event.mask.contains(EventMask::DELETE_SELF) {
            warn!("Webcam device was deleted");
            return Ok(());
        }
    }

    anyhow::bail!("webcam event stream ended")
}

/// Check how many processes have a device file open.
fn is_device_in_use(target: &str) -> i32 {
    let mut used_by = 0;

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let pid_path = entry.path();

            // Skip non-numeric directories (not process folders)
            let fd_path = pid_path.join("fd");
            if !fd_path.exists() {
                continue;
            }

            // Check file descriptors in each process folder
            if let Ok(fd_entries) = fs::read_dir(&fd_path) {
                for fd_entry in fd_entries.flatten() {
                    if let Ok(link_path) = fs::read_link(fd_entry.path())
                        && link_path == Path::new(target)
                    {
                        used_by += 1;
                    }
                }
            }
        }
    }

    used_by
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sink_monitor_is_not_the_microphone() {
        assert_eq!(
            captured_media("Stream/Input/Audio", None),
            Some(Media::Audio)
        );
        assert_eq!(captured_media("Stream/Input/Audio", Some("true")), None);
        assert_eq!(captured_media("Stream/Input/Audio", Some("1")), None);
        assert_eq!(
            captured_media("Stream/Input/Video", None),
            Some(Media::Video)
        );
        assert_eq!(captured_media("Stream/Output/Audio", None), None);
    }

    #[test]
    fn a_repeated_node_event_does_not_write() {
        use futures_signals::signal::SignalExt;
        use futures_util::FutureExt;

        let data = Mutable::new(PrivacyData::default());
        let mut changes = data.signal_ref(|_| ()).to_stream();
        let mut wrote = || changes.next().now_or_never().is_some();
        assert!(wrote());

        track_node(&data, 7, Some(Media::Audio));
        assert!(wrote());
        track_node(&data, 7, Some(Media::Audio));
        assert!(!wrote());
        track_node(&data, 7, None);
        assert!(wrote());
        track_node(&data, 7, None);
        assert!(!wrote());
        assert!(data.lock_ref().nodes.is_empty());
    }
}
