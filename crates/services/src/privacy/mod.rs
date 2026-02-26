//! Privacy service for monitoring camera, microphone, and screenshare access.
//!
//! This module provides a reactive subscriber for tracking media stream access
//! via PipeWire and webcam device usage via inotify.

use std::fs;
use std::path::Path;
use std::thread;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use inotify::{EventMask, Inotify, WatchMask};
use tracing::{debug, error, warn};

use crate::ServiceStatus;

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
#[derive(Debug, Clone)]
pub struct ApplicationNode {
    /// PipeWire node ID.
    pub id: u32,
    /// Type of media being accessed.
    pub media: Media,
}

/// Privacy-related data.
#[derive(Debug, Clone, Default)]
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

    /// Get an icon representing the current privacy state.
    pub fn icon(&self) -> Option<&'static str> {
        if self.webcam_access() {
            Some("󰄀") // Camera
        } else if self.microphone_access() {
            Some("󰍬") // Microphone
        } else if self.screenshare_access() {
            Some("󰹑") // Screen
        } else {
            None
        }
    }
}

/// Event-driven privacy subscriber.
///
/// This subscriber monitors media access via PipeWire and webcam device
/// usage via inotify, providing reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct PrivacySubscriber {
    data: Mutable<PrivacyData>,
    pipewire_status: Mutable<ServiceStatus>,
    webcam_status: Mutable<ServiceStatus>,
}

impl PrivacySubscriber {
    /// Create a new privacy subscriber and start monitoring.
    pub fn new() -> Self {
        let initial_data = PrivacyData {
            nodes: Vec::new(),
            webcam_access: is_device_in_use(WEBCAM_DEVICE_PATH),
        };
        let data = Mutable::new(initial_data);
        let pipewire_status = Mutable::new(ServiceStatus::Active);
        let webcam_status = Mutable::new(ServiceStatus::Active);

        // Start PipeWire listener
        start_pipewire_listener(data.clone(), pipewire_status.clone());

        // Start webcam watcher
        start_webcam_watcher(data.clone(), webcam_status.clone());

        Self {
            data,
            pipewire_status,
            webcam_status,
        }
    }

    /// Get a signal that emits when privacy state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<PrivacyData> {
        self.data.signal_cloned()
    }

    /// Get the current privacy data snapshot.
    pub fn get(&self) -> PrivacyData {
        self.data.get_cloned()
    }

    /// Get the current service status.
    ///
    /// Merges PipeWire and webcam watcher statuses — an error in either
    /// subsystem is surfaced, so failures are not masked by the other.
    /// When both subsystems report errors, the PipeWire error takes priority.
    pub fn status(&self) -> ServiceStatus {
        let pw = self.pipewire_status.get_cloned();
        let wc = self.webcam_status.get_cloned();
        match (&pw, &wc) {
            (ServiceStatus::Error(e), _) => ServiceStatus::Error(e.clone()),
            (_, ServiceStatus::Error(e)) => ServiceStatus::Error(e.clone()),
            (ServiceStatus::Initializing, _) | (_, ServiceStatus::Initializing) => {
                ServiceStatus::Initializing
            }
            _ => ServiceStatus::Active,
        }
    }
}

impl Default for PrivacySubscriber {
    fn default() -> Self {
        Self::new()
    }
}

/// Start the PipeWire listener thread for media stream tracking.
fn start_pipewire_listener(data: Mutable<PrivacyData>, pipewire_status: Mutable<ServiceStatus>) {
    thread::spawn(move || {
        if let Err(e) = run_pipewire_listener(data) {
            error!("PipeWire listener error: {}", e);
            *pipewire_status.lock_mut() = ServiceStatus::Error(None);
        }
    });
}

/// Run the PipeWire listener (blocking).
fn run_pipewire_listener(data: Mutable<PrivacyData>) -> anyhow::Result<()> {
    use pipewire::{context::Context, main_loop::MainLoop};

    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    let data_add = data.clone();
    let data_remove = data.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
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

/// Start the webcam watcher thread.
fn start_webcam_watcher(data: Mutable<PrivacyData>, webcam_status: Mutable<ServiceStatus>) {
    thread::spawn(move || {
        if let Err(e) = run_webcam_watcher(data) {
            warn!("Webcam watcher error: {}", e);
            *webcam_status.lock_mut() = ServiceStatus::Error(None);
        }
    });
}

/// Run the webcam watcher (blocking).
fn run_webcam_watcher(data: Mutable<PrivacyData>) -> anyhow::Result<()> {
    // Check if webcam device exists
    if !Path::new(WEBCAM_DEVICE_PATH).exists() {
        warn!("Webcam device not found: {}", WEBCAM_DEVICE_PATH);
        return Ok(());
    }

    let mut inotify = Inotify::init()?;

    inotify.watches().add(
        WEBCAM_DEVICE_PATH,
        WatchMask::CLOSE_WRITE
            | WatchMask::CLOSE_NOWRITE
            | WatchMask::DELETE_SELF
            | WatchMask::OPEN
            | WatchMask::ATTRIB,
    )?;

    let mut buffer = [0; 1024];

    loop {
        let events = inotify.read_events_blocking(&mut buffer)?;

        for event in events {
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
    }
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
