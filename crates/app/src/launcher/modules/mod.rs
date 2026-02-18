//! Launcher modules.
//!
//! This module contains all the different views available in the launcher,
//! such as applications search, workspaces, shell commands, web search, and help.
//! Each view is in its own folder for better organization.

pub mod apps;
pub mod help;
pub mod shell;
pub mod theme;
pub mod wallpaper;
pub mod web;
pub mod workspaces;

pub use apps::AppsView;
pub use help::HelpView;
pub use shell::ShellView;
pub use theme::ThemeView;
pub use wallpaper::WallpaperView;
pub use web::WebSearchView;
pub use workspaces::WorkspacesView;

use super::config::LauncherConfig;
use super::view::LauncherView;

pub fn all_views(config: &LauncherConfig) -> Vec<Box<dyn LauncherView>> {
    vec![
        Box::new(AppsView::new(&config.modules.apps)),
        Box::new(ShellView::new(&config.modules.shell)),
        Box::new(WebSearchView::new(&config.modules.web)),
        Box::new(WorkspacesView::new(&config.modules.workspaces)),
        Box::new(WallpaperView::new(&config.modules.wallpaper)),
        Box::new(ThemeView::new(&config.modules.themes)),
    ]
}
