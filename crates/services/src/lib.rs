//! Services for system integration via D-Bus and other interfaces.
//!
//! This crate provides reactive services for monitoring and controlling
//! system components like battery, power profiles, compositor, audio, network, etc.

pub mod audio;
pub mod brightness;
pub mod compositor;
pub mod privacy;
pub mod upower;

pub use audio::{AudioCommand, AudioData, AudioSubscriber};
pub use brightness::{BrightnessCommand, BrightnessData, BrightnessSubscriber};
pub use compositor::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, CompositorSubscriber,
    Monitor, Workspace,
};
pub use privacy::{ApplicationNode, Media, PrivacyData, PrivacySubscriber};
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
    pub audio: AudioSubscriber,
    pub brightness: BrightnessSubscriber,
    pub compositor: CompositorSubscriber,
    pub privacy: PrivacySubscriber,
    pub upower: UPowerSubscriber,
    // Future services:
    // pub network: NetworkSubscriber,
    // pub bluetooth: BluetoothSubscriber,
}

impl Services {
    /// Create and initialize all services.
    ///
    /// This should be called once during application startup.
    /// Services will begin monitoring system state immediately.
    pub async fn new() -> anyhow::Result<Self> {
        let audio = AudioSubscriber::new();
        let brightness = BrightnessSubscriber::new().await?;
        let compositor = CompositorSubscriber::new().await?;
        let privacy = PrivacySubscriber::new();
        let upower = UPowerSubscriber::new().await?;

        Ok(Self {
            audio,
            brightness,
            compositor,
            privacy,
            upower,
        })
    }
}
