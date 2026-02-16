//! Services for system integration via D-Bus and other interfaces.
//!
//! This crate provides reactive services for monitoring and controlling
//! system components like battery, power profiles, compositor, audio, network, etc.

pub mod applications;
pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod compositor;
pub mod mpris;
pub mod network;
pub mod notification;
pub mod privacy;
pub mod shell;
pub mod sysinfo;
pub mod themes;
pub mod tray;
pub mod upower;
pub mod wallpaper;

pub use applications::{Application, ApplicationsService};
pub use audio::{AudioCommand, AudioData, AudioSubscriber};
pub use bluetooth::{
    BluetoothCommand, BluetoothData, BluetoothDevice, BluetoothState, BluetoothSubscriber,
};
pub use brightness::{BrightnessCommand, BrightnessData, BrightnessSubscriber};
pub use compositor::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, CompositorSubscriber,
    Monitor, Workspace,
};
pub use mpris::{
    MprisCommand, MprisData, MprisPlayerData, MprisPlayerMetadata, MprisSubscriber, PlaybackStatus,
    PlayerCommand,
};
pub use network::{
    AccessPoint, ActiveConnectionInfo, ConnectivityState, DeviceState, DeviceType, NetworkCommand,
    NetworkData, NetworkStatistics, NetworkSubscriber,
};
pub use notification::{
    Notification, NotificationCommand, NotificationData, NotificationSubscriber,
};
pub use privacy::{ApplicationNode, Media, PrivacyData, PrivacySubscriber};
pub use shell::{InstanceResult, LauncherRequest, ShellSubscriber};
pub use sysinfo::{DiskInfo, NetworkInfo, SysInfoData, SysInfoSubscriber};
pub use themes::{
    Base16Palette, Base16Scheme, PROVIDERS as THEME_PROVIDERS, ThemeProvider, ThemeRepository,
    load_stylix_scheme,
};
pub use tray::{
    MenuLayout, MenuLayoutProps, TrayCommand, TrayData, TrayIcon, TrayItem, TraySubscriber,
};
pub use upower::{
    BatteryData, BatteryLevel, BatteryState, PowerProfile, UPowerCommand, UPowerData,
    UPowerSubscriber, WarningLevel,
};
pub use wallpaper::{WallpaperCommand, WallpaperData, WallpaperEngine, WallpaperSubscriber};

/// Shared services container for all system integrations.
///
/// This struct holds instances of all available services and should be
/// initialized once at application startup, then shared with widgets
/// that need access to system information.
#[derive(Clone)]
pub struct Services {
    pub applications: ApplicationsService,
    pub audio: AudioSubscriber,
    pub bluetooth: BluetoothSubscriber,
    pub brightness: BrightnessSubscriber,
    pub compositor: CompositorSubscriber,
    pub mpris: MprisSubscriber,
    pub network: NetworkSubscriber,
    pub notification: NotificationSubscriber,
    pub privacy: PrivacySubscriber,
    pub sysinfo: SysInfoSubscriber,
    pub tray: TraySubscriber,
    pub upower: UPowerSubscriber,
    pub wallpaper: WallpaperSubscriber,
}

impl Services {
    /// Create and initialize all services.
    ///
    /// This should be called once during application startup.
    /// Services will begin monitoring system state immediately.
    pub async fn new() -> anyhow::Result<Self> {
        let applications = ApplicationsService::new();
        let audio = AudioSubscriber::new();
        let bluetooth = BluetoothSubscriber::new().await?;
        let brightness = BrightnessSubscriber::new().await?;
        let compositor = CompositorSubscriber::new().await?;
        let mpris = MprisSubscriber::new().await?;
        let network = NetworkSubscriber::new().await?;
        let notification = NotificationSubscriber::new().await.unwrap_or_else(|err| {
            tracing::warn!("Notification service unavailable: {}", err);
            NotificationSubscriber::disabled()
        });
        let privacy = PrivacySubscriber::new();
        let sysinfo = SysInfoSubscriber::new();
        let tray = TraySubscriber::new().await?;
        let upower = UPowerSubscriber::new().await?;
        let wallpaper = WallpaperSubscriber::new();

        Ok(Self {
            applications,
            audio,
            bluetooth,
            brightness,
            compositor,
            mpris,
            network,
            notification,
            privacy,
            sysinfo,
            tray,
            upower,
            wallpaper,
        })
    }
}
