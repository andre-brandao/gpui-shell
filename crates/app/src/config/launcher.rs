use serde::{Deserialize, Serialize};

pub use crate::launcher::modules::apps::config::AppsConfig;
pub use crate::launcher::modules::help::config::HelpConfig;
pub use crate::launcher::modules::shell::config::ShellConfig;
pub use crate::launcher::modules::theme::config::ThemesConfig;
pub use crate::launcher::modules::wallpaper::config::WallpaperConfig;
pub use crate::launcher::modules::web::config::{WebConfig, WebProviderConfig};
pub use crate::launcher::modules::workspaces::config::WorkspacesConfig;

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
    pub modules: ModulesConfig,
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
            modules: ModulesConfig::default(),
        }
    }
}

/// Configuration for launcher modules/views.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModulesConfig {
    pub apps: AppsConfig,
    pub shell: ShellConfig,
    pub web: WebConfig,
    pub workspaces: WorkspacesConfig,
    pub wallpaper: WallpaperConfig,
    pub themes: ThemesConfig,
    pub help: HelpConfig,
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            apps: AppsConfig::default(),
            shell: ShellConfig::default(),
            web: WebConfig::default(),
            workspaces: WorkspacesConfig::default(),
            wallpaper: WallpaperConfig::default(),
            themes: ThemesConfig::default(),
            help: HelpConfig::default(),
        }
    }
}
