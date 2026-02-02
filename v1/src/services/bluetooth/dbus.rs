use super::types::{BluetoothDevice, BluetoothState};
use anyhow::Result;
use std::collections::HashMap;
use zbus::{proxy, zvariant::OwnedObjectPath, zvariant::OwnedValue};

type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

/// Helper struct for interacting with BlueZ over D-Bus.
pub struct BluetoothDbus<'a> {
    pub bluez: BluezObjectManagerProxy<'a>,
    pub adapter: Option<AdapterProxy<'a>>,
}

impl BluetoothDbus<'_> {
    /// Create a new BluetoothDbus instance.
    pub async fn new(conn: &zbus::Connection) -> Result<BluetoothDbus<'_>> {
        let bluez = BluezObjectManagerProxy::new(conn).await?;

        // Find the first Bluetooth adapter
        let adapter_path = bluez
            .get_managed_objects()
            .await?
            .into_iter()
            .filter_map(|(key, item)| {
                if item.contains_key("org.bluez.Adapter1") {
                    Some(key)
                } else {
                    None
                }
            })
            .next();

        let adapter = if let Some(adapter_path) = adapter_path {
            Some(
                AdapterProxy::builder(conn)
                    .path(adapter_path)?
                    .build()
                    .await?,
            )
        } else {
            None
        };

        Ok(BluetoothDbus { bluez, adapter })
    }

    /// Set the Bluetooth adapter power state.
    pub async fn set_powered(&self, value: bool) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.set_powered(value).await?;
        }
        Ok(())
    }

    /// Get the current Bluetooth state.
    pub async fn state(&self) -> zbus::Result<BluetoothState> {
        match &self.adapter {
            Some(adapter) => {
                if adapter.powered().await? {
                    Ok(BluetoothState::Active)
                } else {
                    Ok(BluetoothState::Inactive)
                }
            }
            None => Ok(BluetoothState::Unavailable),
        }
    }

    /// Start device discovery.
    pub async fn start_discovery(&self) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.start_discovery().await?;
        }
        Ok(())
    }

    /// Stop device discovery.
    pub async fn stop_discovery(&self) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.stop_discovery().await?;
        }
        Ok(())
    }

    /// Check if device discovery is currently active.
    pub async fn discovering(&self) -> zbus::Result<bool> {
        match &self.adapter {
            Some(adapter) => adapter.discovering().await,
            None => Ok(false),
        }
    }

    /// Get all known Bluetooth devices.
    pub async fn devices(&self) -> Result<Vec<BluetoothDevice>> {
        let devices_info = self
            .bluez
            .get_managed_objects()
            .await?
            .into_iter()
            .filter_map(|(key, item)| {
                if item.contains_key("org.bluez.Device1") {
                    Some((key.clone(), item.contains_key("org.bluez.Battery1")))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut devices = Vec::new();
        for (device_path, has_battery) in devices_info {
            let device = DeviceProxy::builder(self.bluez.inner().connection())
                .path(device_path.clone())?
                .build()
                .await?;

            let name = device.alias().await?;
            let connected = device.connected().await?;
            let paired = device.paired().await?;

            let battery = if connected && has_battery {
                let battery_proxy = BatteryProxy::builder(self.bluez.inner().connection())
                    .path(&device_path)?
                    .build()
                    .await?;
                Some(battery_proxy.percentage().await?)
            } else {
                None
            };

            devices.push(BluetoothDevice {
                name,
                battery,
                path: device_path,
                connected,
                paired,
            });
        }

        Ok(devices)
    }

    /// Pair with a device.
    pub async fn pair_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        let device = DeviceProxy::builder(self.bluez.inner().connection())
            .path(device_path)?
            .build()
            .await?;
        device.pair().await
    }

    /// Connect to a device.
    pub async fn connect_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        let device = DeviceProxy::builder(self.bluez.inner().connection())
            .path(device_path)?
            .build()
            .await?;
        device.connect().await
    }

    /// Disconnect from a device.
    pub async fn disconnect_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        let device = DeviceProxy::builder(self.bluez.inner().connection())
            .path(device_path)?
            .build()
            .await?;
        device.disconnect().await
    }

    /// Remove/unpair a device.
    pub async fn remove_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.remove_device(device_path.as_ref()).await?;
        }
        Ok(())
    }
}

// BlueZ ObjectManager proxy for discovering adapters and devices
#[proxy(
    default_service = "org.bluez",
    default_path = "/",
    interface = "org.freedesktop.DBus.ObjectManager"
)]
pub trait BluezObjectManager {
    /// Get all managed objects (adapters, devices, etc.)
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;

    /// Signal emitted when interfaces are added
    #[zbus(signal)]
    fn interfaces_added(
        &self,
        object_path: OwnedObjectPath,
        interfaces: HashMap<String, HashMap<String, OwnedValue>>,
    ) -> zbus::Result<()>;

    /// Signal emitted when interfaces are removed
    #[zbus(signal)]
    fn interfaces_removed(
        &self,
        object_path: OwnedObjectPath,
        interfaces: Vec<String>,
    ) -> zbus::Result<()>;
}

// BlueZ Adapter proxy
#[proxy(
    default_service = "org.bluez",
    default_path = "/org/bluez/hci0",
    interface = "org.bluez.Adapter1"
)]
pub trait Adapter {
    /// Whether the adapter is powered on
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    /// Set the adapter power state
    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;

    /// Start device discovery
    fn start_discovery(&self) -> zbus::Result<()>;

    /// Stop device discovery
    fn stop_discovery(&self) -> zbus::Result<()>;

    /// Whether device discovery is active
    #[zbus(property)]
    fn discovering(&self) -> zbus::Result<bool>;

    /// Remove a device from the adapter
    fn remove_device(&self, device: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;
}

// BlueZ Device proxy
#[proxy(default_service = "org.bluez", interface = "org.bluez.Device1")]
pub trait Device {
    /// Device alias (friendly name)
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;

    /// Whether the device is connected
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;

    /// Whether the device is paired
    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;

    /// Pair with the device
    fn pair(&self) -> zbus::Result<()>;

    /// Connect to the device
    fn connect(&self) -> zbus::Result<()>;

    /// Disconnect from the device
    fn disconnect(&self) -> zbus::Result<()>;
}

// BlueZ Battery proxy (for devices that report battery level)
#[proxy(default_service = "org.bluez", interface = "org.bluez.Battery1")]
pub trait Battery {
    /// Battery percentage (0-100)
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<u8>;
}
