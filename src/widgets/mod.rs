mod battery;
mod clock;
mod info;
mod launcher_btn;
mod registry;
pub mod sysinfo;
mod systray;
mod workspaces;

pub use battery::Battery;
pub use clock::Clock;
pub use info::Info;
pub use launcher_btn::LauncherBtn;
pub use registry::Widget;
pub use sysinfo::SysInfoWidget;
pub use systray::Systray;
pub use workspaces::Workspaces;
