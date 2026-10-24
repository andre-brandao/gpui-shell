use serde::{Deserialize, Serialize};

use super::views::{
    apps::config::AppsConfig, calculator::config::CalculatorConfig, help::config::HelpConfig,
    services::config::ServicesConfig, shell::config::ShellConfig, theme::config::ThemesConfig,
    wallpaper::config::WallpaperConfig, web::config::WebConfig,
    workspaces::config::WorkspacesConfig,
};

/// Launcher window configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {
    pub width: f32,
    pub height: f32,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    /// Per-view settings, `[launcher.views.<id>]`.
    #[serde(alias = "modules")]
    pub views: ViewsConfig,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            width: 600.0,
            height: 450.0,
            margin_top: 100.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            views: ViewsConfig::default(),
        }
    }
}

/// Configuration for launcher views.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewsConfig {
    pub apps: AppsConfig,
    pub shell: ShellConfig,
    pub web: WebConfig,
    pub workspaces: WorkspacesConfig,
    pub wallpaper: WallpaperConfig,
    pub themes: ThemesConfig,
    pub services: ServicesConfig,
    pub calculator: CalculatorConfig,
    pub help: HelpConfig,
}
