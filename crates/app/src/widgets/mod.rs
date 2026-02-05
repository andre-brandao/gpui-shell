//! Bar widgets for the migrated Zed UI implementation.
//!
//! Phase 1 sets up the module structure and a registry so the bar can
//! render placeholder widgets. Individual widgets will be migrated in
//! later phases.

mod active_window;
mod battery;
mod clock;
mod keyboard_layout;
mod launcher_btn;
mod registry;
mod settings;
mod tray;
mod workspaces;

pub mod sysinfo;

pub use active_window::ActiveWindow;
pub use battery::Battery;
pub use clock::Clock;
pub use keyboard_layout::KeyboardLayout;
pub use launcher_btn::LauncherBtn;
pub use registry::Widget;
pub use settings::Settings;
pub use sysinfo::SysInfo;
pub use tray::Tray;
pub use workspaces::Workspaces;
