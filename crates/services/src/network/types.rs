//! Network service types.

use zbus::zvariant::{ObjectPath, OwnedObjectPath};

/// Device type from NetworkManager.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Ethernet,
    Wifi,
    Bluetooth,
    TunTap,
    WireGuard,
    Generic,
    Other,
    #[default]
    Unknown,
}

impl From<u32> for DeviceType {
    fn from(device_type: u32) -> DeviceType {
        match device_type {
            1 => DeviceType::Ethernet,
            2 => DeviceType::Wifi,
            5 => DeviceType::Bluetooth,
            14 => DeviceType::Generic,
            16 => DeviceType::TunTap,
            29 => DeviceType::WireGuard,
            3..=32 => DeviceType::Other,
            _ => DeviceType::Unknown,
        }
    }
}

/// Device state from NetworkManager.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Unmanaged,
    Unavailable,
    Disconnected,
    Prepare,
    Config,
    NeedAuth,
    IpConfig,
    IpCheck,
    Secondaries,
    Activated,
    Deactivating,
    Failed,
    #[default]
    Unknown,
}

impl From<u32> for DeviceState {
    fn from(device_state: u32) -> Self {
        match device_state {
            10 => DeviceState::Unmanaged,
            20 => DeviceState::Unavailable,
            30 => DeviceState::Disconnected,
            40 => DeviceState::Prepare,
            50 => DeviceState::Config,
            60 => DeviceState::NeedAuth,
            70 => DeviceState::IpConfig,
            80 => DeviceState::IpCheck,
            90 => DeviceState::Secondaries,
            100 => DeviceState::Activated,
            110 => DeviceState::Deactivating,
            120 => DeviceState::Failed,
            _ => DeviceState::Unknown,
        }
    }
}

/// Connectivity state from NetworkManager.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityState {
    None,
    Portal,
    Loss,
    Full,
    #[default]
    Unknown,
}

impl From<u32> for ConnectivityState {
    fn from(state: u32) -> ConnectivityState {
        match state {
            1 => ConnectivityState::None,
            2 => ConnectivityState::Portal,
            3 => ConnectivityState::Loss,
            4 => ConnectivityState::Full,
            _ => ConnectivityState::Unknown,
        }
    }
}

/// A wireless access point.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccessPoint {
    /// SSID (network name).
    pub ssid: String,
    /// Signal strength (0-100).
    pub strength: u8,
    /// Device state for the AP.
    pub state: DeviceState,
    /// Whether the network is open (no security).
    pub public: bool,
    /// Whether a connection attempt is in progress.
    pub working: bool,
    /// Whether we have a saved connection profile for this network.
    pub known: bool,
    /// D-Bus object path for the access point.
    pub path: ObjectPath<'static>,
    /// D-Bus object path for the device.
    pub device_path: ObjectPath<'static>,
}

/// Information about an active network connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveConnectionInfo {
    Wired {
        name: String,
        speed: u32,
    },
    WiFi {
        id: String,
        name: String,
        strength: u8,
        device: String,
        object_path: OwnedObjectPath,
    },
    Vpn {
        name: String,
        object_path: OwnedObjectPath,
    },
}

impl ActiveConnectionInfo {
    /// Get the connection name.
    pub fn name(&self) -> &str {
        match self {
            Self::Wired { name, .. } => name,
            Self::WiFi { name, .. } => name,
            Self::Vpn { name, .. } => name,
        }
    }
}

/// Network traffic statistics for a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkStatistics {
    pub(crate) prev_tx: u64,
    pub(crate) prev_rx: u64,
    pub(crate) prev_tx_time: i64,
    pub(crate) prev_rx_time: i64,
    pub(crate) tx: u64,
    pub(crate) rx: u64,
    pub(crate) tx_time: i64,
    pub(crate) rx_time: i64,
    /// Device path.
    pub device: String,
}

impl NetworkStatistics {
    /// Calculate receive speed in bytes per second.
    pub fn rx_speed(&self) -> f64 {
        let elapsed = self.rx_time - self.prev_rx_time;
        if elapsed == 0 {
            0.0
        } else {
            (self.rx - self.prev_rx) as f64 / elapsed as f64
        }
    }

