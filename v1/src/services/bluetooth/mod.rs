mod dbus;
mod types;

pub use types::*;

use crate::services::{ReadOnlyService, ServiceEvent};
use dbus::BluetoothDbus;
use futures_lite::StreamExt;
use gpui::Context;
use std::ops::Deref;
use std::sync::mpsc;
use std::time::Duration;
use tokio::process::Command;

/// The Bluetooth service state.
#[derive(Debug, Clone)]
pub struct Bluetooth {
    pub data: BluetoothData,
}

impl Default for Bluetooth {
    fn default() -> Self {
        Self {
            data: BluetoothData::default(),
        }
    }
}

impl Deref for Bluetooth {
    type Target = BluetoothData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for Bluetooth {
    type UpdateEvent = BluetoothEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            BluetoothEvent::StateChanged(data) => {
                self.data = data;
            }
        }
    }
}

impl Bluetooth {
    /// Create a new GPUI Entity for the Bluetooth service.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<Bluetooth>>();

        // Spawn a dedicated thread with Tokio runtime for D-Bus operations
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for Bluetooth service");

            rt.block_on(async move {
                if let Err(e) = run_listener(&tx).await {
                    log::error!("Bluetooth service failed: {}", e);
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
                                ServiceEvent::Init(bluetooth) => {
                                    this.data = bluetooth.data;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    log::error!("Bluetooth service error: {}", e);
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
                    .timer(Duration::from_millis(100))
                    .await;
            }
        })
        .detach();

        Bluetooth::default()
    }

    /// Execute a Bluetooth command.
    pub fn dispatch(&self, command: BluetoothCommand, _cx: &mut Context<Self>) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for Bluetooth command");

            rt.block_on(async move {
                if let Err(e) = execute_command(command).await {
                    log::error!("Failed to execute Bluetooth command: {}", e);
                }
            });
        });
    }
}

/// Check if Bluetooth is soft-blocked by rfkill.
async fn check_rfkill_soft_block() -> anyhow::Result<bool> {
    let output = Command::new("rfkill")
        .arg("list")
        .arg("bluetooth")
        .output()
        .await?;

    let output = String::from_utf8(output.stdout)?;
    Ok(output.contains("Soft blocked: yes"))
}

/// Fetch the current Bluetooth data state.
async fn fetch_bluetooth_data(conn: &zbus::Connection) -> anyhow::Result<BluetoothData> {
    let bluetooth = BluetoothDbus::new(conn).await?;

    let state = bluetooth.state().await?;
    let rfkill_soft_block = check_rfkill_soft_block().await.unwrap_or(false);

    // Account for rfkill soft block
    let state = match state {
        BluetoothState::Unavailable => BluetoothState::Unavailable,
        BluetoothState::Active if rfkill_soft_block => BluetoothState::Inactive,
        state => state,
    };

    let devices = bluetooth.devices().await?;
    let discovering = bluetooth.discovering().await.unwrap_or(false);

    Ok(BluetoothData {
        state,
        devices,
        discovering,
    })
}

