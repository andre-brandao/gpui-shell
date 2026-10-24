//! Applications view for searching and launching desktop applications.

pub mod config;

use gpui::{Context, EventEmitter};
use ui::IconName;

use self::config::AppsConfig;
use crate::launcher::item::{Item, ItemIcon};
use crate::launcher::view::{Items, LauncherView, Outcome, ViewDescriptor, ViewEvent, ViewMeta};
use crate::state::AppState;

const ICON: IconName = IconName::Layers;

pub fn descriptor(config: &AppsConfig) -> ViewDescriptor {
    ViewDescriptor::new(
        ViewMeta {
            id: "apps",
            prefix: config.prefix.clone().into(),
            name: "Applications",
            icon: ICON,
            description: "Search and launch applications",
            default: true,
            in_help: true,
        },
        |_| AppsView,
    )
}

pub struct AppsView;

impl EventEmitter<ViewEvent> for AppsView {}

impl LauncherView for AppsView {
    fn items(&mut self, _query: &str, cx: &mut Context<Self>) -> Items {
        let apps = AppState::applications(cx).all();
        Items::Ready(
            apps.iter()
                .enumerate()
                .map(|(ix, app)| {
                    let icon = match &app.icon_path {
                        Some(path) => ItemIcon::Path(path.clone()),
                        None => ItemIcon::Icon(ICON.into()),
                    };
                    let item = Item::new(ix.to_string(), app.name.clone())
                        .icon(icon)
                        .action("launch", "Open");
                    match &app.description {
                        Some(description) => item.subtitle(description.clone()),
                        None => item,
                    }
                })
                .collect(),
        )
    }

    fn activate(&mut self, item: &Item, _action: &str, cx: &mut Context<Self>) -> Outcome {
        let apps = AppState::applications(cx).all();
        if let Some(app) = item.id.parse::<usize>().ok().and_then(|ix| apps.get(ix)) {
            app.launch();
        }
        Outcome::Close
    }
}
