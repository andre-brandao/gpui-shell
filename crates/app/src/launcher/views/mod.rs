//! Built-in launcher views. Adding one means a directory here and a line in
//! [`registry`].

pub mod apps;
pub mod calculator;
pub mod help;
pub mod services;
pub mod shell;
pub mod theme;
pub mod wallpaper;
pub mod web;
pub mod workspaces;

use super::config::ViewsConfig;
use super::view::ViewDescriptor;

/// Every view, in help and footer order. Help comes last so it can list the
/// rest.
pub fn registry(config: &ViewsConfig) -> Vec<ViewDescriptor> {
    let mut views = vec![
        apps::descriptor(&config.apps),
        shell::descriptor(&config.shell),
        web::descriptor(&config.web),
        workspaces::descriptor(&config.workspaces),
        wallpaper::descriptor(&config.wallpaper),
        theme::descriptor(&config.themes),
        services::descriptor(&config.services),
        calculator::descriptor(&config.calculator),
    ];
    let listed = views.iter().map(|view| view.meta.clone()).collect();
    views.push(help::descriptor(&config.help, listed));
    views
}
