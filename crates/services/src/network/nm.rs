//! NetworkManager helper for fetching network state.

use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use anyhow::Result;
use tracing::debug;
use zbus::zvariant::{OwnedObjectPath, Value};

use super::dbus::access_point::AccessPointProxy;
use super::dbus::active_connection::ActiveConnectionProxy;
use super::dbus::device::DeviceProxy;
use super::dbus::device::wired::WiredDeviceProxy;
use super::dbus::device::wireless::WirelessDeviceProxy;
use super::dbus::nm::NetworkManagerProxy;
use super::dbus::settings::{ConnectionProxy, SettingsProxy};
use super::dbus::statistics::StatisticsProxy;
use super::types::{AccessPoint, ActiveConnectionInfo, DeviceState, DeviceType, NetworkStatistics};

/// NetworkManager wrapper for fetching network state.
#[derive(Debug)]
pub struct NetworkManager<'a>(NetworkManagerProxy<'a>);

impl<'a> Deref for NetworkManager<'a> {
    type Target = NetworkManagerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> NetworkManager<'a> {
    /// Create a new NetworkManager wrapper.
    pub async fn new(connection: &'a zbus::Connection) -> zbus::Result<NetworkManager<'a>> {
        NetworkManagerProxy::new(connection).await.map(Self)
    }

    /// Get all active connections with their details.
    pub async fn active_connections(&self) -> Result<Vec<ActiveConnectionInfo>> {
        let active_connections = self.0.active_connections().await?;
        let mut ac_proxies = Vec::with_capacity(active_connections.len());

        for active_connection in active_connections {
            let proxy = ActiveConnectionProxy::builder(self.inner().connection())
                .path(active_connection)?
                .build()
                .await?;
            ac_proxies.push(proxy);
        }

        let mut info = Vec::with_capacity(ac_proxies.len());

        for connection in ac_proxies {
            for device in connection.devices().await.unwrap_or_default() {
                if connection.vpn().await.unwrap_or_default() {
                    info.push(ActiveConnectionInfo::Vpn {
                        name: connection.id().await?,
                        object_path: connection.inner().path().to_owned().into(),
                    });
                    continue;
                }

                let device_proxy = DeviceProxy::builder(self.inner().connection())
                    .path(device)?
                    .build()
                    .await?;

                match device_proxy.device_type().await.map(DeviceType::from).ok() {
                    Some(DeviceType::Ethernet) => {
                        let wired_device = WiredDeviceProxy::builder(self.inner().connection())
                            .path(device_proxy.inner().path())?
                            .build()
                            .await?;

                        info.push(ActiveConnectionInfo::Wired {
                            name: connection.id().await?,
                            speed: wired_device.speed().await?,
                        });
                    }
                    Some(DeviceType::Wifi) => {
                        let wireless_device =
                            WirelessDeviceProxy::builder(self.inner().connection())
                                .path(device_proxy.inner().path())?
                                .build()
                                .await?;

                        if let Ok(access_point) = wireless_device.active_access_point().await {
                            let ap_proxy = AccessPointProxy::builder(self.inner().connection())
                                .path(access_point)?
                                .build()
                                .await?;

                            info.push(ActiveConnectionInfo::WiFi {
                                id: connection.id().await?,
                                name: String::from_utf8_lossy(&ap_proxy.ssid().await?).into_owned(),
                                strength: ap_proxy.strength().await.unwrap_or_default(),
                                device: device_proxy.inner().path().to_string(),
                                object_path: connection.inner().path().to_owned().into(),
                            });
                        }
                    }
                    Some(DeviceType::WireGuard) => {
                        info.push(ActiveConnectionInfo::Vpn {
                            name: connection.id().await?,
                            object_path: connection.inner().path().to_owned().into(),
                        });
                    }
                    _ => {}
                }
            }
        }

        // Sort: VPN first, then Wired, then WiFi
        info.sort_by(|a, b| {
            let priority = |conn: &ActiveConnectionInfo| match conn {
                ActiveConnectionInfo::Vpn { name, .. } => format!("0{name}"),
                ActiveConnectionInfo::Wired { name, .. } => format!("1{name}"),
                ActiveConnectionInfo::WiFi { name, .. } => format!("2{name}"),
            };
            priority(a).cmp(&priority(b))
        });

        Ok(info)
    }

    /// Get all wireless device paths.
    pub async fn wireless_devices(&self) -> Result<Vec<OwnedObjectPath>> {
        let devices = self.devices().await?;
        let mut wireless_devices = Vec::new();

        for device in devices {
            let device_proxy = DeviceProxy::builder(self.inner().connection())
                .path(&device)?
                .build()
                .await?;

            if matches!(
                device_proxy.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                wireless_devices.push(device);
            }
        }

        Ok(wireless_devices)
    }

    /// Get network statistics for WiFi devices.
    pub async fn network_statistics(&self) -> Result<Vec<NetworkStatistics>> {
        let devices = self.devices().await?;
        let mut network_statistics = Vec::new();

        for device in devices {
            let device_proxy = DeviceProxy::builder(self.inner().connection())
                .path(&device)?
                .build()
                .await?;

            if matches!(
                device_proxy.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                let stats_proxy = StatisticsProxy::builder(self.inner().connection())
                    .path(&device)?
                    .build()
                    .await?;

                let tx = stats_proxy.tx_bytes().await?;
                let rx = stats_proxy.rx_bytes().await?;
                let timestamp = chrono::Utc::now().timestamp();

                network_statistics.push(NetworkStatistics {
                    prev_rx: rx,
                    prev_tx: tx,
                    prev_rx_time: timestamp,
                    prev_tx_time: timestamp,
                    tx,
                    rx,
                    rx_time: timestamp,
                    tx_time: timestamp,
                    device: device_proxy.inner().path().to_string(),
                });
            }
        }

        Ok(network_statistics)
    }

    /// Find a saved connection profile by SSID.
    ///
    /// Returns the D-Bus object path of the connection if found.
    pub async fn find_connection_by_ssid(&self, ssid: &str) -> Result<Option<OwnedObjectPath>> {
        let settings_proxy = SettingsProxy::new(self.inner().connection()).await?;
        let connections = settings_proxy.list_connections().await?;

        for conn_path in connections {
            let conn_proxy = ConnectionProxy::builder(self.inner().connection())
                .path(conn_path)?
                .build()
                .await?;

            if let Ok(settings) = conn_proxy.get_settings().await
                && let Some(conn_section) = settings.get("connection")
                && let Some(id_value) = conn_section.get("id")
                && let Ok(id) = <String>::try_from(id_value.clone())
                && id == ssid
            {
                return Ok(Some(conn_proxy.inner().path().to_owned().into()));
            }
        }

        Ok(None)
    }

    /// Connect to a network by SSID.
    ///
    /// For known networks, activates the existing connection profile.
    /// For new networks, finds the best AP and creates a new connection.
    pub async fn connect_by_ssid(&self, ssid: &str, password: Option<String>) -> Result<()> {
        let existing = self.find_connection_by_ssid(ssid).await?;

        // Find the AP and device for this SSID from current scan results
        let aps = self.wireless_access_points().await?;
        let ap = aps
            .iter()
            .find(|a| a.ssid == ssid)
            .ok_or_else(|| anyhow::anyhow!("Access point '{}' not found", ssid))?;

        let device = zbus::zvariant::ObjectPath::try_from(ap.device_path.as_str())?;

        if let Some(conn_path) = existing {
            debug!("Activating known connection for {}", ssid);
            let conn_obj = zbus::zvariant::ObjectPath::try_from(conn_path.as_str())?;
            let root = zbus::zvariant::ObjectPath::try_from("/")?;
            self.0
                .activate_connection(&conn_obj, &device, &root)
                .await?;
        } else {
            debug!("Creating new connection for {}", ssid);
            let ap_obj = zbus::zvariant::ObjectPath::try_from(ap.path.as_str())?;

            let mut settings: HashMap<&str, HashMap<&str, Value<'_>>> = HashMap::from([
                (
                    "802-11-wireless",
                    HashMap::from([("ssid", Value::Array(ssid.as_bytes().into()))]),
                ),
                (
                    "connection",
                    HashMap::from([
                        ("id", Value::Str(ssid.into())),
                        ("type", Value::Str("802-11-wireless".into())),
                    ]),
                ),
            ]);

            if let Some(ref password) = password {
                settings.insert(
                    "802-11-wireless-security",
                    HashMap::from([
                        ("psk", Value::Str(password.clone().into())),
                        ("key-mgmt", Value::Str("wpa-psk".into())),
                    ]),
                );
            }

            self.0
                .add_and_activate_connection(settings, &device, &ap_obj)
                .await?;
        }

