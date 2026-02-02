use crate::launcher::view::{
    LauncherView, ViewAction, ViewContext, execute_action, render_list_item,
};
use gpui::{AnyElement, App, div, prelude::*, px};

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

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let apps = vx.services.applications.read(cx);
        let query_lower = vx.query.to_lowercase();

        let filtered: Vec<_> = apps
            .apps
            .iter()
            .filter(|app| {
                if vx.query.is_empty() {
                    return true;
                }
                if app.name.to_lowercase().contains(&query_lower) {
                    return true;
                }
                if let Some(ref desc) = app.description {
                    if desc.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                false
            })
            .collect();

        let count = filtered.len();
        let services = vx.services.clone();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(filtered.into_iter().enumerate().map(|(i, app)| {
                let exec = app.exec.clone();
                let services_clone = services.clone();
                render_list_item(
                    format!("app-{}", app.name),
                    "",
                    &app.name,
                    app.description.as_deref(),
                    i == vx.selected_index,
                    move |cx| {
                        execute_action(&ViewAction::Launch(exec.clone()), &services_clone, cx);
                    },
                )
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let apps = vx.services.applications.read(cx);
        let query_lower = vx.query.to_lowercase();

        let filtered: Vec<_> = apps
            .apps
            .iter()
            .filter(|app| {
                if vx.query.is_empty() {
                    return true;
                }
                if app.name.to_lowercase().contains(&query_lower) {
                    return true;
                }
                if let Some(ref desc) = app.description {
                    if desc.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                false
            })
            .collect();

        if let Some(app) = filtered.get(index) {
            execute_action(&ViewAction::Launch(app.exec.clone()), vx.services, cx);
            true // Close launcher
        } else {
            false
        }
    }
}
