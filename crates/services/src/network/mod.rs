//! Network service for NetworkManager integration.
//!
//! This module provides a reactive subscriber for monitoring and controlling
//! network connections via NetworkManager D-Bus interface.

mod dbus;
mod nm;
mod types;

pub use types::*;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
use futures_util::stream::select_all;
use std::thread;
use tracing::{debug, error, info};
use zbus::Connection;

use self::dbus::access_point::AccessPointProxy;
use self::dbus::statistics::StatisticsProxy;
use self::nm::NetworkManager;

/// Event-driven network subscriber.
///
/// This subscriber monitors network state via NetworkManager D-Bus
/// and provides reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct NetworkSubscriber {
    data: Mutable<NetworkData>,
    conn: Connection,
}

impl NetworkSubscriber {
    /// Create a new network subscriber and start monitoring.
    pub async fn new() -> anyhow::Result<Self> {
        let conn = Connection::system().await?;
        let initial_data = fetch_network_data(&conn).await.unwrap_or_default();
        let data = Mutable::new(initial_data);

        // Start the D-Bus listener
        start_listener(data.clone(), conn.clone());

        Ok(Self { data, conn })
    }

    /// Get a signal that emits when network state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<NetworkData> {
        self.data.signal_cloned()
    }

    /// Get the current network data snapshot.
    pub fn get(&self) -> NetworkData {
        self.data.get_cloned()
    }

    /// Execute a network command.
    pub async fn dispatch(&self, command: NetworkCommand) -> anyhow::Result<()> {
        let nm = NetworkManager::new(&self.conn).await?;

        match command {
            NetworkCommand::SetWifiEnabled(enabled) => {
                debug!("Setting WiFi enabled: {}", enabled);
                nm.set_wireless_enabled(enabled).await?;
            }
            NetworkCommand::ToggleWifi => {
                let current = nm.wireless_enabled().await?;
                debug!("Toggling WiFi: {} -> {}", current, !current);
                nm.set_wireless_enabled(!current).await?;
            }
            NetworkCommand::RequestScan => {
                debug!("Requesting WiFi scan");
                let wireless_devices = nm.wireless_devices().await?;
                for device in wireless_devices {
                    let wireless = dbus::device::wireless::WirelessDeviceProxy::builder(&self.conn)
                        .path(&device)?
                        .build()
                        .await?;
                    let _ = wireless
                        .request_scan(std::collections::HashMap::new())
                        .await;
                }
            }
            NetworkCommand::ConnectToAccessPoint {
                device_path,
                ap_path,
                password,
            } => {
                debug!("Connecting to access point: {:?}", ap_path);
                let device = zbus::zvariant::ObjectPath::try_from(device_path.as_str())?;
                let ap = zbus::zvariant::ObjectPath::try_from(ap_path.as_str())?;

                if let Some(ref password) = password {
                    let mut settings = std::collections::HashMap::new();
                    let mut wifi_security = std::collections::HashMap::new();
                    wifi_security.insert("key-mgmt", zbus::zvariant::Value::from("wpa-psk"));
                    wifi_security.insert("psk", zbus::zvariant::Value::from(password.as_str()));
                    settings.insert("802-11-wireless-security", wifi_security);
                    nm.add_and_activate_connection(settings, &device, &ap)
                        .await?;
                } else {
                    nm.add_and_activate_connection(std::collections::HashMap::new(), &device, &ap)
                        .await?;
                };
            }
            NetworkCommand::Disconnect(connection_path) => {
                debug!("Disconnecting: {:?}", connection_path);
                nm.deactivate_connection(&zbus::zvariant::ObjectPath::try_from(
                    connection_path.as_str(),
                )?)
                .await?;
            }
        }

        Ok(())
    }
}

/// Fetch current network data from NetworkManager.
async fn fetch_network_data(conn: &Connection) -> anyhow::Result<NetworkData> {
    let nm = NetworkManager::new(conn).await?;

    let wifi_enabled = nm.wireless_enabled().await?;
    let connectivity = nm.connectivity().await?.into();
    let active_connections = nm.active_connections().await?;
    let wireless_access_points = nm.wireless_access_points().await?;
    let network_statistics = nm.network_statistics().await?;

    Ok(NetworkData {
        wifi_enabled,
        connectivity,
        active_connections,
        wireless_access_points,
        network_statistics,
    })
}

/// Start the D-Bus listener in a dedicated thread.
fn start_listener(data: Mutable<NetworkData>, conn: Connection) {
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for Network listener");

        rt.block_on(async move {
            if let Err(e) = run_listener(data, conn).await {
                error!("Network listener error: {}", e);
            }
        });
    });
}

