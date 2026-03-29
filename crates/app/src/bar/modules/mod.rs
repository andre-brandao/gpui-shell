//! Bar modules for displaying system information.

mod active_window;
mod bar_widget;
mod battery;
mod clock;
mod keyboard_layout;
mod launcher_btn;
mod mpris;
mod registry;
pub mod settings;
pub(crate) mod style;
pub mod sysinfo;
mod tray;
mod workspaces;

pub use active_window::{ActiveWindow, ActiveWindowConfig};
pub(crate) use bar_widget::{BarWidget, BarWidgetShell};
pub use battery::{Battery, BatteryConfig};
pub use clock::{Clock, ClockConfig};
pub use keyboard_layout::{KeyboardLayout, KeyboardLayoutConfig};
pub use launcher_btn::{LauncherBtn, LauncherBtnConfig};
pub use mpris::{Mpris, MprisConfig};
pub use registry::Widget;
pub use settings::{Settings, SettingsConfig};
pub use sysinfo::{SysInfo, SysInfoConfig};
pub use tray::{Tray, TrayConfig};
pub use workspaces::{Workspaces, WorkspacesConfig};
