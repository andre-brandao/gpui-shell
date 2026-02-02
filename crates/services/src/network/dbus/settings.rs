//! NetworkManager Settings D-Bus proxy.

use std::collections::HashMap;
use zbus::proxy;
use zbus::zvariant::OwnedValue;

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

/// Proxy for a single saved connection profile.
#[proxy(
    interface = "org.freedesktop.NetworkManager.Settings.Connection",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait Connection {
    /// Delete this connection.
    fn delete(&self) -> zbus::Result<()>;

    /// Get the settings for this connection.
    /// Returns a nested HashMap: section -> key -> value
    fn get_settings(&self) -> zbus::Result<HashMap<String, HashMap<String, OwnedValue>>>;

    /// Get secrets for this connection (requires authorization).
    fn get_secrets(
        &self,
        setting_name: &str,
    ) -> zbus::Result<HashMap<String, HashMap<String, OwnedValue>>>;

    /// Update connection settings.
    fn update(
        &self,
        properties: HashMap<&str, HashMap<&str, zbus::zvariant::Value<'_>>>,
    ) -> zbus::Result<()>;

    /// Save changes to disk.
    fn save(&self) -> zbus::Result<()>;

    /// Removed signal - emitted when this connection is deleted.
    #[zbus(signal)]
    fn removed(&self) -> zbus::Result<()>;

    /// Updated signal - emitted when this connection is modified.
    #[zbus(signal)]
    fn updated(&self) -> zbus::Result<()>;

    /// Filename property - path to the connection file on disk.
    #[zbus(property)]
    fn filename(&self) -> zbus::Result<String>;

    /// Unsaved property - whether there are unsaved changes.
    #[zbus(property)]
    fn unsaved(&self) -> zbus::Result<bool>;
}
