mod dbus;
mod types;

pub use types::*;

use crate::services::{ReadOnlyService, ServiceEvent};
use gpui::Context;
use std::ops::Deref;
use std::sync::mpsc;

/// The network service state.
#[derive(Debug, Clone)]
pub struct Network {
    pub data: NetworkData,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            data: NetworkData::default(),
        }
    }
}

impl Deref for Network {
    type Target = NetworkData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for Network {
    type UpdateEvent = NetworkEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            NetworkEvent::StateChanged(data) => {
                self.data = data;
            }
        }
    }
}

impl Network {
    /// Create a new GPUI Entity for the network service.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<Network>>();

        // Spawn a dedicated thread with Tokio runtime for D-Bus operations
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for network service");

            rt.block_on(async move {
                if let Err(e) = dbus::run_listener(&tx).await {
                    log::error!("Network service failed: {}", e);
                    let _ = tx.send(ServiceEvent::Error(e.to_string()));
                }
            });
        });

        // Poll the channel for updates
        cx.spawn(async move |this, cx| {
            loop {
                let mut last_event = None;
                while let Ok(event) = rx.try_recv() {
                    last_event = Some(event);
                }

                if let Some(event) = last_event {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            match event {
                                ServiceEvent::Init(network) => {
                                    this.data = network.data;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    log::error!("Network service error: {}", e);
                                }
                            }
                            cx.notify();
                        })
                        .is_ok();

                    if !should_continue {
                        break;
                    }
                }

                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
            }
        })
        .detach();

        Network::default()
    }

    /// Execute a network command.
    pub fn dispatch(&self, command: NetworkCommand, _cx: &mut Context<Self>) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for network command");

            rt.block_on(async move {
                if let Err(e) = dbus::execute_command(command).await {
                    log::error!("Failed to execute network command: {}", e);
                }
            });
        });
    }
}