/// Run the Bluetooth service listener.
async fn run_listener(tx: &mpsc::Sender<ServiceEvent<Bluetooth>>) -> anyhow::Result<()> {
    let conn = zbus::Connection::system().await?;

    // Send initial state
    let data = fetch_bluetooth_data(&conn).await?;
    let _ = tx.send(ServiceEvent::Update(BluetoothEvent::StateChanged(data)));

    let bluetooth = BluetoothDbus::new(&conn).await?;

    // Set up streams for interface changes
    let mut interfaces_added = bluetooth.bluez.receive_interfaces_added().await?;
    let mut interfaces_removed = bluetooth.bluez.receive_interfaces_removed().await?;

    // Set up adapter-specific streams if adapter exists
    let (mut powered_stream, mut discovering_stream) = if let Some(adapter) = &bluetooth.adapter {
        (
            Some(adapter.receive_powered_changed().await),
            Some(adapter.receive_discovering_changed().await),
        )
    } else {
        (None, None)
    };

    // Set up rfkill monitoring using inotify
    let rfkill_stream = setup_rfkill_monitor();

    // Main event loop
    loop {
        let event_occurred = tokio::select! {
            Some(_) = interfaces_added.next() => {
                log::debug!("Bluetooth interfaces added");
                true
            }
            Some(_) = interfaces_removed.next() => {
                log::debug!("Bluetooth interfaces removed");
                true
            }
            Some(_) = async {
                match &mut powered_stream {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            } => {
                log::debug!("Bluetooth powered state changed");
                true
            }
            Some(_) = async {
                match &mut discovering_stream {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            } => {
                log::debug!("Bluetooth discovering state changed");
                true
            }
            _ = async {
                match &rfkill_stream {
                    Some(rx) => {
                        // Poll periodically since std mpsc isn't async
                        loop {
                            if rx.try_recv().is_ok() {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                    None => std::future::pending::<()>().await,
                }
            } => {
                log::debug!("rfkill state changed");
                true
            }
        };

        if event_occurred {
            if let Ok(data) = fetch_bluetooth_data(&conn).await {
                let _ = tx.send(ServiceEvent::Update(BluetoothEvent::StateChanged(data)));
            }
        }
    }
}

/// Set up rfkill monitoring via inotify.
fn setup_rfkill_monitor() -> Option<mpsc::Receiver<()>> {
    use inotify::{Inotify, WatchMask};

    let mut inotify = Inotify::init().ok()?;
    inotify
        .watches()
        .add("/dev/rfkill", WatchMask::MODIFY)
        .ok()?;

    let (tx, rx) = mpsc::sync_channel(1);

    std::thread::spawn(move || {
        let mut buffer = [0; 512];
        loop {
            match inotify.read_events_blocking(&mut buffer) {
                Ok(_events) => {
                    let _ = tx.try_send(());
                }
                Err(e) => {
                    log::error!("Error reading rfkill events: {}", e);
                    break;
                }
            }
        }
    });

    Some(rx)
}

/// Execute a Bluetooth command.
async fn execute_command(command: BluetoothCommand) -> anyhow::Result<()> {
    let conn = zbus::Connection::system().await?;
    let bluetooth = BluetoothDbus::new(&conn).await?;

    match command {
        BluetoothCommand::Toggle => {
            let state = bluetooth.state().await?;
            match state {
                BluetoothState::Active => {
                    log::debug!("Turning Bluetooth off");
                    bluetooth.set_powered(false).await?;
                }
                BluetoothState::Inactive => {
                    log::debug!("Turning Bluetooth on");
                    bluetooth.set_powered(true).await?;
                }
                BluetoothState::Unavailable => {
                    log::warn!("Cannot toggle Bluetooth: no adapter available");
                }
            }
        }
        BluetoothCommand::StartDiscovery => {
            log::debug!("Starting Bluetooth discovery");
            bluetooth.start_discovery().await?;

            // Auto-stop discovery after 15 seconds
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(15)).await;
                if let Ok(conn) = zbus::Connection::system().await {
                    if let Ok(bt) = BluetoothDbus::new(&conn).await {
                        let _ = bt.stop_discovery().await;
                        log::debug!("Auto-stopped Bluetooth discovery after 15 seconds");
                    }
                }
            });
        }
        BluetoothCommand::StopDiscovery => {
            log::debug!("Stopping Bluetooth discovery");
            bluetooth.stop_discovery().await?;
        }
        BluetoothCommand::PairDevice(device_path) => {
            log::debug!("Pairing Bluetooth device: {:?}", device_path);
            bluetooth.pair_device(&device_path).await?;
        }
        BluetoothCommand::ConnectDevice(device_path) => {
            log::debug!("Connecting to Bluetooth device: {:?}", device_path);
            bluetooth.connect_device(&device_path).await?;
        }
        BluetoothCommand::DisconnectDevice(device_path) => {
            log::debug!("Disconnecting Bluetooth device: {:?}", device_path);
            bluetooth.disconnect_device(&device_path).await?;
        }
        BluetoothCommand::RemoveDevice(device_path) => {
            log::debug!("Removing Bluetooth device: {:?}", device_path);
            bluetooth.remove_device(&device_path).await?;
        }
    }

    Ok(())
}
