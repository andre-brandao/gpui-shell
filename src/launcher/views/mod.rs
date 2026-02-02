mod apps;
mod control;
mod help;
mod monitors;
mod system;
mod workspaces;

pub use apps::AppsView;
pub use control::ControlView;
pub use help::HelpView;
pub use monitors::MonitorsView;
pub use system::SystemView;
pub use workspaces::WorkspacesView;

use super::view::LauncherView;

/// Create all available views.
pub fn all_views() -> Vec<Box<dyn LauncherView>> {
    vec![
        Box::new(AppsView),
        Box::new(WorkspacesView),
        Box::new(MonitorsView),
        Box::new(SystemView),
        Box::new(ControlView),
    ]
}
