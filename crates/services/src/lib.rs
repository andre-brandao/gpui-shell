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
pub mod sysinfo;
pub mod themes;
pub mod tray;
pub mod upower;
pub mod wallpaper;
pub mod watcher;

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
pub use watcher::FileWatcher;
