//! NetworkManager Wireless Device D-Bus proxy.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait WirelessDevice {
    /// GetAccessPoints method.
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// GetAllAccessPoints method.
    fn get_all_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// RequestScan method.
    fn request_scan(
        &self,
        options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// AccessPointAdded signal.
    #[zbus(signal)]
    fn access_point_added(&self, access_point: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    /// AccessPointRemoved signal.
    #[zbus(signal)]
    fn access_point_removed(
        &self,
        access_point: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    /// AccessPoints property.
    #[zbus(property)]
    fn access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// ActiveAccessPoint property.
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Bitrate property.
    #[zbus(property)]
    fn bitrate(&self) -> zbus::Result<u32>;

    /// HwAddress property.
    #[zbus(property)]
    fn hw_address(&self) -> zbus::Result<String>;

    /// LastScan property.
    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;

    /// Mode property.
    #[zbus(property)]
    fn mode(&self) -> zbus::Result<u32>;

    /// WirelessCapabilities property.
    #[zbus(property)]
    fn wireless_capabilities(&self) -> zbus::Result<u32>;
}
