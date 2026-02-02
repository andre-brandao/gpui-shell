//! Bar widgets for displaying system information.

mod battery;
mod clock;
mod keyboard_layout;
mod registry;
mod sysinfo;
mod tray;
mod workspaces;

pub use battery::Battery;
pub use clock::Clock;
pub use keyboard_layout::KeyboardLayout;
pub use registry::Widget;
pub use sysinfo::SysInfo;
pub use tray::Tray;
pub use workspaces::Workspaces;