/// Run the network listener event loop.
async fn run_listener(data: Mutable<NetworkData>, conn: Connection) -> anyhow::Result<()> {
    info!("Network subscriber started");

    let nm = NetworkManager::new(&conn).await?;

    // Stream for wireless enabled changes
    let data_wifi = data.clone();
    let wireless_enabled = nm
        .receive_wireless_enabled_changed()
        .await
        .then(move |v| {
            let data = data_wifi.clone();
            async move {
                if let Ok(value) = v.get().await {
                    data.lock_mut().wifi_enabled = value;
                    debug!("WiFi enabled changed: {}", value);
                }
            }
        })
        .boxed();

    // Stream for connectivity changes
    let data_conn = data.clone();
    let connectivity_changed = nm
        .receive_connectivity_changed()
        .await
        .then(move |val| {
            let data = data_conn.clone();
            async move {
                if let Ok(value) = val.get().await {
                    data.lock_mut().connectivity = value.into();
                    debug!("Connectivity changed: {:?}", value);
                }
            }
        })
        .boxed();

    // Stream for active connections changes
    let data_ac = data.clone();
    let conn_ac = conn.clone();
    let active_connections = nm
        .receive_active_connections_changed()
        .await
        .then(move |_| {
            let data = data_ac.clone();
            let conn = conn_ac.clone();
            async move {
                if let Ok(nm) = NetworkManager::new(&conn).await
                    && let Ok(connections) = nm.active_connections().await
                {
                    data.lock_mut().active_connections = connections;
                    debug!("Active connections changed");
                }
            }
        })
        .boxed();

    // Set up streams for access point strength changes
    let wireless_aps = nm.wireless_access_points().await?;
    let mut strength_changes = Vec::with_capacity(wireless_aps.len());

    for ap in &wireless_aps {
        let ssid = ap.ssid.clone();
        let data_ap = data.clone();

        if let Ok(ap_proxy) = AccessPointProxy::builder(&conn)
            .path(ap.path.clone())?
            .build()
            .await
        {
            strength_changes.push(
                ap_proxy
                    .receive_strength_changed()
                    .await
                    .then(move |val| {
                        let ssid = ssid.clone();
                        let data = data_ap.clone();
                        async move {
                            if let Ok(value) = val.get().await {
                                let mut guard = data.lock_mut();

                                // Update in access points
                                if let Some(ap) = guard
                                    .wireless_access_points
                                    .iter_mut()
                                    .find(|ap| ap.ssid == ssid)
                                {
                                    ap.strength = value;
                                }

                                // Update in active connections
                                if let Some(ActiveConnectionInfo::WiFi { strength, .. }) = guard
                                    .active_connections
                                    .iter_mut()
                                    .find(|ac| ac.name() == ssid)
                                {
                                    *strength = value;
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }
    }

    // Set up streams for network statistics
    let devices = nm.devices().await?;
    let mut statistics_changes = Vec::new();

    for device in devices {
        let device_string = device.to_string();

        if let Ok(stats_proxy) = StatisticsProxy::builder(&conn)
            .path(device.clone())?
            .build()
            .await
        {
            // Set refresh rate
            let _ = stats_proxy.set_refresh_rate_ms(1000).await;

            // RX bytes stream
            let device_rx = device_string.clone();
            let data_rx = data.clone();
            statistics_changes.push(
                stats_proxy
                    .receive_rx_bytes_changed()
                    .await
                    .then(move |val| {
                        let device_str = device_rx.clone();
                        let data = data_rx.clone();
                        async move {
                            if let Ok(value) = val.get().await {
                                let mut guard = data.lock_mut();
                                for stat in guard.network_statistics.iter_mut() {
                                    if stat.device == device_str {
                                        stat.prev_rx = stat.rx;
                                        stat.prev_rx_time = stat.rx_time;
                                        stat.rx = value;
                                        stat.rx_time = chrono::Utc::now().timestamp();
                                    }
                                }
                            }
                        }
                    })
                    .boxed(),
            );

            // TX bytes stream
            let device_tx = device_string.clone();
            let data_tx = data.clone();
            statistics_changes.push(
                stats_proxy
                    .receive_tx_bytes_changed()
                    .await
                    .then(move |val| {
                        let device_str = device_tx.clone();
                        let data = data_tx.clone();
                        async move {
                            if let Ok(value) = val.get().await {
                                let mut guard = data.lock_mut();
                                for stat in guard.network_statistics.iter_mut() {
                                    if stat.device == device_str {
                                        stat.prev_tx = stat.tx;
                                        stat.prev_tx_time = stat.tx_time;
                                        stat.tx = value;
                                        stat.tx_time = chrono::Utc::now().timestamp();
                                    }
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }
    }

    // Combine all streams
    let mut events = select_all(vec![
        wireless_enabled,
        connectivity_changed,
        active_connections,
    ]);

    for stream in strength_changes {
        events.push(stream);
    }

    for stream in statistics_changes {
        events.push(stream);
    }

    // Process events
    while (events.next().await).is_some() {}

    Ok(())
}
