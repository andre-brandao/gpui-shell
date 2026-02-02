use zbus::zvariant::OwnedObjectPath;

/// Network connectivity state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConnectivityState {
    #[default]
    Unknown,
    None,
    Portal,
    Limited,
    Full,
}

/// Wi-Fi access point information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    pub ssid: String,
    pub strength: u8,
    pub state: DeviceState,
    pub public: bool,
    pub working: bool,
    pub path: OwnedObjectPath,
    pub device_path: OwnedObjectPath,
}

/// Network device state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeviceState {
    #[default]
    Unknown,
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
}

/// VPN connection info.
#[derive(Debug, Clone)]
pub struct Vpn {
    pub name: String,
    pub path: OwnedObjectPath,
}

/// Known network connection.
#[derive(Debug, Clone)]
pub enum KnownConnection {
    AccessPoint(AccessPoint),
    Vpn(Vpn),
}

/// Active connection information.
#[derive(Debug, Clone)]
pub enum ActiveConnectionInfo {
    Wired {
        name: String,
    },
    WiFi {
        name: String,
        strength: u8,
    },
    Vpn {
        name: String,
        object_path: OwnedObjectPath,
    },
}

impl ActiveConnectionInfo {
    pub fn name(&self) -> &str {
        match self {
            Self::Wired { name, .. } => name,
            Self::WiFi { name, .. } => name,
            Self::Vpn { name, .. } => name,
        }
    }
}

/// Network state data.
#[derive(Debug, Clone, Default)]
pub struct NetworkData {
    pub wifi_present: bool,
    pub wireless_access_points: Vec<AccessPoint>,
    pub active_connections: Vec<ActiveConnectionInfo>,
    pub known_connections: Vec<KnownConnection>,
    pub wifi_enabled: bool,
    pub airplane_mode: bool,
    pub connectivity: ConnectivityState,
    pub scanning_nearby_wifi: bool,
}

/// Events from the network service.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    StateChanged(NetworkData),
}

/// Commands for the network service.
#[derive(Debug, Clone)]
pub enum NetworkCommand {
    ScanNearByWiFi,
    ToggleWiFi,
    ToggleAirplaneMode,
    SelectAccessPoint {
        ap: AccessPoint,
        password: Option<String>,
    },
    ToggleVpn(Vpn),
}
