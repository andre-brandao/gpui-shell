use crate::services::{ReadOnlyService, ServiceEvent};
use gpui::Context;
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, warn};
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";

/// Media type being accessed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    Video,
    Audio,
}

/// An application node accessing media.
#[derive(Debug, Clone)]
pub struct ApplicationNode {
    pub id: u32,
    pub media: Media,
}

/// Privacy-related data.
#[derive(Debug, Clone, Default)]
pub struct PrivacyData {
    pub nodes: Vec<ApplicationNode>,
    pub webcam_access: i32,
}

impl PrivacyData {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            webcam_access: is_device_in_use(WEBCAM_DEVICE_PATH),
        }
    }

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

/// Events from Privacy service.
#[derive(Debug, Clone)]
pub enum PrivacyEvent {
    AddNode(ApplicationNode),
    RemoveNode(u32),
    WebcamOpen,
    WebcamClose,
}

/// Privacy service for monitoring camera/mic/screenshare access.
#[derive(Debug, Clone, Default)]
pub struct Privacy {
    pub data: PrivacyData,
}

impl Deref for Privacy {
    type Target = PrivacyData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for Privacy {
    type UpdateEvent = PrivacyEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            PrivacyEvent::AddNode(node) => {
                self.data.nodes.push(node);
            }
            PrivacyEvent::RemoveNode(id) => {
                self.data.nodes.retain(|n| n.id != id);
            }
            PrivacyEvent::WebcamOpen => {
                self.data.webcam_access += 1;
                debug!("Webcam opened: {}", self.data.webcam_access);
            }
            PrivacyEvent::WebcamClose => {
                self.data.webcam_access = i32::max(self.data.webcam_access - 1, 0);
                debug!("Webcam closed: {}", self.data.webcam_access);
            }
        }
    }
}

impl Privacy {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<Privacy>>();

        // Spawn PipeWire listener thread
        let tx_pw = tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_pipewire_listener(tx_pw) {
                error!("PipeWire listener error: {}", e);
            }
        });

        // Spawn webcam watcher thread
        let tx_webcam = tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_webcam_watcher(tx_webcam) {
                warn!("Webcam watcher error: {}", e);
            }
        });

        // Poll the channel for updates
        cx.spawn(async move |this, cx| {
            loop {
                let mut events = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    events.push(event);
                }

                for event in events {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            match event {
                                ServiceEvent::Init(privacy) => {
                                    this.data = privacy.data;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    error!("Privacy service error: {}", e);
                                }
                            }
                            cx.notify();
                        })
                        .is_ok();

                    if !should_continue {
                        return;
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
            }
        })
        .detach();

        Privacy {
            data: PrivacyData::new(),
        }
    }
}

/// Run PipeWire listener to track media streams.
fn run_pipewire_listener(tx: mpsc::Sender<ServiceEvent<Privacy>>) -> anyhow::Result<()> {
    use pipewire::{context::Context, main_loop::MainLoop};

    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    let tx_add = tx.clone();
    let tx_remove = tx.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if let Some(props) = global.props {
                if let Some(media_class) = props.get("media.class") {
                    let is_video = media_class == "Stream/Input/Video";
                    let is_audio = media_class == "Stream/Input/Audio";

                    if is_video || is_audio {
                        debug!("New media node: {:?}", global);
                        let _ = tx_add.send(ServiceEvent::Update(PrivacyEvent::AddNode(
                            ApplicationNode {
                                id: global.id,
                                media: if is_video { Media::Video } else { Media::Audio },
                            },
                        )));
                    }
                }
            }
        })
        .global_remove(move |id| {
            debug!("Remove media node: {}", id);
            let _ = tx_remove.send(ServiceEvent::Update(PrivacyEvent::RemoveNode(id)));
        })
        .register();

    mainloop.run();

    Ok(())
}

/// Watch webcam device for open/close events.
fn run_webcam_watcher(tx: mpsc::Sender<ServiceEvent<Privacy>>) -> anyhow::Result<()> {
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
                let _ = tx.send(ServiceEvent::Update(PrivacyEvent::WebcamOpen));
            } else if event.mask.contains(EventMask::CLOSE_WRITE)
                || event.mask.contains(EventMask::CLOSE_NOWRITE)
            {
                let _ = tx.send(ServiceEvent::Update(PrivacyEvent::WebcamClose));
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
                    if let Ok(link_path) = fs::read_link(fd_entry.path()) {
                        if link_path == Path::new(target) {
                            used_by += 1;
                        }
                    }
                }
            }
        }
    }

    used_by
}
