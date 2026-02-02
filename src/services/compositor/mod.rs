pub mod hyprland;
pub mod niri;
pub mod types;

pub use self::types::{
    CompositorChoice, CompositorCommand, CompositorEvent, CompositorMonitor, CompositorState,
};

use crate::services::{ReadOnlyService, Service, ServiceEvent};
use gpui::Context;
use std::ops::Deref;
use std::sync::mpsc;

/// The compositor service state, holding workspace/monitor info.
#[derive(Debug, Clone)]
pub struct Compositor {
    pub state: CompositorState,
    pub backend: CompositorChoice,
}

impl Default for Compositor {
    fn default() -> Self {
        Self {
            state: CompositorState::default(),
            backend: detect_backend().unwrap_or(CompositorChoice::Hyprland),
        }
    }
}

impl Deref for Compositor {
    type Target = CompositorState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl ReadOnlyService for Compositor {
    type UpdateEvent = CompositorEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            CompositorEvent::StateChanged(new_state) => {
                self.state = new_state;
            }
            CompositorEvent::ActionPerformed => {}
        }
    }
}

impl Service for Compositor {
    type Command = CompositorCommand;

    async fn command(&mut self, command: Self::Command) -> Result<(), Self::Error> {
        execute_command(self.backend, command)
    }
}

impl Compositor {
    /// Create a new GPUI Entity for the compositor service.
    /// This spawns a background thread with a Tokio runtime that listens for
    /// compositor events and sends them via mpsc channel to GPUI.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<Compositor>>();

        // Spawn a dedicated thread with Tokio runtime for the compositor backend
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for compositor");

            rt.block_on(async move {
                let Some(backend) = detect_backend() else {
                    log::error!("No supported compositor backend found");
                    let _ = tx.send(ServiceEvent::Error(
                        "No supported compositor backend found".into(),
                    ));
                    return;
                };

                log::info!("Starting compositor event loop with {:?} backend", backend);

                let result = match backend {
                    CompositorChoice::Hyprland => hyprland::run_listener(&tx).await,
                    CompositorChoice::Niri => niri::run_listener(&tx).await,
                };

                if let Err(e) = result {
                    log::error!("Compositor event loop failed: {}", e);
                    let _ = tx.send(ServiceEvent::Error(e.to_string()));
                }
            });
        });

        // Poll the channel for updates from GPUI's async executor
        cx.spawn(async move |this, cx| {
            loop {
                // Drain all pending events, keep only the last state
                let mut last_event = None;
                while let Ok(event) = rx.try_recv() {
                    last_event = Some(event);
                }

                if let Some(event) = last_event {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            match event {
                                ServiceEvent::Init(compositor) => {
                                    this.state = compositor.state;
                                    this.backend = compositor.backend;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    log::error!("Compositor service error: {}", e);
                                }
                            }
                            cx.notify();
                        })
                        .is_ok();

                    if !should_continue {
                        log::debug!("Compositor entity dropped, stopping listener");
                        break;
                    }
                }

                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
            }
        })
        .detach();

        // Return initial state; it will be populated by events
        let backend = detect_backend().unwrap_or(CompositorChoice::Hyprland);
        Compositor {
            state: CompositorState::default(),
            backend,
        }
    }

    /// Execute a compositor command (e.g., switch workspace).
    pub fn dispatch(&self, command: CompositorCommand, _cx: &mut Context<Self>) {
        let backend = self.backend;
        // Spawn in a separate thread since we need Tokio for hyprland commands
        std::thread::spawn(move || {
            if let Err(e) = execute_command(backend, command) {
                log::error!("Failed to execute compositor command: {}", e);
            }
        });
    }
}

fn detect_backend() -> Option<CompositorChoice> {
    if hyprland::is_available() {
        Some(CompositorChoice::Hyprland)
    } else if niri::is_available() {
        Some(CompositorChoice::Niri)
    } else {
        None
    }
}

fn execute_command(backend: CompositorChoice, command: CompositorCommand) -> Result<(), String> {
    match backend {
        CompositorChoice::Hyprland => hyprland::execute_command_sync(command),
        CompositorChoice::Niri => niri::execute_command_sync(command),
    }
    .map_err(|e| e.to_string())
}
