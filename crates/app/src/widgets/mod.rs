//! Bar widgets for displaying system information.

mod battery;
mod clock;
mod keyboard_layout;
mod registry;
mod tray;
mod workspaces;

pub use battery::Battery;
pub use clock::Clock;
pub use keyboard_layout::KeyboardLayout;
pub use registry::Widget;
pub use tray::Tray;
pub use workspaces::Workspaces;
