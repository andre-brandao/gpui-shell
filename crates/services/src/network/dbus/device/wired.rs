//! NetworkManager Wired Device D-Bus proxy.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wired",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait WiredDevice {
    /// Carrier property.
    #[zbus(property)]
    fn carrier(&self) -> zbus::Result<bool>;

    /// HwAddress property.
    #[zbus(property)]
    fn hw_address(&self) -> zbus::Result<String>;

    /// PermHwAddress property.
    #[zbus(property)]
    fn perm_hw_address(&self) -> zbus::Result<String>;

    /// Speed property.
    #[zbus(property)]
    fn speed(&self) -> zbus::Result<u32>;
}
