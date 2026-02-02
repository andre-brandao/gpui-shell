//! NetworkManager Settings D-Bus proxy.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Settings",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings"
)]
pub trait Settings {
    /// AddConnection method.
    fn add_connection(
        &self,
        connection: std::collections::HashMap<
            &str,
            std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
        >,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// GetConnectionByUuid method.
    fn get_connection_by_uuid(&self, uuid: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// ListConnections method.
    fn list_connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// ReloadConnections method.
    fn reload_connections(&self) -> zbus::Result<bool>;

    /// SaveHostname method.
    fn save_hostname(&self, hostname: &str) -> zbus::Result<()>;

    /// ConnectionRemoved signal.
    #[zbus(signal)]
    fn connection_removed(&self, connection: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    /// NewConnection signal.
    #[zbus(signal)]
    fn new_connection(&self, connection: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    /// CanModify property.
    #[zbus(property)]
    fn can_modify(&self) -> zbus::Result<bool>;

    /// Connections property.
    #[zbus(property)]
    fn connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// Hostname property.
    #[zbus(property)]
    fn hostname(&self) -> zbus::Result<String>;
}
