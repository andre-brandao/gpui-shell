//! NetworkManager WireGuard Device D-Bus proxy.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.WireGuard",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait WireGuardDevice {
    /// FwMark property.
    #[zbus(property)]
    fn fw_mark(&self) -> zbus::Result<u32>;

    /// ListenPort property.
    #[zbus(property)]
    fn listen_port(&self) -> zbus::Result<u16>;

    /// PublicKey property.
    #[zbus(property)]
    fn public_key(&self) -> zbus::Result<Vec<u8>>;
}
