//! D-Bus proxy definitions for UPower and PowerProfiles services.
//!
//! Based on the UPower D-Bus specification and power-profiles-daemon.

use std::ops::Deref;

use zbus::zvariant::OwnedValue;
use zbus::{Connection, proxy};

/// Battery charging/discharging state from UPower.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, OwnedValue)]
#[repr(u32)]
pub enum BatteryState {
    #[default]
    Unknown = 0,
    Charging = 1,
    Discharging = 2,
    Empty = 3,
    FullyCharged = 4,
    PendingCharge = 5,
    PendingDischarge = 6,
}

/// Device type from UPower.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, OwnedValue)]
#[repr(u32)]
pub enum DeviceType {
    #[default]
    Unknown = 0,
    LinePower = 1,
    Battery = 2,
    Ups = 3,
    Monitor = 4,
    Mouse = 5,
    Keyboard = 6,
    Pda = 7,
    Phone = 8,
    MediaPlayer = 9,
    Tablet = 10,
    Computer = 11,
    GamingInput = 12,
    Pen = 13,
    Touchpad = 14,
    Modem = 15,
    Network = 16,
    Headset = 17,
    Speakers = 18,
    Headphones = 19,
    Video = 20,
    OtherAudio = 21,
    RemoteControl = 22,
    Printer = 23,
    Scanner = 24,
    Camera = 25,
    Wearable = 26,
    Toy = 27,
    BluetoothGeneric = 28,
}

/// Warning level from UPower.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, OwnedValue)]
#[repr(u32)]
pub enum WarningLevel {
    #[default]
    Unknown = 0,
    None = 1,
    Discharging = 2,
    Low = 3,
    Critical = 4,
    Action = 5,
}

/// Battery level from UPower (coarse-grained).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, OwnedValue)]
#[repr(u32)]
pub enum BatteryLevel {
    #[default]
    Unknown = 0,
    None = 1,
    Low = 3,
    Critical = 4,
    Normal = 6,
    High = 7,
    Full = 8,
}

/// UPower Device D-Bus proxy with all properties.
#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower",
    assume_defaults = false
)]
pub trait Device {
    /// Refresh the data collected from the power source.
    fn refresh(&self) -> zbus::Result<()>;

    /// Get history of a specific type.
    fn get_history(
        &self,
        type_: &str,
        timespan: u32,
        resolution: u32,
    ) -> zbus::Result<Vec<(u32, f64, u32)>>;

    /// Get statistics of a specific type.
    fn get_statistics(&self, type_: &str) -> zbus::Result<Vec<(f64, f64)>>;

    // Properties

    /// Type of power source.
    #[zbus(property, name = "Type")]
    fn device_type(&self) -> zbus::Result<DeviceType>;

    /// Native path of the power source.
    #[zbus(property)]
    fn native_path(&self) -> zbus::Result<String>;

    /// Vendor string.
    #[zbus(property)]
    fn vendor(&self) -> zbus::Result<String>;

    /// Model string.
    #[zbus(property)]
    fn model(&self) -> zbus::Result<String>;

    /// Serial number.
    #[zbus(property)]
    fn serial(&self) -> zbus::Result<String>;

    /// Update time (seconds since epoch).
    #[zbus(property)]
    fn update_time(&self) -> zbus::Result<u64>;

    /// Whether power is currently being provided.
    #[zbus(property)]
    fn power_supply(&self) -> zbus::Result<bool>;

    /// Whether the device has history.
    #[zbus(property)]
    fn has_history(&self) -> zbus::Result<bool>;

    /// Whether the device has statistics.
    #[zbus(property)]
    fn has_statistics(&self) -> zbus::Result<bool>;

    /// Whether the device is online (for line power).
    #[zbus(property)]
    fn online(&self) -> zbus::Result<bool>;

    /// Amount of energy available in Wh.
    #[zbus(property)]
    fn energy(&self) -> zbus::Result<f64>;

    /// Amount of energy when empty in Wh.
    #[zbus(property)]
    fn energy_empty(&self) -> zbus::Result<f64>;

    /// Amount of energy when full in Wh.
    #[zbus(property)]
    fn energy_full(&self) -> zbus::Result<f64>;

    /// Amount of energy when full by design in Wh.
    #[zbus(property)]
    fn energy_full_design(&self) -> zbus::Result<f64>;

    /// Energy rate in W.
    #[zbus(property)]
    fn energy_rate(&self) -> zbus::Result<f64>;

    /// Voltage in V.
    #[zbus(property)]
    fn voltage(&self) -> zbus::Result<f64>;

    /// Charge cycles count.
    #[zbus(property)]
    fn charge_cycles(&self) -> zbus::Result<i32>;

    /// Luminosity in Lux.
    #[zbus(property)]
    fn luminosity(&self) -> zbus::Result<f64>;

    /// Seconds until empty.
    #[zbus(property)]
    fn time_to_empty(&self) -> zbus::Result<i64>;

