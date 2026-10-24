//! Privacy service for monitoring camera, microphone, and screenshare access.
//!
//! This module provides a reactive subscriber for tracking media stream access
//! via PipeWire and webcam device usage via inotify.

use std::fs;
use std::path::Path;
use std::thread;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
use inotify::{EventMask, Inotify, WatchMask};
use tracing::{debug, error, warn};

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
        self.data.lock_mut().webcam_access = is_device_in_use(WEBCAM_DEVICE_PATH);
        start_pipewire_listener(self.data.clone(), token.clone());
        start_webcam_watcher(self.data.clone(), token.clone());
        token.active();
    }
}

/// Start the PipeWire listener thread for media stream tracking.
///
/// The PipeWire loop is not async, so it keeps its own thread; a message on a
/// loop-attached channel quits it when the run ends.
fn start_pipewire_listener(data: Mutable<PrivacyData>, token: RunToken) {
    let (quit_tx, quit_rx) = pipewire::channel::channel::<()>();
    tokio::spawn({
        let token = token.clone();
        async move {
            token.cancelled().await;
            let _ = quit_tx.send(());
        }
    });

    thread::spawn(move || {
        if let Err(e) = run_pipewire_listener(data, token.clone(), quit_rx) {
            error!("PipeWire listener error: {}", e);
            token.error(e.to_string());
        }
    });
}

/// Run the PipeWire listener (blocking).
fn run_pipewire_listener(
    data: Mutable<PrivacyData>,
    token: RunToken,
    quit: pipewire::channel::Receiver<()>,
) -> anyhow::Result<()> {
    use pipewire::{context::ContextBox, main_loop::MainLoopRc};

    let mainloop = MainLoopRc::new(None)?;
    let _quit = quit.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |()| mainloop.quit()
    });
    let context = ContextBox::new(mainloop.loop_(), None)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    let data_add = data.clone();
    let data_remove = data.clone();
    let token_add = token.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if !token_add.alive() {
                return;
            }
            if let Some(props) = global.props
                && let Some(media_class) = props.get("media.class")
            {
                let is_video = media_class == "Stream/Input/Video";
                let is_audio = media_class == "Stream/Input/Audio";

                if is_video || is_audio {
                    debug!("New media node: id={}, class={}", global.id, media_class);
                    let node = ApplicationNode {
                        id: global.id,
                        media: if is_video { Media::Video } else { Media::Audio },
                    };
                    data_add.lock_mut().nodes.push(node);
                }
            }
        })
        .global_remove(move |id| {
            if !token.alive() {
                return;
            }
            let mut guard = data_remove.lock_mut();
            let before_len = guard.nodes.len();
            guard.nodes.retain(|n| n.id != id);
            // Only log if we actually removed a tracked media node
            if guard.nodes.len() < before_len {
                debug!("Removed tracked media node: {}", id);
            }
        })
        .register();

    mainloop.run();

    Ok(())
}

/// Start the webcam watcher task.
fn start_webcam_watcher(data: Mutable<PrivacyData>, token: RunToken) {
    tokio::spawn(async move {
        if let Some(Err(e)) = token.run_until_cancelled(run_webcam_watcher(data)).await {
            warn!("Webcam watcher error: {}", e);
            token.error(e.to_string());
        }
    });
}

/// Run the webcam watcher. The inotify fd sits on the tokio reactor, so a
/// stop drops the watch at once instead of at the next device event.
async fn run_webcam_watcher(data: Mutable<PrivacyData>) -> anyhow::Result<()> {
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

    Ok(())
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
