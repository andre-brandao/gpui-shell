use crate::launcher::view::{LauncherView, ViewContext, ViewObserver, render_list_item};
use crate::services::Services;
use gpui::{AnyElement, App, Context, div, prelude::*, px};

pub struct AppsView;

impl AppsView {
    /// Launch an application by exec command.
    fn launch(exec: &str) {
        let exec = exec.to_string();
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
}

impl<T: 'static> ViewObserver<T> for AppsView {
    fn observe_services(services: &Services, cx: &mut Context<T>) {
        // AppsView only needs to observe the applications service
        cx.observe(&services.applications, |_, _, cx| cx.notify())
            .detach();
    }
}

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

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(filtered.into_iter().enumerate().map(|(i, app)| {
                let exec = app.exec.clone();
                render_list_item(
                    format!("app-{}", app.name),
                    "",
                    &app.name,
                    app.description.as_deref(),
                    i == vx.selected_index,
                    move |_cx| {
                        Self::launch(&exec);
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
            Self::launch(&app.exec);
            true // Close launcher
        } else {
            false
        }
    }
}
