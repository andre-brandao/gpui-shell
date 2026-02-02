//! Services for system integration via D-Bus and other interfaces.
//!
//! This crate provides reactive services for monitoring and controlling
//! system components like battery, power profiles, compositor, audio, network, etc.

pub mod compositor;
pub mod upower;

pub use compositor::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, CompositorSubscriber,
    Monitor, Workspace,
};
pub use upower::{
    BatteryData, BatteryLevel, BatteryState, PowerProfile, UPowerCommand, UPowerData,
    UPowerSubscriber, WarningLevel,
};

/// Shared services container for all system integrations.
///
/// This struct holds instances of all available services and should be
/// initialized once at application startup, then shared with widgets
/// that need access to system information.
#[derive(Clone)]
pub struct Services {
    pub upower: UPowerSubscriber,
    pub compositor: CompositorSubscriber,
    // Future services:
    // pub audio: AudioSubscriber,
    // pub network: NetworkSubscriber,
    // pub bluetooth: BluetoothSubscriber,
}

impl Services {
    /// Create and initialize all services.
    ///
    /// This should be called once during application startup.
    /// Services will begin monitoring system state immediately.
    pub async fn new() -> anyhow::Result<Self> {
        let upower = UPowerSubscriber::new().await?;
        let compositor = CompositorSubscriber::new().await?;

        Ok(Self { upower, compositor })
    }
}