    /// Seconds until full.
    #[zbus(property)]
    fn time_to_full(&self) -> zbus::Result<i64>;

    /// Percentage charge.
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;

    /// Temperature in degrees Celsius.
    #[zbus(property)]
    fn temperature(&self) -> zbus::Result<f64>;

    /// Whether the device is present.
    #[zbus(property)]
    fn is_present(&self) -> zbus::Result<bool>;

    /// Current state of the device.
    #[zbus(property)]
    fn state(&self) -> zbus::Result<BatteryState>;

    /// Whether the device is rechargeable.
    #[zbus(property)]
    fn is_rechargeable(&self) -> zbus::Result<bool>;

    /// Capacity as a percentage (0-100, health indicator).
    #[zbus(property)]
    fn capacity(&self) -> zbus::Result<f64>;

    /// Technology of the battery.
    #[zbus(property)]
    fn technology(&self) -> zbus::Result<u32>;

    /// Warning level.
    #[zbus(property)]
    fn warning_level(&self) -> zbus::Result<WarningLevel>;

    /// Battery level (coarse).
    #[zbus(property)]
    fn battery_level(&self) -> zbus::Result<BatteryLevel>;

    /// Icon name.
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;
}

/// Main UPower D-Bus proxy.
#[proxy(interface = "org.freedesktop.UPower", assume_defaults = true)]
pub trait UPower {
    /// Enumerate all power devices.
    fn enumerate_devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// Get the display device (composite battery).
    #[zbus(object = "Device")]
    fn get_display_device(&self);

    /// Get the critical action setting.
    fn get_critical_action(&self) -> zbus::Result<String>;

    // Signals

    /// Device added signal.
    #[zbus(signal)]
    fn device_added(&self, device: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    /// Device removed signal.
    #[zbus(signal)]
    fn device_removed(&self, device: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    // Properties

    /// Daemon version.
    #[zbus(property)]
    fn daemon_version(&self) -> zbus::Result<String>;

    /// Whether running on battery.
    #[zbus(property)]
    fn on_battery(&self) -> zbus::Result<bool>;

    /// Whether a lid is present.
    #[zbus(property)]
    fn lid_is_present(&self) -> zbus::Result<bool>;

    /// Whether the lid is closed.
    #[zbus(property)]
    fn lid_is_closed(&self) -> zbus::Result<bool>;
}

/// Power Profiles Daemon D-Bus proxy.
#[proxy(
    interface = "net.hadess.PowerProfiles",
    default_service = "net.hadess.PowerProfiles",
    default_path = "/net/hadess/PowerProfiles"
)]
pub trait PowerProfiles {
    // Properties

    /// Currently active power profile.
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;

    /// Set the active power profile.
    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> zbus::Result<()>;

    /// List of available profiles with metadata.
    #[zbus(property)]
    fn profiles(
        &self,
    ) -> zbus::Result<Vec<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>;

    /// Actions currently holding a profile.
    #[zbus(property)]
    fn actions(
        &self,
    ) -> zbus::Result<Vec<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>;

    /// Reason for degraded performance (if any).
    #[zbus(property)]
    fn performance_degraded(&self) -> zbus::Result<String>;

    /// Whether performance is inhibited.
    #[zbus(property)]
    fn performance_inhibited(&self) -> zbus::Result<String>;

    /// Active profile holds.
    #[zbus(property)]
    fn active_profile_holds(
        &self,
    ) -> zbus::Result<Vec<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>;

    // Methods

    /// Hold a profile for a reason.
    fn hold_profile(&self, profile: &str, reason: &str, application_id: &str) -> zbus::Result<u32>;

    /// Release a held profile.
    fn release_profile(&self, cookie: u32) -> zbus::Result<()>;
}

/// Wrapper around UPowerProxy for convenience.
#[derive(Debug)]
pub struct UPowerService<'a>(UPowerProxy<'a>);

impl<'a> Deref for UPowerService<'a> {
    type Target = UPowerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> UPowerService<'a> {
    /// Create a new UPower service connection.
    pub async fn new(conn: &'a Connection) -> zbus::Result<Self> {
        UPowerProxy::new(conn).await.map(Self)
    }
}

/// Wrapper around PowerProfilesProxy for convenience.
#[derive(Debug)]
pub struct PowerProfilesService<'a>(PowerProfilesProxy<'a>);

impl<'a> Deref for PowerProfilesService<'a> {
    type Target = PowerProfilesProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> PowerProfilesService<'a> {
    /// Create a new PowerProfiles service connection.
    pub async fn new(conn: &'a Connection) -> zbus::Result<Self> {
        PowerProfilesProxy::new(conn).await.map(Self)
    }

    /// Check if the service is available.
    pub async fn is_available(conn: &Connection) -> bool {
        PowerProfilesProxy::new(conn).await.is_ok()
    }
}
