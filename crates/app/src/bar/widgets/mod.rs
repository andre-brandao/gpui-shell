//! Bar widgets for displaying system information.

mod active_window;
mod battery;
mod clock;
mod keyboard_layout;
mod launcher_btn;
mod registry;
pub mod settings;
pub(crate) mod style;
pub mod sysinfo;
mod tray;
mod workspaces;

pub use active_window::ActiveWindow;
pub use battery::Battery;
pub use clock::Clock;
pub use keyboard_layout::KeyboardLayout;
pub use launcher_btn::LauncherBtn;
pub use registry::{Widget, WidgetSlot};
pub use settings::Settings;
pub use sysinfo::SysInfo;
pub use tray::Tray;
pub use workspaces::Workspaces;
