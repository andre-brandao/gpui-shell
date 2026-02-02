//! NetworkManager Device Statistics D-Bus proxy.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Statistics",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait Statistics {
    /// RefreshRateMs property.
    #[zbus(property)]
    fn refresh_rate_ms(&self) -> zbus::Result<u32>;

    #[zbus(property)]
    fn set_refresh_rate_ms(&self, value: u32) -> zbus::Result<()>;

    /// RxBytes property.
    #[zbus(property)]
    fn rx_bytes(&self) -> zbus::Result<u64>;

    /// TxBytes property.
    #[zbus(property)]
    fn tx_bytes(&self) -> zbus::Result<u64>;
}
