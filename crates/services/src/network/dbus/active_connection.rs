//! NetworkManager ActiveConnection D-Bus proxy.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait ActiveConnection {
    /// Connection property.
    #[zbus(property)]
    fn connection_(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Default property.
    #[zbus(property)]
    fn default(&self) -> zbus::Result<bool>;

    /// Default6 property.
    #[zbus(property)]
    fn default6(&self) -> zbus::Result<bool>;

    /// Devices property.
    #[zbus(property)]
    fn devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// Id property.
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// Master property.
    #[zbus(property)]
    fn master(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// SpecificObject property.
    #[zbus(property)]
    fn specific_object(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// State property.
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// StateFlags property.
    #[zbus(property)]
    fn state_flags(&self) -> zbus::Result<u32>;

    /// Type property.
    #[zbus(property)]
    fn type_(&self) -> zbus::Result<String>;

    /// Uuid property.
    #[zbus(property)]
    fn uuid(&self) -> zbus::Result<String>;

    /// Vpn property.
    #[zbus(property)]
    fn vpn(&self) -> zbus::Result<bool>;
}
