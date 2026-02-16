//! Launcher views module.
//!
//! This module contains all the different views available in the launcher,
//! such as applications search, workspaces, shell commands, web search, and help.

mod apps;
mod help;
mod shell;
mod theme;
mod wallpaper;
mod web;
mod workspaces;

pub use apps::AppsView;
pub use help::HelpView;
pub use shell::ShellView;
pub use theme::ThemeView;
pub use wallpaper::WallpaperView;
pub use web::WebSearchView;
pub use workspaces::WorkspacesView;

use super::view::LauncherView;

/// Create all available views.
///
/// These views will be registered with the launcher and matched by their prefix.
/// The order matters for prefix matching - more specific prefixes should come first.
pub fn all_views() -> Vec<Box<dyn LauncherView>> {
    vec![
        Box::new(AppsView),       // @ prefix (default view)
        Box::new(ShellView),      // $ prefix
        Box::new(WebSearchView),  // ! prefix
        Box::new(WorkspacesView), // ;ws prefix
        Box::new(WallpaperView),  // ;wp prefix
        Box::new(ThemeView),      // ~ prefix
    ]
}