    /// Calculate transmit speed in bytes per second.
    pub fn tx_speed(&self) -> f64 {
        let elapsed = self.tx_time - self.prev_tx_time;
        if elapsed == 0 {
            0.0
        } else {
            (self.tx - self.prev_tx) as f64 / elapsed as f64
        }
    }
}

/// A saved VPN or WireGuard profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnProfile {
    /// Profile name (`connection.id`).
    pub name: String,
    /// Settings object path, what [`NetworkCommand::ConnectVpn`] takes.
    pub path: OwnedObjectPath,
    /// Whether the profile is currently active.
    pub active: bool,
}

/// The last WiFi connect the shell started, until it succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiAttempt {
    pub ssid: String,
    pub status: WifiAttemptStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiAttemptStatus {
    Connecting,
    /// NetworkManager had no password, or rejected the one given.
    NeedsPassword,
    Failed(String),
}

/// Network service data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkData {
    /// Whether WiFi is enabled.
    pub wifi_enabled: bool,
    /// Active network connections.
    pub active_connections: Vec<ActiveConnectionInfo>,
    /// Available wireless access points.
    pub wireless_access_points: Vec<AccessPoint>,
    /// Current connectivity state.
    pub connectivity: ConnectivityState,
    /// Network traffic statistics per device.
    pub network_statistics: Vec<NetworkStatistics>,
    /// A WiFi scan was requested and has not reported back yet.
    pub scanning: bool,
    /// Saved VPN profiles, by name.
    pub vpn_profiles: Vec<VpnProfile>,
    /// Outcome of the last connect started from the shell.
    pub wifi_attempt: Option<WifiAttempt>,
}

impl Default for NetworkData {
    fn default() -> Self {
        Self {
            wifi_enabled: false,
            active_connections: Vec::new(),
            wireless_access_points: Vec::new(),
            connectivity: ConnectivityState::Unknown,
            network_statistics: Vec::new(),
            scanning: false,
            vpn_profiles: Vec::new(),
            wifi_attempt: None,
        }
    }
}

impl NetworkData {
    /// Check if there's any active connection.
    pub fn is_connected(&self) -> bool {
        !self.active_connections.is_empty()
    }

    /// Get the primary connection info.
    pub fn primary_connection(&self) -> Option<&ActiveConnectionInfo> {
        self.active_connections.first()
    }
}

/// Commands for the Network service.
#[derive(Clone, PartialEq, Eq)]
pub enum NetworkCommand {
    /// Enable or disable WiFi.
    SetWifiEnabled(bool),
    /// Toggle WiFi on/off.
    ToggleWifi,
    /// Request a scan for wireless networks.
    RequestScan,
    /// Connect to a wireless network by SSID.
    Connect {
        ssid: String,
        password: Option<String>,
    },
    /// Disconnect the active WiFi connection by SSID.
    Disconnect(String),
    /// Delete every saved profile for this SSID.
    Forget(String),
    /// Forget the last connect's outcome, e.g. when its prompt closes.
    DismissWifiAttempt,
    /// Activate a saved VPN profile by its settings path.
    ConnectVpn(OwnedObjectPath),
    /// Deactivate a VPN profile by its settings path.
    DisconnectVpn(OwnedObjectPath),
}

// By hand so the job queue's command logs never print a password.
impl std::fmt::Debug for NetworkCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetWifiEnabled(enabled) => {
                f.debug_tuple("SetWifiEnabled").field(enabled).finish()
            }
            Self::ToggleWifi => f.write_str("ToggleWifi"),
            Self::RequestScan => f.write_str("RequestScan"),
            Self::Connect { ssid, password } => f
                .debug_struct("Connect")
                .field("ssid", ssid)
                .field("password", &password.as_ref().map(|_| "<redacted>"))
                .finish(),
            Self::Disconnect(ssid) => f.debug_tuple("Disconnect").field(ssid).finish(),
            Self::Forget(ssid) => f.debug_tuple("Forget").field(ssid).finish(),
            Self::DismissWifiAttempt => f.write_str("DismissWifiAttempt"),
            Self::ConnectVpn(path) => f.debug_tuple("ConnectVpn").field(path).finish(),
            Self::DisconnectVpn(path) => f.debug_tuple("DisconnectVpn").field(path).finish(),
        }
    }
}
