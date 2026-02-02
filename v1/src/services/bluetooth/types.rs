use zbus::zvariant::OwnedObjectPath;

/// Bluetooth adapter state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BluetoothState {
    /// No Bluetooth adapter available.
    #[default]
    Unavailable,
    /// Bluetooth is powered on and active.
    Active,
    /// Bluetooth adapter exists but is powered off.
    Inactive,
}

/// A Bluetooth device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    /// Device name (alias).
    pub name: String,
    /// Battery percentage if available.
    pub battery: Option<u8>,
    /// D-Bus object path for this device.
    pub path: OwnedObjectPath,
    /// Whether the device is currently connected.
    pub connected: bool,
    /// Whether the device is paired.
    pub paired: bool,
}

/// Bluetooth service data.
#[derive(Debug, Clone, Default)]
pub struct BluetoothData {
    /// Current Bluetooth adapter state.
    pub state: BluetoothState,
    /// List of known Bluetooth devices.
    pub devices: Vec<BluetoothDevice>,
    /// Whether device discovery is currently active.
    pub discovering: bool,
}

/// Events from the Bluetooth service.
#[derive(Debug, Clone)]
pub enum BluetoothEvent {
    /// Bluetooth state changed.
    StateChanged(BluetoothData),
}

/// Commands for the Bluetooth service.
#[derive(Debug, Clone)]
pub enum BluetoothCommand {
    /// Toggle Bluetooth power on/off.
    Toggle,
    /// Start scanning for nearby devices.
    StartDiscovery,
    /// Stop scanning for devices.
    StopDiscovery,
    /// Pair with a device.
    PairDevice(OwnedObjectPath),
    /// Connect to a paired device.
    ConnectDevice(OwnedObjectPath),
    /// Disconnect from a device.
    DisconnectDevice(OwnedObjectPath),
    /// Remove/unpair a device.
    RemoveDevice(OwnedObjectPath),
}
