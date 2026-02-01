use crate::services::Services;
use gpui::App;

/// A launcher view that provides items for a specific category.
pub trait LauncherView: Send + Sync {
    /// The prefix command to activate this view (e.g., "apps", "ws").
    fn prefix(&self) -> &'static str;

    /// Display name for the view.
    fn name(&self) -> &'static str;

    /// Icon for the view (Nerd font).
    fn icon(&self) -> &'static str;

    /// Description shown in help.
    fn description(&self) -> &'static str;

    /// Get items matching the query.
    fn items(&self, query: &str, services: &Services, cx: &App) -> Vec<ViewItem>;

    /// Whether this view is the default when no prefix is given.
    fn is_default(&self) -> bool {
        false
    }
}

/// A single item in a launcher view.
#[derive(Clone)]
pub struct ViewItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: String,
    pub action: ViewAction,
}

/// Action to perform when an item is selected.
#[derive(Clone)]
pub enum ViewAction {
    /// Launch an application by exec command.
    Launch(String),
    /// Focus a workspace by ID.
    FocusWorkspace(i32),
    /// Focus a monitor by ID.
    FocusMonitor(i128),
    /// Toggle WiFi.
    ToggleWifi,
    /// Toggle audio mute.
    ToggleMute,
    /// Adjust volume by delta.
    AdjustVolume(i8),
    /// Switch to a different view prefix.
    SwitchView(String),
    /// No action (for display only).
    None,
}

impl ViewItem {
    pub fn new(id: impl Into<String>, title: impl Into<String>, icon: impl Into<String>) -> Self {
        ViewItem {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            icon: icon.into(),
            action: ViewAction::None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn with_action(mut self, action: ViewAction) -> Self {
        self.action = action;
        self
    }

    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let query_lower = query.to_lowercase();
        if self.title.to_lowercase().contains(&query_lower) {
            return true;
        }
        if let Some(ref subtitle) = self.subtitle {
            if subtitle.to_lowercase().contains(&query_lower) {
                return true;
            }
        }
        false
    }
}

/// Execute a view action.
pub fn execute_action(action: &ViewAction, services: &Services, cx: &mut App) {
    use crate::services::audio::AudioCommand;
    use crate::services::compositor::types::CompositorCommand;
    use crate::services::network::NetworkCommand;

    match action {
        ViewAction::Launch(exec) => {
            let exec = exec.clone();
            std::thread::spawn(move || {
                let exec_cleaned = exec
                    .replace("%f", "")
                    .replace("%F", "")
                    .replace("%u", "")
                    .replace("%U", "")
                    .replace("%d", "")
                    .replace("%D", "")
                    .replace("%n", "")
                    .replace("%N", "")
                    .replace("%i", "")
                    .replace("%c", "")
                    .replace("%k", "");
                let _ = std::process::Command::new("sh")
                    .args(["-c", &exec_cleaned])
                    .spawn();
            });
        }
        ViewAction::FocusWorkspace(id) => {
            services.compositor.update(cx, |compositor, cx| {
                compositor.dispatch(CompositorCommand::FocusWorkspace(*id), cx);
            });
        }
        ViewAction::FocusMonitor(id) => {
            services.compositor.update(cx, |compositor, cx| {
                compositor.dispatch(CompositorCommand::FocusMonitor(*id), cx);
            });
        }
        ViewAction::ToggleWifi => {
            services.network.update(cx, |network, cx| {
                network.dispatch(NetworkCommand::ToggleWiFi, cx);
            });
        }
        ViewAction::ToggleMute => {
            services.audio.update(cx, |audio, cx| {
                audio.dispatch(AudioCommand::ToggleSinkMute, cx);
            });
        }
        ViewAction::AdjustVolume(delta) => {
            services.audio.update(cx, |audio, cx| {
                audio.dispatch(AudioCommand::AdjustSinkVolume(*delta), cx);
            });
        }
        ViewAction::SwitchView(_) | ViewAction::None => {}
    }
}
