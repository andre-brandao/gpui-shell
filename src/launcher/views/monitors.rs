use crate::launcher::view::{LauncherView, ViewAction, ViewItem};
use crate::services::Services;
use gpui::App;

pub struct MonitorsView;

impl LauncherView for MonitorsView {
    fn prefix(&self) -> &'static str {
        "mon"
    }

    fn name(&self) -> &'static str {
        "Monitors"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Focus a monitor"
    }

    fn items(&self, query: &str, services: &Services, cx: &App) -> Vec<ViewItem> {
        let compositor = services.compositor.read(cx);

        compositor
            .monitors
            .iter()
            .filter_map(|mon| {
                let item = ViewItem::new(format!("mon-{}", mon.id), &mon.name, "")
                    .with_subtitle(format!("Workspace {}", mon.active_workspace_id))
                    .with_action(ViewAction::FocusMonitor(mon.id));

                if item.matches(query) {
                    Some(item)
                } else {
                    None
                }
            })
            .collect()
    }
}
