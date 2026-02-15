//! NetworkManager helper for fetching network state.

use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use anyhow::Result;
use zbus::zvariant::OwnedObjectPath;

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

            // Request a scan (ignore errors)
            let _ = wireless_device.request_scan(HashMap::new()).await;

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
