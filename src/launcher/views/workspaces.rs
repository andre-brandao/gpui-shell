use crate::launcher::view::{LauncherView, ViewAction, ViewItem};
use crate::services::Services;
use gpui::App;

pub struct WorkspacesView;

impl LauncherView for WorkspacesView {
    fn prefix(&self) -> &'static str {
        "ws"
    }

    fn name(&self) -> &'static str {
        "Workspaces"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Switch between workspaces"
    }

    fn items(&self, query: &str, services: &Services, cx: &App) -> Vec<ViewItem> {
        let compositor = services.compositor.read(cx);

        compositor
            .workspaces
            .iter()
            .filter(|ws| !ws.is_special)
            .filter_map(|ws| {
                let title = if ws.name.is_empty() {
                    format!("Workspace {}", ws.id)
                } else {
                    ws.name.clone()
                };

                let subtitle = format!("{} windows on {}", ws.windows, ws.monitor);

                let item = ViewItem::new(format!("ws-{}", ws.id), title, "")
                    .with_subtitle(subtitle)
                    .with_action(ViewAction::FocusWorkspace(ws.id));

                if item.matches(query) {
                    Some(item)
                } else {
                    None
                }
            })
            .collect()
    }
}
