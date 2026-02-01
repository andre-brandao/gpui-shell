use crate::launcher::view::{LauncherView, ViewAction, ViewItem};
use crate::services::Services;
use gpui::App;

pub struct AppsView;

impl LauncherView for AppsView {
    fn prefix(&self) -> &'static str {
        "apps"
    }

    fn name(&self) -> &'static str {
        "Applications"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Search and launch applications"
    }

    fn is_default(&self) -> bool {
        true
    }

    fn items(&self, query: &str, services: &Services, cx: &App) -> Vec<ViewItem> {
        let apps = services.applications.read(cx);

        apps.apps
            .iter()
            .filter_map(|app| {
                let item = ViewItem::new(&app.name, &app.name, "")
                    .with_action(ViewAction::Launch(app.exec.clone()));

                let item = if let Some(ref desc) = app.description {
                    item.with_subtitle(desc)
                } else {
                    item
                };

                if item.matches(query) {
                    Some(item)
                } else {
                    None
                }
            })
            .collect()
    }
}
