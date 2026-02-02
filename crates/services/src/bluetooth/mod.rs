//! Bluetooth service for device management via BlueZ.
//!
//! This module provides a reactive subscriber for monitoring and controlling
//! Bluetooth adapters and devices using BlueZ D-Bus interface.

mod dbus;
mod types;

pub use types::*;

use dbus::BluetoothDbus;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
use inotify::{Inotify, WatchMask};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Event-driven Bluetooth subscriber.
///
/// This subscriber monitors Bluetooth adapter and device state via BlueZ D-Bus
/// and provides reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct BluetoothSubscriber {
    data: Mutable<BluetoothData>,
    conn: zbus::Connection,
}

impl BluetoothSubscriber {
    /// Create a new Bluetooth subscriber and start monitoring.
    pub async fn new() -> anyhow::Result<Self> {
        let conn = zbus::Connection::system().await?;
        let initial_data = fetch_bluetooth_data(&conn).await.unwrap_or_default();
        let data = Mutable::new(initial_data);

        // Start the D-Bus listener
        start_listener(data.clone(), conn.clone());

        Ok(Self { data, conn })
    }

    /// Get a signal that emits when Bluetooth state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<BluetoothData> {
        self.data.signal_cloned()
    }

    /// Get the current Bluetooth data snapshot.
    pub fn get(&self) -> BluetoothData {
        self.data.get_cloned()
    }

    /// Execute a Bluetooth command.
    pub async fn dispatch(&self, command: BluetoothCommand) -> anyhow::Result<()> {
        let bluetooth = BluetoothDbus::new(&self.conn).await?;

        match command {
            BluetoothCommand::Toggle => {
                let state = bluetooth.state().await?;
                match state {
                    BluetoothState::Active => {
                        debug!("Turning Bluetooth off");
                        bluetooth.set_powered(false).await?;
                    }
                    BluetoothState::Inactive => {
                        debug!("Turning Bluetooth on");
                        bluetooth.set_powered(true).await?;
                    }
                    BluetoothState::Unavailable => {
                        warn!("Cannot toggle Bluetooth: no adapter available");
                    }
                }
            }
            BluetoothCommand::StartDiscovery => {
                debug!("Starting Bluetooth discovery");
                bluetooth.start_discovery().await?;

                // Auto-stop discovery after 15 seconds
                let conn = self.conn.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    if let Ok(bt) = BluetoothDbus::new(&conn).await {
                        let _ = bt.stop_discovery().await;
                        debug!("Auto-stopped Bluetooth discovery after 15 seconds");
                    }
                });
            }
            BluetoothCommand::StopDiscovery => {
                debug!("Stopping Bluetooth discovery");
                bluetooth.stop_discovery().await?;
            }
            BluetoothCommand::PairDevice(device_path) => {
                debug!("Pairing Bluetooth device: {:?}", device_path);
                bluetooth.pair_device(&device_path).await?;
            }
            BluetoothCommand::ConnectDevice(device_path) => {
                debug!("Connecting to Bluetooth device: {:?}", device_path);
                bluetooth.connect_device(&device_path).await?;
            }
            BluetoothCommand::DisconnectDevice(device_path) => {
                debug!("Disconnecting Bluetooth device: {:?}", device_path);
                bluetooth.disconnect_device(&device_path).await?;
            }
            BluetoothCommand::RemoveDevice(device_path) => {
                debug!("Removing Bluetooth device: {:?}", device_path);
                bluetooth.remove_device(&device_path).await?;
            }
        }

        Ok(())
    }
}

/// Check if Bluetooth is soft-blocked by rfkill.
fn check_rfkill_soft_block() -> bool {
    Command::new("rfkill")
        .args(["list", "bluetooth"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.contains("Soft blocked: yes"))
        .unwrap_or(false)
}

/// Fetch the current Bluetooth data state.
async fn fetch_bluetooth_data(conn: &zbus::Connection) -> anyhow::Result<BluetoothData> {
    let bluetooth = BluetoothDbus::new(conn).await?;

    let state = bluetooth.state().await?;
    let rfkill_soft_block = check_rfkill_soft_block();

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

/// Start the D-Bus listener in a dedicated thread.
fn start_listener(data: Mutable<BluetoothData>, conn: zbus::Connection) {
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for Bluetooth listener");

        rt.block_on(async move {
            if let Err(e) = run_listener(data, conn).await {
                error!("Bluetooth listener error: {}", e);
            }
        });
    });
}

/// Run the Bluetooth service listener.
async fn run_listener(data: Mutable<BluetoothData>, conn: zbus::Connection) -> anyhow::Result<()> {
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
    let rfkill_rx = setup_rfkill_monitor();

    // Main event loop
    loop {
        let event_occurred = tokio::select! {
            Some(_) = interfaces_added.next() => {
                debug!("Bluetooth interfaces added");
                true
            }
            Some(_) = interfaces_removed.next() => {
                debug!("Bluetooth interfaces removed");
                true
            }
            Some(_) = async {
                match &mut powered_stream {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            } => {
                debug!("Bluetooth powered state changed");
                true
            }
            Some(_) = async {
                match &mut discovering_stream {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            } => {
                debug!("Bluetooth discovering state changed");
                true
            }
            _ = async {
                match &rfkill_rx {
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
                debug!("rfkill state changed");
                true
            }
        };

        if event_occurred {
            if let Ok(new_data) = fetch_bluetooth_data(&conn).await {
                *data.lock_mut() = new_data;
            }
        }
    }
}

/// Set up rfkill monitoring via inotify.
fn setup_rfkill_monitor() -> Option<mpsc::Receiver<()>> {
    let mut inotify = Inotify::init().ok()?;
    inotify
        .watches()
        .add("/dev/rfkill", WatchMask::MODIFY)
        .ok()?;

    let (tx, rx) = mpsc::sync_channel(1);

    thread::spawn(move || {
        let mut buffer = [0; 512];
        loop {
            match inotify.read_events_blocking(&mut buffer) {
                Ok(_events) => {
                    let _ = tx.try_send(());
                }
                Err(e) => {
                    error!("Error reading rfkill events: {}", e);
                    break;
                }
            }
        }
    });

    Some(rx)
}
