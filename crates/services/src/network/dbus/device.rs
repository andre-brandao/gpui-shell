//! NetworkManager Device D-Bus proxies.

pub mod wired;
pub mod wireguard;
pub mod wireless;

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait Device {
    /// Delete method.
    fn delete(&self) -> zbus::Result<()>;

    /// Disconnect method.
    fn disconnect(&self) -> zbus::Result<()>;

    /// ActiveConnection property.
    #[zbus(property)]
    fn active_connection(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Autoconnect property.
    #[zbus(property)]
    fn autoconnect(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_autoconnect(&self, value: bool) -> zbus::Result<()>;

    /// AvailableConnections property.
    #[zbus(property)]
    fn available_connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// DeviceType property.
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;

    /// Driver property.
    #[zbus(property)]
    fn driver(&self) -> zbus::Result<String>;

    /// HwAddress property.
    #[zbus(property)]
    fn hw_address(&self) -> zbus::Result<String>;

    /// Interface property.
    #[zbus(property)]
    fn interface(&self) -> zbus::Result<String>;

    /// Managed property.
    #[zbus(property)]
    fn managed(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_managed(&self, value: bool) -> zbus::Result<()>;

    /// Mtu property.
    #[zbus(property)]
    fn mtu(&self) -> zbus::Result<u32>;

    /// State property.
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// StateReason property.
    #[zbus(property)]
    fn state_reason(&self) -> zbus::Result<(u32, u32)>;
}
