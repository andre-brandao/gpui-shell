//! NetworkManager AccessPoint D-Bus proxy.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait AccessPoint {
    /// Flags property.
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u32>;

    /// Frequency property.
    #[zbus(property)]
    fn frequency(&self) -> zbus::Result<u32>;

    /// HwAddress property.
    #[zbus(property)]
    fn hw_address(&self) -> zbus::Result<String>;

    /// LastSeen property.
    #[zbus(property)]
    fn last_seen(&self) -> zbus::Result<i32>;

    /// MaxBitrate property.
    #[zbus(property)]
    fn max_bitrate(&self) -> zbus::Result<u32>;

    /// Mode property.
    #[zbus(property)]
    fn mode(&self) -> zbus::Result<u32>;

    /// RsnFlags property.
    #[zbus(property)]
    fn rsn_flags(&self) -> zbus::Result<u32>;

    /// Ssid property.
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;

    /// Strength property.
    #[zbus(property)]
    fn strength(&self) -> zbus::Result<u8>;

    /// WpaFlags property.
    #[zbus(property)]
    fn wpa_flags(&self) -> zbus::Result<u32>;
}
