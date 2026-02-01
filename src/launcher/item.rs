use crate::services::Services;
use crate::services::applications::Application;
use crate::services::audio::AudioCommand;
use crate::services::compositor::types::{
    CompositorCommand, CompositorMonitor, CompositorWorkspace,
};
use crate::services::network::NetworkCommand;
use gpui::App;

/// Categories for launcher items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Apps,
    Windows,
    Workspaces,
    Monitors,
    System,
}

impl Category {
    pub fn icon(&self) -> &'static str {
        match self {
            Category::Apps => "",
            Category::Windows => "",
            Category::Workspaces => "",
            Category::Monitors => "",
            Category::System => "",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Category::Apps => "Apps",
            Category::Windows => "Windows",
            Category::Workspaces => "Workspaces",
            Category::Monitors => "Monitors",
            Category::System => "System",
        }
    }

    pub fn prefix(&self) -> Option<char> {
        match self {
            Category::Apps => None,
            Category::Windows => Some('@'),
            Category::Workspaces => Some('#'),
            Category::Monitors => Some('!'),
            Category::System => Some('>'),
        }
    }
}

/// System actions available in the launcher.
#[derive(Debug, Clone)]
pub enum SystemAction {
    ToggleWifi,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    // Add more as needed
}

impl SystemAction {
    pub fn all() -> Vec<SystemAction> {
        vec![
            SystemAction::ToggleWifi,
            SystemAction::ToggleMute,
            SystemAction::VolumeUp,
            SystemAction::VolumeDown,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            SystemAction::ToggleWifi => "Toggle WiFi",
            SystemAction::ToggleMute => "Toggle Mute",
            SystemAction::VolumeUp => "Volume Up",
            SystemAction::VolumeDown => "Volume Down",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SystemAction::ToggleWifi => "󰤨",
            SystemAction::ToggleMute => "󰝟",
            SystemAction::VolumeUp => "󰕾",
            SystemAction::VolumeDown => "󰖀",
        }
    }

    pub fn keywords(&self) -> &[&'static str] {
        match self {
            SystemAction::ToggleWifi => &["wifi", "wireless", "network", "internet"],
            SystemAction::ToggleMute => &["mute", "sound", "audio", "silent"],
            SystemAction::VolumeUp => &["volume", "louder", "sound", "audio"],
            SystemAction::VolumeDown => &["volume", "quieter", "sound", "audio"],
        }
    }
}

/// Unified launcher item that can represent any searchable/actionable item.
#[derive(Debug, Clone)]
pub enum LauncherItem {
    App(Application),
    Workspace(CompositorWorkspace),
    Monitor(CompositorMonitor),
    System(SystemAction),
}

impl LauncherItem {
    pub fn id(&self) -> String {
        match self {
            LauncherItem::App(app) => format!("app:{}", app.name),
            LauncherItem::Workspace(ws) => format!("workspace:{}", ws.id),
            LauncherItem::Monitor(mon) => format!("monitor:{}", mon.id),
            LauncherItem::System(action) => format!("system:{}", action.title()),
        }
    }

    pub fn title(&self) -> String {
        match self {
            LauncherItem::App(app) => app.name.clone(),
            LauncherItem::Workspace(ws) => {
                if ws.name.is_empty() {
                    format!("Workspace {}", ws.id)
                } else {
                    ws.name.clone()
                }
            }
            LauncherItem::Monitor(mon) => mon.name.clone(),
            LauncherItem::System(action) => action.title().to_string(),
        }
    }

    pub fn subtitle(&self) -> Option<String> {
        match self {
            LauncherItem::App(app) => app.description.clone(),
            LauncherItem::Workspace(ws) => Some(format!("{} windows", ws.windows)),
            LauncherItem::Monitor(_) => None,
            LauncherItem::System(_) => None,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LauncherItem::App(_) => "",
            LauncherItem::Workspace(_) => "",
            LauncherItem::Monitor(_) => "",
            LauncherItem::System(action) => action.icon(),
        }
    }

    pub fn category(&self) -> Category {
        match self {
            LauncherItem::App(_) => Category::Apps,
            LauncherItem::Workspace(_) => Category::Workspaces,
            LauncherItem::Monitor(_) => Category::Monitors,
            LauncherItem::System(_) => Category::System,
        }
    }

    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        let query_lower = query.to_lowercase();
        let title_lower = self.title().to_lowercase();

        if title_lower.contains(&query_lower) {
            return true;
        }

        if let Some(subtitle) = self.subtitle() {
            if subtitle.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        // Check keywords for system actions
        if let LauncherItem::System(action) = self {
            for keyword in action.keywords() {
                if keyword.contains(&query_lower) {
                    return true;
                }
            }
        }

        false
    }

    pub fn execute(&self, services: &Services, cx: &mut App) {
        match self {
            LauncherItem::App(app) => {
                app.launch();
            }
            LauncherItem::Workspace(ws) => {
                services.compositor.update(cx, |compositor, cx| {
                    compositor.dispatch(CompositorCommand::FocusWorkspace(ws.id), cx);
                });
            }
            LauncherItem::Monitor(mon) => {
                services.compositor.update(cx, |compositor, cx| {
                    compositor.dispatch(CompositorCommand::FocusMonitor(mon.id), cx);
                });
            }
            LauncherItem::System(action) => match action {
                SystemAction::ToggleWifi => {
                    services.network.update(cx, |network, cx| {
                        network.dispatch(NetworkCommand::ToggleWiFi, cx);
                    });
                }
                SystemAction::ToggleMute => {
                    services.audio.update(cx, |audio, cx| {
                        audio.dispatch(AudioCommand::ToggleSinkMute, cx);
                    });
                }
                SystemAction::VolumeUp => {
                    services.audio.update(cx, |audio, cx| {
                        audio.dispatch(AudioCommand::AdjustSinkVolume(5), cx);
                    });
                }
                SystemAction::VolumeDown => {
                    services.audio.update(cx, |audio, cx| {
                        audio.dispatch(AudioCommand::AdjustSinkVolume(-5), cx);
                    });
                }
            },
        }
    }
}