        Ok(())
    }

    /// Disconnect an active WiFi connection by SSID.
    pub async fn disconnect_by_ssid(&self, ssid: &str) -> Result<()> {
        let active = self.active_connections().await?;
        let wifi_conn = active.iter().find_map(|c| {
            if let ActiveConnectionInfo::WiFi {
                name, object_path, ..
            } = c
            {
                if name == ssid {
                    Some(object_path.clone())
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(path) = wifi_conn {
            self.0
                .deactivate_connection(&zbus::zvariant::ObjectPath::try_from(path.as_str())?)
                .await?;
        } else {
            anyhow::bail!("No active WiFi connection for '{}'", ssid);
        }

        Ok(())
    }

    /// Get SSIDs of all known/saved WiFi connections.
    pub async fn known_wifi_ssids(&self) -> Result<HashSet<String>> {
        let settings_proxy = SettingsProxy::new(self.inner().connection()).await?;
        let connections = settings_proxy.list_connections().await?;
        let mut known_ssids = HashSet::new();

        for conn_path in connections {
            let conn_proxy = ConnectionProxy::builder(self.inner().connection())
                .path(conn_path)?
                .build()
                .await?;

            if let Ok(settings) = conn_proxy.get_settings().await {
                // Check if this is a WiFi connection
                if let Some(wifi_settings) = settings.get("802-11-wireless") {
                    // Get the SSID from the settings
                    if let Some(ssid_value) = wifi_settings.get("ssid") {
                        // SSID is stored as an array of bytes
                        if let Ok(ssid_bytes) = <Vec<u8>>::try_from(ssid_value.clone())
                            && let Ok(ssid) = String::from_utf8(ssid_bytes)
                        {
                            known_ssids.insert(ssid);
                        }
                    }
                }
            }
        }

        Ok(known_ssids)
    }

    /// Get all visible wireless access points.
    pub async fn wireless_access_points(&self) -> Result<Vec<AccessPoint>> {
        let wireless_devices = self.wireless_devices().await?;
        let known_ssids = self.known_wifi_ssids().await.unwrap_or_default();
        let mut all_access_points = Vec::new();

        for path in wireless_devices {
            let device_proxy = DeviceProxy::builder(self.inner().connection())
                .path(&path)?
                .build()
                .await?;

            let wireless_device = WirelessDeviceProxy::builder(self.inner().connection())
                .path(&path)?
                .build()
                .await?;

            // Pure read of NetworkManager's cached scan results. Requesting a
            // scan here caused a feedback loop: the LastScan-changed handler
            // calls this function, and a scan request completes by changing
            // LastScan again. Scans are triggered explicitly via
            // `NetworkCommand::RequestScan` instead.
            let access_points = wireless_device.get_access_points().await?;
            let state = device_proxy
                .cached_state()
                .unwrap_or_default()
                .map(DeviceState::from)
                .unwrap_or(DeviceState::Unknown);

            let mut aps = HashMap::<String, AccessPoint>::new();

            for ap in access_points {
                let ap_proxy = AccessPointProxy::builder(self.inner().connection())
                    .path(ap)?
                    .build()
                    .await?;

                let ssid = String::from_utf8_lossy(&ap_proxy.ssid().await?).into_owned();
                if ssid.is_empty() {
                    continue;
                }

                let public = ap_proxy.flags().await.unwrap_or_default() == 0;
                let strength = ap_proxy.strength().await?;

                // Keep the strongest signal for each SSID
                if let Some(existing) = aps.get(&ssid)
                    && existing.strength >= strength
                {
                    continue;
                }

                let known = known_ssids.contains(&ssid);

                aps.insert(
                    ssid.clone(),
                    AccessPoint {
                        ssid,
                        strength,
                        state,
                        public,
                        working: false,
                        known,
                        path: ap_proxy.inner().path().to_owned(),
                        device_path: device_proxy.inner().path().to_owned(),
                    },
                );
            }

            all_access_points.extend(aps.into_values());
        }

        // Sort by signal strength (strongest first)
        all_access_points.sort_by_key(|b| std::cmp::Reverse(b.strength));

        Ok(all_access_points)
    }
}
