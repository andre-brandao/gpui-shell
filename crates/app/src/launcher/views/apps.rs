//! Applications view for searching and launching desktop applications.

use crate::launcher::view::{LauncherView, ViewContext, render_list_item};
use gpui::{AnyElement, App, div, prelude::*, px};

/// Applications view - searches and launches desktop applications.
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

    fn render(&self, vx: &ViewContext, _cx: &App) -> (AnyElement, usize) {
        let query_lower = vx.query.to_lowercase();
        let apps = vx.services.applications.search(&query_lower);
        let count = apps.len();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(apps.into_iter().enumerate().map(|(i, app)| {
                let exec = app.exec.clone();
                render_list_item(
                    format!("app-{}", app.name),
                    "",
                    &app.name,
                    app.description.as_deref(),
                    i == vx.selected_index,
                    move |_cx| {
                        launch_app(&exec);
                    },
                )
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let query_lower = vx.query.to_lowercase();
        let apps = vx.services.applications.search(&query_lower);

        if let Some(app) = apps.get(index) {
            launch_app(&app.exec);
            true // Close launcher
        } else {
            false
        }
    }
}

/// Launch an application by exec command.
fn launch_app(exec: &str) {
    let exec = exec.to_string();
    std::thread::spawn(move || {
        // Remove field codes like %f, %F, %u, %U, etc.
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
