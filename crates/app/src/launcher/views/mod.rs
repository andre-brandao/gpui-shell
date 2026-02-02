//! Launcher views module.
//!
//! This module contains all the different views available in the launcher,
//! such as applications search, workspaces, and help.

mod apps;
mod help;
mod workspaces;

pub use apps::AppsView;
pub use help::HelpView;
pub use workspaces::WorkspacesView;

use super::view::LauncherView;

/// Create all available views.
pub fn all_views() -> Vec<Box<dyn LauncherView>> {
    vec![Box::new(AppsView), Box::new(WorkspacesView)]
}
