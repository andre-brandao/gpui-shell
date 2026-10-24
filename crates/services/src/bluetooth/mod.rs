//! Bluetooth service for device management via BlueZ.
//!
//! This module provides a reactive subscriber for monitoring and controlling
//! Bluetooth adapters and devices using BlueZ D-Bus interface.

mod dbus;
mod types;

pub use types::*;

use anyhow::Context as _;
use dbus::BluetoothDbus;
use futures_signals::signal::Mutable;
use futures_util::StreamExt;
use futures_util::stream::{BoxStream, select_all};
use inotify::{Inotify, WatchMask};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::task::AbortHandle;
use tracing::{debug, error, warn};

use crate::lifecycle::{Lifecycle, ManagedService, RunToken};

/// Event-driven Bluetooth subscriber.
///
/// This subscriber monitors Bluetooth adapter and device state via BlueZ D-Bus
/// and provides reactive state updates through `futures_signals`.
#[derive(Debug, Clone, Default)]
pub struct BluetoothSubscriber {
    core: crate::ServiceCore<BluetoothData, BluetoothCommand>,
    /// Stops a discovery after 15s. Replaced by every discovery command, so
    /// a timer from an earlier scan cannot cut a newer one short.
    discovery_timer: Arc<Mutex<Option<AbortHandle>>>,
}

impl std::ops::Deref for BluetoothSubscriber {
    type Target = crate::ServiceCore<BluetoothData, BluetoothCommand>;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl BluetoothSubscriber {
    /// Create a new Bluetooth subscriber. Monitoring starts with
    /// [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
    }

    async fn execute(&self, command: BluetoothCommand, token: &RunToken) -> anyhow::Result<()> {
        let conn = crate::bus::system().await?;
        let bluetooth = BluetoothDbus::new(&conn).await?;

        if matches!(
            command,
            BluetoothCommand::StartDiscovery | BluetoothCommand::StopDiscovery
        ) && let Some(timer) = self
            .discovery_timer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            timer.abort();
        }

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

                // Auto-stop discovery after 15 seconds, unless the run ends first.
                let token = token.clone();
                let timer = tokio::spawn(async move {
                    token
                        .run_until_cancelled(async {
                            tokio::time::sleep(Duration::from_secs(15)).await;
                            if let Ok(bt) = BluetoothDbus::new(&conn).await {
                                let _ = bt.stop_discovery().await;
                                debug!("Auto-stopped Bluetooth discovery after 15 seconds");
                            }
                        })
                        .await;
                });
                *self
                    .discovery_timer
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = Some(timer.abort_handle());
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

/// Whether any Bluetooth radio is soft-blocked, read from sysfs instead of
/// forking `rfkill` on every BlueZ signal.
fn check_rfkill_soft_block() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/rfkill") else {
        return false;
    };
    entries.flatten().any(|entry| {
        let read = |name| std::fs::read_to_string(entry.path().join(name)).unwrap_or_default();
        read("type").trim() == "bluetooth" && read("soft").trim() == "1"
    })
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

impl ManagedService for BluetoothSubscriber {
    fn name(&self) -> &'static str {
        "Bluetooth"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    fn memory_bytes(&self) -> usize {
        let data = self.data.lock_ref();
        size_of::<BluetoothData>()
            + data
                .devices
                .iter()
                .map(|device| {
                    size_of::<BluetoothDevice>() + device.name.len() + device.path.as_str().len()
                })
                .sum::<usize>()
    }

    fn start(&self) {
        let token = self.lifecycle.begin();
        let data = self.data.clone();

        let subscriber = self.clone();
        let jobs_token = token.clone();
        self.jobs.run(token.clone(), move |command| {
            let subscriber = subscriber.clone();
            let token = jobs_token.clone();
            async move { subscriber.execute(command, &token).await }
        });

        token.supervise(self.name(), move |token| run(data.clone(), token));
    }
}

/// One run: snapshot, then follow BlueZ until every stream ends.
async fn run(data: Mutable<BluetoothData>, token: RunToken) -> anyhow::Result<()> {
    let conn = crate::bus::system()
        .await
        .context("connecting to the system bus")?;
    let fetched = fetch_bluetooth_data(&conn)
        .await
        .context("fetching initial Bluetooth state")?;
    data.set_neq(fetched);
    token.active();
    run_listener(data, conn).await
}

/// Run the Bluetooth service listener.
///
/// Every event triggers a full refetch, so the streams are merged into one
/// stream of "something changed" ticks.
async fn run_listener(data: Mutable<BluetoothData>, conn: zbus::Connection) -> anyhow::Result<()> {
    let bluetooth = BluetoothDbus::new(&conn).await?;

    let mut changes: Vec<BoxStream<'_, &'static str>> = vec![
        bluetooth
            .bluez
            .receive_interfaces_added()
            .await?
            .map(|_| "interfaces added")
            .boxed(),
        bluetooth
            .bluez
            .receive_interfaces_removed()
            .await?
            .map(|_| "interfaces removed")
            .boxed(),
    ];

    if let Some(adapter) = &bluetooth.adapter {
        changes.push(
            adapter
                .receive_powered_changed()
                .await
                .map(|_| "powered state changed")
                .boxed(),
        );
        changes.push(
            adapter
                .receive_discovering_changed()
                .await
                .map(|_| "discovering state changed")
                .boxed(),
        );
    }

    if let Some(rfkill) = rfkill_changes() {
        changes.push(rfkill);
    }

    let mut changes = select_all(changes);
    while let Some(change) = changes.next().await {
        debug!("Bluetooth {change}");
        if let Ok(new_data) = fetch_bluetooth_data(&conn).await {
            data.set_neq(new_data);
        }
    }

    anyhow::bail!("Bluetooth event streams ended")
}

/// rfkill soft-block changes, watched with inotify on the tokio reactor so
/// the watch goes away with the run instead of parking a thread.
fn rfkill_changes() -> Option<BoxStream<'static, &'static str>> {
    let inotify = Inotify::init().ok()?;
    inotify
        .watches()
        .add("/dev/rfkill", WatchMask::MODIFY)
        .ok()?;

    let events = match inotify.into_event_stream([0; 512]) {
        Ok(events) => events,
        Err(e) => {
            error!("Error watching rfkill events: {}", e);
            return None;
        }
    };

    Some(
        events
            .take_while(|event| {
                if let Err(e) = event {
                    error!("Error reading rfkill events: {}", e);
                }
                std::future::ready(event.is_ok())
            })
            .map(|_| "rfkill state changed")
            .boxed(),
    )
}
